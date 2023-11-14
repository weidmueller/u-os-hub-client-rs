//! Collection of types that are used in the provider side of the library.
//! These types abstract the low level flatbuffer types and provide a more user-friendly interface.

use crate::dh_types::{self, TimestampValue, VariableID, VariableQuality, VariableValue};

/// Errors for data hub type conversions
//TODO: how to handle type conversion errors? should that really cause an error, or do we want to have placeholder values?
pub type Error = dh_types::Error;

/// Represents the mutable state of a provider variable in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableState {
    // Members are explicitly private outside this crate, so that users can only change them via the provided methods.
    // This is to ensure that timestamps are always set correctly.
    pub(crate) id: VariableID,
    pub(crate) timestamp: Option<TimestampValue>,
    pub(crate) value: VariableValue,
    pub(crate) quality: VariableQuality,
}

impl VariableState {
    /// Sets the value and updates the timestamp.
    ///
    /// Please see documentation of [`Self::set_all`] for more details.
    #[inline(always)]
    pub fn set_value(&mut self, value: impl Into<VariableValue>) {
        self.value = value.into();
        self.timestamp = Some(TimestampValue::now());
    }

    /// Sets the quality and updates the timestamp.
    ///
    /// Please see documentation of [`Self::set_all`] for more details.
    #[inline(always)]
    pub fn set_quality(&mut self, quality: VariableQuality) {
        self.quality = quality;
        self.timestamp = Some(TimestampValue::now());
    }

    /// Explicitly sets all properties of the variable state.
    ///
    /// This is useful if you need full control over the variable state or want to calculate the timestamp manually.
    ///
    /// Please note that this only changes the variable state in the RAM.
    /// For changes to take effect on the data hub, you still need to call
    /// [`Provider::update_variable_states`](crate::provider::Provider::update_variable_states) afterwards.
    ///
    /// If timestamp is set to `None`, the variable will inherit its timestamp from the variable list
    /// when a consumer receives a variable update or reads the variable list explicitly.
    /// This can be a useful optimization if you have a very large variable list,
    /// as this reduces the payload size for the data hub.
    /// However, be aware that if the variable timestamp is `None`, consumers will always receive the timestamp of
    /// reading the variable list instead of the timestamp of the last value update, which may not be what the consumer expects, so
    /// use this optimization with caution.
    #[inline(always)]
    pub fn set_all(
        &mut self,
        value: impl Into<VariableValue>,
        quality: VariableQuality,
        timestamp: Option<TimestampValue>,
    ) {
        self.value = value.into();
        self.quality = quality;
        self.timestamp = timestamp;
    }

    /// Returns the current value of the variable.
    #[inline(always)]
    pub fn get_value(&self) -> &VariableValue {
        &self.value
    }

    /// Returns the current quality of the variable.
    #[inline(always)]
    pub fn get_quality(&self) -> &VariableQuality {
        &self.quality
    }

    /// Returns the current timestamp of the variable.
    ///
    /// If timestamp is `None`, the variable will inherit its timestamp from the variable list.
    #[inline(always)]
    pub fn get_timestamp(&self) -> &Option<TimestampValue> {
        &self.timestamp
    }

    /// Returns the ID of the variable.
    ///
    /// This can be used to identify the variable definition that belongs to this state.
    #[inline(always)]
    pub fn get_id(&self) -> VariableID {
        self.id
    }
}

/// Represents a variable write command from a consumer.
///
/// Note that consumers can only request to change the value of a variable, but timestamp and quality are controlled by the provider.
/// A provider may also reject the write request based on its own logic.
#[derive(Debug, Clone)]
pub struct VariableWriteCommand {
    /// The ID of the variable to be written.
    pub id: VariableID,
    /// The requested new value of the variable.
    pub value: VariableValue,
}
