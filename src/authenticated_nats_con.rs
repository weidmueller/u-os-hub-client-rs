//! Handles authentication and connection to NATS server.
//! Used by both provider and consumer modules.

use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use tokio::sync::broadcast;
use tracing::{debug, error};

use crate::oauth2::OAuth2Credentials;

type Result<T> = core::result::Result<T, async_nats::Error>;

/// Access permissions for the data hub.
/// Internally gets converted to Oauth2 scopes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Permissions {
    Read,
    ReadWrite,
    Provide,
}

impl Display for Permissions {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Permissions::Read => write!(f, "hub.variables.readonly"),
            Permissions::ReadWrite => write!(f, "hub.variables.readwrite"),
            Permissions::Provide => write!(f, "hub.variables.provide"),
        }
    }
}

pub type PermissionsList = HashSet<Permissions>;

/// Determines how the connection authenticates to the NATS server.
#[derive(Clone, Debug)]
pub struct AuthenticationSettings {
    permissions: PermissionsList,
    oauth2_endpoint: String,
    creds: Option<OAuth2Credentials>,
}

/// Helper struct to build the authentication settings.
///
/// This is used to create the authentication settings in a more readable way.
pub struct AuthenticationSettingsBuilder {
    settings: AuthenticationSettings,
}

impl AuthenticationSettingsBuilder {
    pub fn new(permission: Permissions) -> Self {
        Self {
            settings: AuthenticationSettings {
                permissions: PermissionsList::from([permission]),
                oauth2_endpoint: "https://127.0.0.1/oauth2/token".to_string(),
                creds: None,
            },
        }
    }

    /// Allows to add multiple permissions at once.
    ///
    /// This is useful if the connection should be shared between e.g. a provider and a consumer.
    pub fn add_permission(mut self, permission: Permissions) -> Self {
        self.settings.permissions.insert(permission);
        self
    }

    /// Allows to specificy oauth2 credentials.
    ///
    /// This is always needed for providers. For consumers,
    /// this can be left out if unauthenticated access is enabled on the device.
    /// In this case, the client name will be "_UNAUTHENTICATED".
    pub fn with_credentials(mut self, creds: OAuth2Credentials) -> Self {
        self.settings.creds = Some(creds);
        self
    }

    /// Allows to specifiy a different oauth2 endpoint address.
    ///
    /// Useful e.g. if the oauth endpoint is on another device.
    /// If not specified, uses the default localhost endpoint that comes with uOS.
    pub fn with_custom_oauth2_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.settings.oauth2_endpoint = endpoint.into();
        self
    }

    /// Builds the authentication settings.
    pub fn build(self) -> AuthenticationSettings {
        self.settings
    }
}

/// Abstracts the nats connection and handles authentication and reconnection.
///
/// Can be used by provider and consumer modules alike.
/// Multiple users can reuse the same nats connection by using a shared instance (Arc).
#[derive(Debug)]
pub struct AuthenticatedNatsConnection {
    nats_client: async_nats::Client,
    event_sender: broadcast::Sender<async_nats::Event>,
    nats_permissions: PermissionsList,
    client_name: String,
}

impl AuthenticatedNatsConnection {
    /// Tries to connect and authenticate to the NATS server.
    ///
    /// If no client_name is suppied in the settings, "_UNAUTHENTICATED" is used.
    ///
    /// The constructor will wait until the first connection event is received by the nats client.
    /// No internal timeout is used. If you want, you can use tokio::timeout to limit the time for connection.
    pub async fn new(
        nats_server_addr: impl Into<String>,
        auth_settings: &AuthenticationSettings,
    ) -> Result<Self> {
        let (event_sender, _) = broadcast::channel(128);

        let nats_permissions = auth_settings.permissions.clone();
        let client_name = auth_settings.creds.as_ref().map_or_else(
            || "_UNAUTHENTICATED".to_string(),
            |creds| creds.client_name.clone(),
        );

        //must subscribe to nats events before trying to connect, as otherwise we may miss the connect event
        let event_receiver = event_sender.subscribe();

        let nats_client = Self::connect_to_nats(
            auth_settings,
            nats_server_addr.into(),
            &client_name,
            event_sender.clone(),
        )
        .await?;

        let instance = Self {
            nats_client: nats_client.clone(),
            event_sender,
            nats_permissions,
            client_name,
        };

        Self::wait_for_connection(event_receiver).await;

        Ok(instance)
    }

