//!Utility functions and shared code for the examples

use std::path::PathBuf;

use clap::Parser;
use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticationSettings, AuthenticationSettingsBuilder, NatsPermission,
    },
    env_file_parser::read_and_parse_env_file,
    oauth2::OAuth2Credentials,
};

#[derive(Parser, Debug)]
pub struct Config {
    #[clap(long, default_value = "127.0.0.1")]
    pub nats_ip: String,
    #[clap(long, default_value_t = 49360)]
    pub nats_port: u16,
    /// Name of the NATS/DataHub participant
    #[clap(long)]
    pub client_name: String,
    /// Path to the credentials file
    #[clap(long)]
    pub cred_file: PathBuf,
    /// Optional OAuth2 token endpoint address.
    /// If not provided, the default endpoint will be used.
    #[clap(long)]
    pub oauth_token_endpoint: Option<String>,
    /// The provider ID to connect to.
    /// Mandadory for consumers, ignored by providers.
    #[clap(long)]
    pub provider_id: Option<String>,
}

pub async fn build_auth_settings_from_conf(
    conf: &Config,
    is_provider: bool,
) -> anyhow::Result<AuthenticationSettings> {
    println!("{conf:#?}");

    let mut builder = AuthenticationSettingsBuilder::new(if is_provider {
        NatsPermission::VariableHubProvide
    } else {
        NatsPermission::VariableHubReadWrite
    });

    let env_vars = read_and_parse_env_file(&conf.cred_file).await?;

    builder = builder.with_credentials(OAuth2Credentials {
        client_name: conf.client_name.clone(),
        client_id: env_vars
            .get("CLIENT_ID")
            .ok_or(anyhow::anyhow!("Can't get CLIENT_ID"))?
            .clone(),
        client_secret: env_vars
            .get("CLIENT_SECRET")
            .ok_or(anyhow::anyhow!("Can't get CLIENT_SECRET"))?
            .clone(),
    });

    //Add the token endpoint if it is provided, otherwise use default
    builder = if let Some(token_endpoint) = &conf.oauth_token_endpoint {
        builder.with_custom_oauth2_endpoint(token_endpoint.clone())
    } else {
        builder.with_custom_oauth2_endpoint(format!("https://{}/oauth2/token", conf.nats_ip))
    };

    let auth_settings = builder.build();
    Ok(auth_settings)
}
