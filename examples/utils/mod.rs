//!Utility functions and shared code for the examples

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticationSettings, AuthenticationSettingsBuilder, NatsPermission,
    },
    oauth2::OAuth2Credentials,
};

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    pub nats_ip: String,
    pub nats_port: u16,
    pub client_name: String,
    pub cred_file: PathBuf,
    pub oauth_token_endpoint: Option<String>,
}

impl Config {
    pub fn from_file(file_path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(file_path)?;
        let reader = std::io::BufReader::new(file);
        let mut instance: Self = serde_json::from_reader(reader)?;

        if instance.oauth_token_endpoint.is_none() {
            instance.oauth_token_endpoint =
                Some(format!("https://{}/oauth2/token", instance.nats_ip));
        }

        Ok(instance)
    }
}

/// Read an env file and parse it into a HashMap
fn read_and_parse_env_file(path: &PathBuf) -> anyhow::Result<HashMap<String, String>> {
    let file_content = fs::read_to_string(path)?;

    let mut map = HashMap::new();
    for line in file_content.lines() {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if let [key, value] = parts.as_slice() {
            map.insert((*key).to_string(), (*value).trim_matches('"').to_string());
        }
    }

    Ok(map)
}

pub fn build_auth_settings_from_conf(
    conf: &Config,
    is_provider: bool,
) -> anyhow::Result<AuthenticationSettings> {
    println!("{conf:#?}");

    let mut builder = AuthenticationSettingsBuilder::new(if is_provider {
        NatsPermission::VariableHubProvide
    } else {
        NatsPermission::VariableHubReadWrite
    });

    let env_vars = read_and_parse_env_file(&conf.cred_file)?;

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

    if let Some(token_endpoint) = &conf.oauth_token_endpoint {
        builder = builder.with_custom_oauth2_endpoint(token_endpoint.clone());
    }

    let auth_settings = builder.build();
    Ok(auth_settings)
}