    /// Waits for the first NATS connection event to be received.
    ///
    /// Usually you will want to wait for this event to be received before using the connection for other operations.
    async fn wait_for_connection(mut event_receiver: broadcast::Receiver<async_nats::Event>) {
        //wait for first connection event
        while let Ok(event) = event_receiver.recv().await {
            if let async_nats::Event::Connected = event {
                //Connection established!
                break;
            }
        }
    }

    /// Returns a set of permissions that were requested by the client.
    pub fn get_permissions(&self) -> &PermissionsList {
        &self.nats_permissions
    }

    /// Gets the nats client name that was supplied in the auth settings.
    /// If no client name was supplied, "_UNAUTHENTICATED" is used.
    pub fn get_client_name(&self) -> &str {
        &self.client_name
    }

    /// Returns the nats client.
    pub fn get_client(&self) -> &async_nats::Client {
        &self.nats_client
    }

    /// Allows to subscribe to nats events and react to them.
    /// This simply forwards nats events to the caller.
    pub fn get_events(&self) -> broadcast::Receiver<async_nats::Event> {
        self.event_sender.subscribe()
    }

    fn setup_nats_auth_callback(
        auth_settings: &AuthenticationSettings,
    ) -> async_nats::ConnectOptions {
        let token_endpoint = auth_settings.oauth2_endpoint.clone();

        let scope_list = auth_settings
            .permissions
            .iter()
            .map(|perm| perm.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        if let Some(creds) = auth_settings.creds.clone() {
            if creds.client_id.is_empty() {
                return async_nats::ConnectOptions::new();
            }

            async_nats::ConnectOptions::with_auth_callback(move |_| {
                debug!("Requesting token for client id: {}", creds.client_id);

                let creds = creds.clone();
                let token_endpoint = token_endpoint.clone();
                let scope_list = scope_list.clone();

                async move {
                    let result = creds.request_token(&token_endpoint, &scope_list).await;

                    match result {
                        Ok(token_response) => {
                            let mut auth = async_nats::Auth::new();
                            auth.token = Some(token_response.access_token);
                            Ok(auth)
                        }
                        Err(e) => {
                            error!("Error requesting token: {:?}", e);
                            Err(async_nats::AuthError::new(
                                "Error requesting token".to_string(),
                            ))
                        }
                    }
                }
            })
            .retry_on_initial_connect()
        } else {
            async_nats::ConnectOptions::new()
        }
    }

    async fn connect_to_nats(
        auth_settings: &AuthenticationSettings,
        nats_hostname: String,
        client_name: &str,
        event_sender: broadcast::Sender<async_nats::Event>,
    ) -> Result<async_nats::Client> {
        let connection_options = Self::setup_nats_auth_callback(auth_settings);

        let connection_options = connection_options
            .name(client_name)
            .custom_inbox_prefix(format!("_INBOX.{}", client_name));

        let connection_options = connection_options
            .event_callback(move |event| {
                let event_sender = event_sender.clone();
                async move {
                    event_sender.send(event).ok();
                }
            })
            .reconnect_delay_callback(|attempts| {
                // The first attempt should be immediate, then we increase the delay.
                // The delay is increased so that not so many tokens are fetched.
                let duration_sec = match attempts {
                    1 => 0,
                    2..=10 => 5,
                    11..=20 => 30,
                    _ => 300,
                };

                debug!(
                    "Reconnect in {}s, current attempt: {attempts}",
                    duration_sec,
                );
                std::time::Duration::from_secs(duration_sec)
            });

        let client = connection_options.connect(nats_hostname).await?;

        Ok(client)
    }
}
