use tokio::{
    select, task,
    time::{sleep, Duration},
};
use uc_hub_client::{
    provider::{Provider, ProviderOptions, VariableBuilder},
    variable::value::Value,
};

#[tokio::main]
async fn main() {
    let builder = ProviderOptions::new("example-provider");

    let hub_provider = builder.register_and_connect("nats:4222").await.unwrap();

    // The provider can be copied into different tasks
    let provider_cloned = hub_provider.clone();
    task::spawn(async move {
        example_service_1(provider_cloned).await;
    });

    example_service_2(hub_provider.clone()).await;
}

async fn example_service_1(hub_provider: Provider) {
    let dat1_builder = VariableBuilder::new(0, "folder1.data1").value(Value::Boolean(false));

    let mut data1 = dat1_builder.build().unwrap();

    let folder_version = VariableBuilder::new(1, "folder1.version")
        .value(Value::String("1.0.0".to_string()))
        .build()
        .unwrap();

    hub_provider
        .add_variables(&[data1.clone(), folder_version.clone()])
        .await
        .unwrap();

    loop {
        data1.value = Value::String("Test".to_string());

        hub_provider
            .update_variable_values(vec![data1.clone()])
            .await
            .ok();
        sleep(Duration::from_secs(1)).await;
    }
}

async fn example_service_2(hub_provider: Provider) {
    let dat1_builder = VariableBuilder::new(3, "folder2.data1").value(Value::Boolean(true));

    let mut data1 = dat1_builder.build().unwrap();

    let folder_version = VariableBuilder::new(4, "folder2.version")
        .value("1.0.0".into())
        .build()
        .unwrap();

    hub_provider
        .add_variables(&[data1.clone(), folder_version.clone()])
        .await
        .unwrap();

    let mut interval = tokio::time::interval(Duration::from_secs(1));

    let mut rw_subscribtion = hub_provider
        .subscribe_to_write_command(&[data1.clone()])
        .await
        .unwrap();

    loop {
        select! {
            _ = interval.tick() => {
                data1.value = Value::String("Test".to_string());

                hub_provider.update_variable_values(vec![data1.clone()]).await.unwrap();
            }

            Some(vars) = rw_subscribtion.recv() => {
                // Just accept all
                hub_provider.update_variable_values(vars).await.unwrap();
            }
        }
    }
}
