//!Utility functions and shared code for the examples

use clap::Parser;
use u_os_hub_client::{
    authenticated_nats_con::{
        AuthenticationSettings, AuthenticationSettingsBuilder, NatsPermission,
    },
    oauth2::OAuth2Credentials,
};

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(long, default_value = "127.0.0.1")]
    pub nats_ip: String,
    #[clap(long, default_value_t = 4222)]
    pub nats_port: u16,
    #[clap(long)]
    pub client_name: String,
    #[clap(long, default_value = "")]
    pub client_id: String,
    #[clap(long, default_value = "")]
    pub client_secret: String,
}

pub fn build_auth_settings_from_args(args: &Args, is_provider: bool) -> AuthenticationSettings {
    println!("{args:#?}");

    let mut builder = AuthenticationSettingsBuilder::new(if is_provider {
        NatsPermission::VariableHubProvide
    } else {
        NatsPermission::VariableHubReadWrite
    });

    builder = builder.with_credentials(OAuth2Credentials {
        client_name: args.client_name.clone(),
        client_id: args.client_id.clone(),
        client_secret: args.client_secret.clone(),
    });

    builder = builder.with_custom_oauth2_endpoint(format!("https://{}/oauth2/token", args.nats_ip));
    builder.build()
}
