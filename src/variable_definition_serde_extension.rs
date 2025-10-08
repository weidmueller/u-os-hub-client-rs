// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::generated::weidmueller::ucontrol::hub::VariableDefinitionT;

impl Serialize for VariableDefinitionT {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("definition", 2)?;
        let _ = s.serialize_field("data_type", &self.data_type);
        let _ = s.serialize_field("access_type", &self.access_type);
        s.end()
    }
}
