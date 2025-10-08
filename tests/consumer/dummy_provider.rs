// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

use std::{sync::Arc, time::Duration};

use tokio::sync::Notify;
use u_os_hub_client::{
    dh_types::{VariableAccessType, VariableValue},
    provider::{ProviderBuilder, VariableBuilder},
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

        let provider_builder = ProviderBuilder::new();

        //add two readonly and two RW variables
        let mut ro_float = VariableBuilder::new(100, "my_folder.ro_float")
            .initial_value(VariableValue::Float64(123.0))
            .experimental()
            .build()?;

        let rw_string = VariableBuilder::new(200, "my_folder.rw_string")
            .access_type(VariableAccessType::ReadWrite)
            .initial_value("write me!")
            .build()?;

        let rw_int = VariableBuilder::new(300, "my_folder.rw_int")
            .access_type(VariableAccessType::ReadWrite)
            .initial_value(VariableValue::Int(1000))
            .build()?;

        let mut ro_int = VariableBuilder::new(400, "my_folder.ro_int")
            .initial_value(VariableValue::Int(0))
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
            let mut subscription_to_write_cmd = provider
                .subscribe_to_write_command(vec![rw_string.clone(), rw_int.clone()])
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
                        let ro_float_state = ro_float.get_mut_state();
                        let ro_int_state = ro_int.get_mut_state();

                        ro_float_state.set_value(cur_float_val);
                        ro_int_state.set_value(cur_int_val);

                        let updated_vars = vec![ro_float_state.clone(), ro_int_state.clone()];
                        provider.update_variable_states(updated_vars).await.unwrap();

                        cur_float_val += 123.0;
                        cur_int_val += 1;
                    },
                    //change variables
                    _ = change_vars_notify_clone.notified() => {
                        //remove all existing vars
                        provider.remove_variables(cur_vars.clone()).await.unwrap();

                        let new_ro_float = VariableBuilder::new(10, "my_folder.ro_float")
                            .initial_value(VariableValue::Float64(255.0))
                            .build().unwrap();

                        let new_ro_int = VariableBuilder::new(40, "my_folder.ro_int2")
                            .initial_value(VariableValue::Int(0))
                            .build().unwrap();

                        //change variable defs
                        let new_vars = vec![
                            new_ro_int.clone(),
                            new_ro_float.clone(),
                            VariableBuilder::new(20, "my_folder.rw_string")
                                .access_type(VariableAccessType::ReadWrite)
                                .initial_value("new string value")
                                .build().unwrap(),
                            VariableBuilder::new(30, "my_folder.rw_int2")
                                .access_type(VariableAccessType::ReadWrite)
                                .initial_value(VariableValue::Int(-1000))
                                .build().unwrap(),
                            VariableBuilder::new(50, "my_folder.rw_int3")
                                .access_type(VariableAccessType::ReadWrite)
                                .initial_value(VariableValue::Int(-500))
                                .build().unwrap(),
                        ];

                        provider.add_variables(new_vars.clone()).await.unwrap();
                        cur_vars = new_vars;
                        ro_float = new_ro_float;
                        ro_int = new_ro_int;
                    }
                    //wait for write command
                    Some(write_commands) = subscription_to_write_cmd.recv() => {
                        // Just accept all and update the states
                        let mut updated_states = Vec::with_capacity(write_commands.len());

                        for write_cmd in write_commands {
                            //Note: Should use hashmap in production code here, but performance is ok for dummy
                            let written_var = cur_vars
                                .iter_mut()
                                .find(|var| var.get_definition().id == write_cmd.id).ok_or(
                                    anyhow::anyhow!("Received write command for unknown variable: {}", write_cmd.id)
                                ).unwrap();

                            let written_var_state = written_var.get_mut_state();
                            written_var_state.set_value(write_cmd.value);
                            updated_states.push(written_var_state.clone());
                        }

                        provider.update_variable_states(updated_states).await.unwrap();
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
