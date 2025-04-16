//! Contains the internal provider worker.
//!
//! All provider instances sends commands via tokio channels to this worker task.
//! The worker task will then process this command.
//!
//! It always saves the current state of the provider.
//! For example, the current values and the last change to the values.
//! It automatically responds to read requests.
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use async_nats::{Event, Message, Subscriber};
use futures::StreamExt;

use tokio::{
    select,
    sync::{
        broadcast::{self, error::RecvError},
        mpsc,
    },
    time::timeout,
};
use tracing::{debug, error, info, trace};

use crate::{
    authenticated_nats_con::AuthenticatedNatsConnection,
    generated::weidmueller::ucontrol::hub::{
        ProviderDefinitionChangedEvent, ProviderDefinitionState, ProviderDefinitionT,
        ReadVariablesQueryRequest, WriteVariablesCommand,
    },
    nats_subjects,
    payload_builders::{
        build_provider_definition_changed_event, build_read_variables_query_response,
        build_variables_changed_event,
    },
    variable::calc_variables_hash,
};

use crate::variable::{value::Value, Variable};

use super::{
    provider_options::{check_for_duplicates, ConnectError, UpdateProviderDefinitionError},
    AddVariablesError, ProviderCommand, RemoveVariablesError, SubscribeToWriteCommandError,
    UpdateVariableValuesError,
};

/// The state of the provider worker.
#[derive(Debug, Copy, Clone)]
pub enum State {
    Connecting,
    Registering,
    Running,
}

#[derive(Debug)]
pub(super) struct ProviderWorker {
    nats_con: Arc<AuthenticatedNatsConnection>,
    state: State,
    state_changed_sender: tokio::sync::broadcast::Sender<State>,
    /// The receiver of the commands from the [`crate::provider::Provider`] instances.
    command_channel: mpsc::Receiver<ProviderCommand>,
    /// Stores the variables with fast access.
    /// A BTreeMap is used instead of a HashMap because it is always deterministic.
    variables: BTreeMap<u32, Variable>,
    /// The current fingerprint of the provider definition.
    current_fingerprint: u64,
    /// Stores the senders for the write commands.
    /// Each sender will be notified about write commands for it's subscribed variable ids.
    /// A variable can't be mapped to multiple senders to avoid conflicts.
    write_event_notiers: Vec<(HashSet<u32>, mpsc::Sender<Vec<Variable>>)>,
    query_subscription: Subscriber,
    write_subscription: Subscriber,
    registry_up: Subscriber,
}

impl ProviderWorker {
    /// Creates a worker task and registers the provider.
    #[allow(clippy::new_ret_no_self)] // we return the control, because this runs in a separate thread
    pub(super) async fn new(
        nats_con: Arc<AuthenticatedNatsConnection>,
        variables: BTreeMap<u32, Variable>,
        wait_for_success: bool,
    ) -> Result<mpsc::Sender<ProviderCommand>, ConnectError> {
        let client = nats_con.get_client();
        let provider_id = nats_con.get_client_name();
        let nats_events = nats_con.get_events();

        let (tx, rx) = mpsc::channel(100);
        let query_subscription = client
            .subscribe(nats_subjects::read_variables_query(provider_id))
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;
        let write_subscription = client
            .subscribe(nats_subjects::write_variables_command(provider_id))
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;
        let registry_up = client
            .subscribe(nats_subjects::registry_state_changed_event())
            .await
            .map_err(|x| ConnectError::Nats(Box::new(x)))?;

        let (state_sender, mut state_receiver) = tokio::sync::broadcast::channel(1);

        //Calculate initial FP. The FP will change each time the variables in the definition change.
        let current_fingerprint = calc_variables_hash(&variables);

        let mut created = ProviderWorker {
            nats_con,
            state: State::Connecting,
            state_changed_sender: state_sender,
            command_channel: rx,
            variables,
            current_fingerprint,
            write_event_notiers: vec![],
            query_subscription,
            write_subscription,
            registry_up,
        };

        if created.nats_con.get_client().connection_state()
            != async_nats::connection::State::Connected
        {
            created.enter_state(State::Connecting).await;
        } else {
            created.enter_state(State::Registering).await;
        }

        tokio::spawn(async move { created.run(nats_events).await });

        if wait_for_success {
            // Wait until the provider is registered or timeout after 5 minutes
            timeout(tokio::time::Duration::from_secs(300), async {
                loop {
                    if let Ok(State::Running) = state_receiver.recv().await {
                        break;
                    }
                }
            })
            .await
            .map_err(|_| ConnectError::Timeout)?;
        }

        Ok(tx)
    }

    /// Changes the state of the provider.
    pub async fn enter_state(&mut self, new_state: State) {
        debug!("Provider is {new_state:?}");
        self.state_changed_sender.send(new_state).ok();
        self.state = new_state;
    }

    fn get_nats_client(&self) -> &async_nats::Client {
        self.nats_con.get_client()
    }

    fn get_provider_id(&self) -> &str {
        self.nats_con.get_client_name()
    }

