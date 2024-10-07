//! Contains the internal provider worker.
//!
//! All provider instances sends commands via tokio channels to this worker task.
//! The worker task will then process this command.
//!
//! It always saves the current state of the provider.
//! For example, the current values and the last change to the values.
//! It automatically responds to read requests.
use std::collections::{BTreeMap, HashSet};

use async_nats::{Client, Message, Subscriber};
use futures::StreamExt;

use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
};
use tracing::{debug, info, trace};

use crate::{
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEvent, ProviderDefinitionState, ProviderDefinitionT,
        ReadVariablesQueryRequest, WriteVariablesCommand,
    },
    payload_builders::{
        build_provider_definition_changed_event, build_read_variables_query_response,
        build_variables_changed_event,
    },
    subjects::{registry_provider_definition_changed_event, REGISTRY_STATE_CHANGED_EVENT_SUBJECT},
    variable::calc_variables_hash,
};

use crate::variable::{value::Value, Variable};

use super::{
    provider_options::{check_for_duplicates, ConnectError, UpdateProviderDefinitionError},
    AddVariablesError, ProviderCommand, RemoveVariablesError, SubscribeToWriteCommandError,
    UpdateVariableValuesError,
};

#[derive(Debug)]
pub(super) struct ProviderWorker {
    client: Client,
    provider_id: String,
    /// The receiver of the commands from the [`crate::provider::Provider`] instances.
    command_channel: Receiver<ProviderCommand>,
    /// Stores the variables with fast access.
    /// A BTreeMap is used instead of a HashMap because it is always deterministic.
    variables: BTreeMap<u32, Variable>,
    /// Stores the senders for the write commands.
    /// Each sender will be notified about write commands for it's subscribed variable ids.
    /// A variable can't be mapped to multiple senders to avoid conflicts.
    write_event_notiers: Vec<(HashSet<u32>, Sender<Vec<Variable>>)>,
    query_subscription: Subscriber,
    write_subscription: Subscriber,
    registry_up: Subscriber,
}

impl ProviderWorker {
    /// Creates a worker task and registers the provider.
    #[allow(clippy::new_ret_no_self)] // we return the control, because this runs in a separate thread
    pub(super) async fn new(
        client: Client,
        provider_id: String,
        variables: BTreeMap<u32, Variable>,
        wait_for_success: bool,
    ) -> Result<Sender<ProviderCommand>, ConnectError> {
        let (tx, rx) = mpsc::channel(100);
        let query_subscription = client
            .subscribe(format!("v1.loc.{}.vars.qry.read", provider_id))
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;
        let write_subscription = client
            .subscribe(format!("v1.loc.{}.vars.cmd.write", provider_id))
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;
        let registry_up = client
            .subscribe(REGISTRY_STATE_CHANGED_EVENT_SUBJECT.to_string())
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;

        let mut created = ProviderWorker {
            client: client.clone(),
            provider_id: provider_id.clone(),
            command_channel: rx,
            variables,
            write_event_notiers: vec![],
            query_subscription,
            write_subscription,
            registry_up,
        };

        created
            .update_definition(wait_for_success)
            .await
            .map_err(ConnectError::UpdateProviderDefinition)?;

        info!("u-OS Data Hub provider `{provider_id}` successfully registered");

        tokio::spawn(async move { created.run().await });
        Ok(tx)
    }

