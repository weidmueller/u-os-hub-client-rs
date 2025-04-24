use std::{sync::Arc, time::Duration};

use tokio::sync::Notify;
use u_os_hub_client::{
    provider::{ProviderOptions, VariableBuilder},
    variable::value::Value,
};

use crate::utils::create_auth_con;

pub const PROVIDER_ID: &str = "dummy_provider";
pub const VARIABLE_UPDATE_RATE: Duration = Duration::from_millis(200);

pub struct DummyProvider {
    change_vars_notify: Arc<Notify>,
    worker_task: tokio::task::JoinHandle<()>,
}

impl Drop for DummyProvider {
    fn drop(&mut self) {
        self.worker_task.abort();
    }
}

impl DummyProvider {
    pub async fn new() -> anyhow::Result<Self> {
        Self::new_with_delay(Duration::ZERO).await
    }

    pub async fn new_with_delay(registration_delay: Duration) -> anyhow::Result<Self> {
        //Create connection for dummy provider
        let auth_nats_con = create_auth_con(PROVIDER_ID).await;

        let provider_builder = ProviderOptions::new();

        //add two readonly and two RW variables
        let mut ro_float = VariableBuilder::new(100, "my_folder.ro_float")
            .value(Value::Float64(123.0))
            .experimental()
            .build()?;

        let rw_string = VariableBuilder::new(200, "my_folder.rw_string")
            .read_write()
            .value(Value::String("write me!".to_string()))
            .build()?;

        let rw_int = VariableBuilder::new(300, "my_folder.rw_int")
            .read_write()
            .value(Value::Int(1000))
            .build()?;

        let mut ro_int = VariableBuilder::new(400, "my_folder.ro_int")
            .value(Value::Int(0))
            .build()?;

        let mut cur_vars = vec![
            ro_float.clone(),
            rw_string.clone(),
            rw_int.clone(),
            ro_int.clone(),
        ];
        let provider_opts = provider_builder.add_variables(cur_vars.clone())?;

        let change_vars_notify = Arc::new(Notify::new());

        //start worker thread
        let change_vars_notify_clone = change_vars_notify.clone();
        let worker_task = tokio::spawn(async move {
            if !registration_delay.is_zero() {
                tokio::time::sleep(registration_delay).await;
            }

            let provider = provider_opts
                .register_with_existing_connection(auth_nats_con)
                .await
                .unwrap();

            //Register write handler for RW vars
            let mut subscribtion_to_write_cmd = provider
                .subscribe_to_write_command(&[rw_string.clone(), rw_int.clone()])
                .await
                .unwrap();

            let mut var_write_timer = tokio::time::interval(VARIABLE_UPDATE_RATE);
            let mut cur_float_val = 0.0;
            let mut cur_int_val = 1000;

            var_write_timer.tick().await; //skip first tick

            loop {
                tokio::select! {
                    //wait for timer
                    _ = var_write_timer.tick() => {
                        ro_float.value = Value::Float64(cur_float_val);
                        ro_int.value = Value::Int(cur_int_val);

                        let updated_vars = vec![ro_float.clone(), ro_int.clone()];
                        provider.update_variable_values(updated_vars).await.unwrap();

                        cur_float_val += 123.0;
                        cur_int_val += 1;
                    },
                    //change variables
                    _ = change_vars_notify_clone.notified() => {
                        //remove all existing vars
                        provider.remove_variables(cur_vars.clone()).await.unwrap();

                        let new_ro_float = VariableBuilder::new(10, "my_folder.ro_float")
                            .value(Value::Float64(255.0))
                            .build().unwrap();

                        let new_ro_int = VariableBuilder::new(40, "my_folder.ro_int2")
                            .value(Value::Int(0))
                            .build().unwrap();

                        //change variable defs
                        let new_vars = vec![
                            new_ro_int.clone(),
                            new_ro_float.clone(),
                            VariableBuilder::new(20, "my_folder.rw_string")
                                .read_write()
                                .value(Value::String("new string value".to_string()))
                                .build().unwrap(),
                            VariableBuilder::new(30, "my_folder.rw_int2")
                                .read_write()
                                .value(Value::Int(-1000))
                                .build().unwrap(),
                            VariableBuilder::new(50, "my_folder.rw_int3")
                                .read_write()
                                .value(Value::Int(-500))
                                .build().unwrap(),
                        ];

                        provider.add_variables(&new_vars).await.unwrap();
                        cur_vars = new_vars;
                        ro_float = new_ro_float;
                        ro_int = new_ro_int;
                    }
                    //wait for write command
                    Some(written_vars) = subscribtion_to_write_cmd.recv() => {
                        provider.update_variable_values(written_vars).await.unwrap();
                    }
                }
            }
        });

        Ok(Self {
            worker_task,
            change_vars_notify,
        })
    }

    pub fn change_variables(&self) {
        self.change_vars_notify.notify_one();
    }
}