    /// Sends an empty provider definition to the registry to unregister the provider.
    pub async fn send_empty_definition(&self) -> Result<(), UpdateProviderDefinitionError> {
        let provider_def_payload = build_provider_definition_changed_event(None);

        self.get_nats_client()
            .publish(
                nats_subjects::provider_changed_event(self.get_provider_id()),
                provider_def_payload,
            )
            .await
            .map_err(|x| UpdateProviderDefinitionError::Nats(Box::new(x)))?;

        Ok(())
    }

    /// Registers or updates a new provider definition to the registry.
    pub async fn update_definition(
        &mut self,
        wait_for_success: bool,
    ) -> Result<(), UpdateProviderDefinitionError> {
        // Update the fingerprint
        self.current_fingerprint = calc_variables_hash(&self.variables);

        let mut registry_provider_definition_updated_subscribtion = self
            .get_nats_client()
            .subscribe(nats_subjects::registry_provider_definition_changed_event(
                self.get_provider_id(),
            ))
            .await
            .map_err(|x| UpdateProviderDefinitionError::Nats(Box::new(x)))?;

        let provider_def_payload =
            build_provider_definition_changed_event(Some(ProviderDefinitionT {
                fingerprint: self.current_fingerprint,
                variable_definitions: Some(self.variables.values().map(|var| var.into()).collect()),
                state: ProviderDefinitionState::UNSPECIFIED,
            }));
        self.get_nats_client()
            .publish(
                nats_subjects::provider_changed_event(self.get_provider_id()),
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
                        self.get_nats_client()
                            .publish(
                                nats_subjects::provider_changed_event(self.get_provider_id()),
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
    ///
    /// It handles the different state events and can be interrupted by nats events.
    async fn run(mut self, mut nats_events: broadcast::Receiver<Event>) {
        while !self.command_channel.is_closed() {
            match &self.state {
                State::Connecting => {
                    let msg = nats_events.recv().await;
                    self.handle_nats_event(msg).await;
                }
                State::Registering => {
                    select! {
                        // run_state_registering() must be safe to abort on every "await" (by select)
                        _ = self.run_state_registering() => {},
                        msg = nats_events.recv() => self.handle_nats_event(msg).await,
                    };
                }
                State::Running => {
                    select! {
                        command = self.run_state_running_receive_command() => self.run_state_running_handle_command(command).await,
                        msg = nats_events.recv() => self.handle_nats_event(msg).await,
                    };
                }
            }
        }
    }

    /// Handles the nats events.
    async fn handle_nats_event(&mut self, msg: Result<Event, RecvError>) {
        match msg {
            Ok(Event::Connected) => {
                debug!("Connected to NATS");
                self.enter_state(State::Registering).await;
            }
            Ok(Event::Disconnected) => {
                debug!("Disconnected from NATS");
                self.enter_state(State::Connecting).await;
            }
            Ok(Event::LameDuckMode) => {
                debug!("NATS server entered lame duck mode");
            }
            Ok(Event::SlowConsumer(_)) => {
                debug!("Slow consumer detected");
            }
            Ok(Event::ServerError(error)) => {
                debug!("Server Error: {:?}", error);
            }
            Ok(Event::ClientError(error)) => {
                debug!("Client Error: {:?}", error);
            }
            Err(error) => {
                debug!("NATS event channel closed, error: {:?}", error);
            }
        }
    }

    /// Handler for the registering state
    ///
    /// Waits for the provider to be registered.
    ///
    /// run_state_registering() must be safe to abort on every "await" (by select)
    async fn run_state_registering(&mut self) {
        let result = self
            .update_definition(true)
            .await
            .map_err(ConnectError::UpdateProviderDefinition);

        if let Err(e) = result {
            // TODO: refactor the error handling, should this be returned on connect? Is this even possible because of the further checks?
            error!(
                "u-OS Data Hub provider `{}` failed to register: {:?}",
                self.get_provider_id(),
                e
            );
            panic!(
                "u-OS Data Hub provider `{}` failed to register: {:?}",
                self.get_provider_id(),
                e
            );
        }

        info!(
            "u-OS Data Hub provider `{}` successfully registered",
            self.get_provider_id()
        );
        self.enter_state(State::Running).await;
    }

    /// Receive commands for the worker task.
    async fn run_state_running_receive_command(&mut self) -> ProviderCommand {
        let msg = select! {
            msg = self.command_channel.recv() => {
                match msg {
                    Some(msg) => msg,
                    None => {
                        //If the command channel is closed, we need to unregister the provider
                        //The channel only closes if the connected Provider object is dropped
                        ProviderCommand::Unregister
                    }
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
        msg
    }

    /// Handler for the running state
    ///
    /// Executes commands for the worker task.
    async fn run_state_running_handle_command(&mut self, command: ProviderCommand) {
        // React in the command
        match command {
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
            ProviderCommand::Unregister => {
                self.send_empty_definition()
                    .await
                    .expect("should unregister provider");
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
        if write_command.variables.provider_definition_fingerprint != self.current_fingerprint {
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

        let response = build_read_variables_query_response(
            read_request.unpack(),
            &self.variables,
            self.current_fingerprint,
        );
        self.get_nats_client()
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

        let payload = build_variables_changed_event(&to_publish, self.current_fingerprint);

        self.get_nats_client()
            .publish(
                nats_subjects::vars_changed_event(self.get_provider_id()),
                payload,
            )
            .await
    }
}