    /// Registers or updates a new provider definition to the registry.
    pub async fn update_definition(
        &mut self,
        wait_for_success: bool,
    ) -> Result<(), UpdateProviderDefinitionError> {
        let mut registry_provider_definition_updated_subscribtion = self
            .client
            .subscribe(registry_provider_definition_changed_event(
                self.provider_id.clone(),
            ))
            .await
            .map_err(|x| UpdateProviderDefinitionError::Nats(Box::new(x)))?;

        let provider_def_payload =
            build_provider_definition_changed_event(Some(ProviderDefinitionT {
                fingerprint: calc_variables_hash(&self.variables),
                variable_definitions: Some(self.variables.values().map(|var| var.into()).collect()),
                state: ProviderDefinitionState::UNSPECIFIED,
            }));
        self.client
            .publish(
                format!("v1.loc.{}.def.evt.changed", &self.provider_id).to_string(),
                provider_def_payload.clone(),
            )
            .await
            .map_err(|x| UpdateProviderDefinitionError::Nats(Box::new(x)))?;

        if wait_for_success {
            debug!("Waiting for the provider definition validation of the registry...");
            loop {
                select! {
                    Some(msg) = registry_provider_definition_updated_subscribtion.next() => {
                        if let Ok(parsed_message) =
                        flatbuffers::root::<ProviderDefinitionChangedEvent>(&msg.payload)
                        {
                            if let Some(def) = parsed_message.provider_definition() {
                                if def.state() == ProviderDefinitionState::OK {
                                    debug!("Provider definition successfully updated");
                                    return Ok(());
                                }
                                return Err(UpdateProviderDefinitionError::InvalidProviderDefinition("The registry marked the definition as invalid".to_string()));
                            }
                            return Err(UpdateProviderDefinitionError::InvalidProviderDefinition("Provider definition changed event did not contain provider definition".to_string()));
                        } else {
                            return Err(UpdateProviderDefinitionError::InvalidProviderDefinition("Could not parse provider definition changed event".to_string()));
                        }
                    }
                    Some(_) = self.registry_up.next() => {
                        // Republish the definition
                        self.client
                        .publish(
                            format!("v1.loc.{}.def.evt.changed", &self.provider_id).to_string(),
                            provider_def_payload.clone(),
                        )
                        .await.map_err(|x| UpdateProviderDefinitionError::Nats(Box::new(x)))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// The loop of the worker task.
    /// It waits for internal commands and nats messages and reacts on them.
    async fn run(mut self) {
        loop {
            // Wait for a internal command or nats message
            let msg = select! {
                msg = self.command_channel.recv() => {
                    match msg {
                        Some(msg) => msg,
                        None => break, // channel has been dropped
                    }
                }
                Some(msg) = self.query_subscription.next() => {
                    ProviderCommand::Query(msg)
                }
                Some(msg) = self.write_subscription.next() => {
                    ProviderCommand::HandleWrite(msg)
                }
                _ = self.registry_up.next() => {
                    ProviderCommand::Register
                }
            };

            // React in the command
            match msg {
                ProviderCommand::AddVariables(vars, result_tx) => {
                    result_tx.send(self.add_variables(vars, true).await).ok();
                }
                ProviderCommand::RemoveVariables(vars, result_tx) => {
                    result_tx.send(self.remove_variables(vars, true).await).ok();
                }
                ProviderCommand::UpdateValues(vars, result_tx) => {
                    result_tx.send(self.update_variable_values(vars).await).ok();
                }
                ProviderCommand::Query(msg) => {
                    self.handle_variable_read_query(msg).await;
                }
                ProviderCommand::Register => {
                    // This will be called on registry UP event.
                    // This thread crashes when the register fails.
                    // This can only fail on a nats error (e.g Permissions violation)
                    // because there were no changes to the register beforce (e.g. first register or definition change).
                    self.update_definition(true)
                        .await
                        .expect("should register provider");
                }
                ProviderCommand::Subscribe(vars, result_tx) => {
                    result_tx
                        .send(self.create_write_event_notifier(vars).await)
                        .ok();
                }
                ProviderCommand::HandleWrite(msg) => {
                    self.handle_write(msg).await;
                }
            }
        }
    }

    /// Creates a new write event notifier
    async fn create_write_event_notifier(
        &mut self,
        variables: Vec<Variable>,
    ) -> Result<mpsc::Receiver<Vec<Variable>>, SubscribeToWriteCommandError> {
        // First remove closed channels
        self.write_event_notiers
            .retain(|(_, sender)| !sender.is_closed());

        for variable in &variables {
            // Check if all ids exists
            if !self.variables.contains_key(&variable.id) {
                return Err(super::SubscribeToWriteCommandError::VariableNotFound(
                    variable.key.to_string(),
                ));
            }

            // TODO: Could we move this to compile time?
            // Check if a write event notifier for any variable still exists (avoid conflicts)
            for (ids, _) in &self.write_event_notiers {
                if ids.contains(&variable.id) {
                    return Err(super::SubscribeToWriteCommandError::AlreadySubscribed(
                        variable.key.to_string(),
                    ));
                }
            }
        }
        // Create the write event notifier
        let (tx, rx) = mpsc::channel(100);

        let variable_ids = variables.iter().map(|v| v.id).collect();
        self.write_event_notiers.push((variable_ids, tx));
        Ok(rx)
    }

    /// Handle a write command from nats
    async fn handle_write(&mut self, msg: async_nats::Message) {
        let write_command = match flatbuffers::root::<WriteVariablesCommand>(&msg.payload) {
            Ok(x) => x,
            _ => return,
        }
        .unpack();
        if write_command.variables.provider_definition_fingerprint
            != calc_variables_hash(&self.variables)
        {
            trace!("Ignore write command with wrong fingerprint");
            return;
        }
        let items = match write_command.variables.items {
            Some(x) => x,

            _ => {
                trace!("Ignore write command without items");
                return;
            }
        };

        // Add definition to items, and filter out readonly and non existing variables.
        let items = items
            .into_iter()
            .filter_map(|to_conv| {
                if let Some(current_variable) = self.variables.get(&to_conv.id) {
                    // Filter out read only variables
                    if current_variable.read_only {
                        trace!(
                            "Ignore write command on readonly variable with id `{}`",
                            to_conv.id
                        );
                        return None;
                    }
                    // TODO: Could we do this without cloning?
                    let mut variable_to_pass = current_variable.clone();
                    let new_value = <Option<Value>>::from(to_conv.value)?;
                    variable_to_pass.value = new_value.clone();
                    Some(variable_to_pass)
                } else {
                    trace!("Ignore non existing id `{}` from write command", to_conv.id);
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut dead_conns = vec![];
        for (index, (ids, tx)) in self.write_event_notiers.iter_mut().enumerate() {
            // Search out the variables for each sender
            // TODO: Could we do this without cloning?
            let items_for_sender: Vec<Variable> = items
                .iter()
                .filter(|var| ids.contains(&var.id))
                .cloned()
                .collect();

            // Forward the write command
            if !items_for_sender.is_empty() && tx.send(items_for_sender).await.is_err() {
                // Marks the sender as dead so it can removed
                dead_conns.push(index);
            }
        }

        // Remove dead connections from write_event_notiers
        self.write_event_notiers = self
            .write_event_notiers
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(index, _)| !dead_conns.contains(index))
            .map(|(_, x)| x)
            .collect();
    }

    /// Update the value of a variable. You are not allowed to change the variable value data type.
    async fn update_variable_values(
        &mut self,
        vars: Vec<Variable>,
    ) -> Result<(), UpdateVariableValuesError> {
        // Check if the variable exists and if the value type is the same.
        for update_variable in &vars {
            // TODO: Can we move some checks to compile time?
            if let Some(current_variable) = self.variables.get_mut(&update_variable.id) {
                if std::mem::discriminant(&update_variable.value)
                    != std::mem::discriminant(&current_variable.value)
                {
                    // Wrong value type
                    return Err(UpdateVariableValuesError::TypeMismatch(
                        update_variable.key.clone(),
                    ));
                }
            } else {
                // Can't find variable
                return Err(UpdateVariableValuesError::VariableNotFound(
                    update_variable.key.clone(),
                ));
            }
        }

        // Update the values internal
        vars.iter().for_each(|update_variable| {
            if let Some(current_variable) = self.variables.get_mut(&update_variable.id) {
                current_variable.value = update_variable.value.clone();
                current_variable.last_value_change = update_variable.last_value_change;
            }
        });

        // Publish the changes to nats
        self.publish_updates(Some(vars.iter().map(|x| x.id.to_owned()).collect()))
            .await
            .map_err(|e| UpdateVariableValuesError::NatsError(Box::new(e)))
    }

    /// Remove a variable from the provider
    async fn remove_variables(
        &mut self,
        vars: Vec<Variable>,
        wait_for_success: bool,
    ) -> Result<(), RemoveVariablesError> {
        vars.into_iter().for_each(|x| {
            self.variables.remove(&x.id);
        });

        self.update_definition(wait_for_success)
            .await
            .map_err(RemoveVariablesError::UpdateProviderDefinition)
    }

    /// Adds a variable to the provider
    async fn add_variables(
        &mut self,
        vars: Vec<Variable>,
        wait_for_success: bool,
    ) -> Result<(), AddVariablesError> {
        check_for_duplicates(&self.variables, &vars).map_err(|e| match e {
            super::provider_options::AddVariablesError::DuplicatedId(id) => {
                AddVariablesError::DuplicatedId(id)
            }
            super::provider_options::AddVariablesError::DuplicatedKey(key) => {
                AddVariablesError::DuplicatedKey(key)
            }
        })?;

        self.variables
            .extend(vars.iter().map(|variable| (variable.id, variable.clone())));
        self.update_definition(wait_for_success)
            .await
            .map_err(AddVariablesError::UpdateProviderDefinition)?;

        self.publish_updates(Some(vars.iter().map(|x| x.id.to_owned()).collect()))
            .await
            .map_err(|e| AddVariablesError::NatsError(Box::new(e)))?;
        Ok(())
    }

    /// Handles the variable read query. It answers with the cached values.
    async fn handle_variable_read_query(&self, msg: Message) {
        let read_request = match flatbuffers::root::<ReadVariablesQueryRequest>(&msg.payload) {
            Ok(x) => x,
            _ => return,
        };
        let reply_subject = match msg.reply {
            Some(x) => x,
            None => return,
        };

        let response = build_read_variables_query_response(read_request.unpack(), &self.variables);
        self.client
            .publish(reply_subject.into_string(), response)
            .await
            .ok();
    }

    /// Publish value updates
    async fn publish_updates(
        &mut self,
        changed: Option<Vec<u32>>,
    ) -> Result<(), async_nats::error::Error<async_nats::client::PublishErrorKind>> {
        let to_publish = match changed {
            Some(changed) => {
                let mut filtered = BTreeMap::new();
                for changed_id in changed {
                    if let Some(variable) = self.variables.get(&changed_id) {
                        filtered.insert(changed_id, variable.clone());
                    }
                }
                filtered
            }
            None => self.variables.clone(),
        };

        let payload = build_variables_changed_event(&to_publish);

        self.client
            .publish(
                format!("v1.loc.{}.vars.evt.changed", self.provider_id),
                payload,
            )
            .await
    }
}
