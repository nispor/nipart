// SPDX-License-Identifier: Apache-2.0

// This file is based on the work of nmstate project(https://nmstate.io/) which
// is under license of Apache 2.0, author of nmstate origin file is:
//  * Gris Ge <fge@redhat.com>

// Hold 'Database Operations' defined in RFC-7047

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::db::OvsDbCondition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OvsDbOperation {
    Select(OvsDbSelect),
    _Update(OvsDbUpdate),
    _Mutate(OvsDbMutate),
}

impl OvsDbOperation {
    pub(crate) fn to_value(&self) -> Value {
        match self {
            Self::Select(s) => s.to_value(),
            Self::_Update(s) => s.to_value(),
            Self::_Mutate(s) => s.to_value(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OvsDbSelect {
    pub(crate) table: String,
    pub(crate) conditions: Vec<OvsDbCondition>,
    pub(crate) columns: Option<Vec<&'static str>>,
}

impl OvsDbSelect {
    pub(crate) fn to_value(&self) -> Value {
        let mut ret = Map::new();
        ret.insert("op".to_string(), Value::String("select".to_string()));
        ret.insert("table".to_string(), Value::String(self.table.clone()));
        let condition_values: Vec<Value> =
            self.conditions.iter().map(|c| c.to_value()).collect();
        ret.insert("where".to_string(), Value::Array(condition_values));
        if let Some(columns) = self.columns.as_ref() {
            ret.insert(
                "columns".to_string(),
                Value::Array(
                    columns
                        .as_slice()
                        .iter()
                        .map(|c| Value::String(c.to_string()))
                        .collect(),
                ),
            );
        }
        Value::Object(ret)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OvsDbUpdate {
    pub(crate) table: String,
    pub(crate) conditions: Vec<OvsDbCondition>,
    pub(crate) row: HashMap<String, Value>,
}

impl OvsDbUpdate {
    pub(crate) fn to_value(&self) -> Value {
        let mut ret = Map::new();
        ret.insert("op".to_string(), Value::String("update".to_string()));
        ret.insert("table".to_string(), Value::String(self.table.clone()));
        let condition_values: Vec<Value> =
            self.conditions.iter().map(|c| c.to_value()).collect();
        ret.insert("where".to_string(), Value::Array(condition_values));
        let mut row_map = Map::new();
        for (k, v) in self.row.iter() {
            row_map.insert(k.to_string(), v.clone());
        }
        ret.insert("row".to_string(), Value::Object(row_map));
        Value::Object(ret)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OvsDbMutation {
    pub(crate) column: String,
    pub(crate) mutator: String,
    pub(crate) value: Value,
}

impl OvsDbMutation {
    pub(crate) fn to_value(&self) -> Value {
        Value::Array(vec![
            Value::String(self.column.clone()),
            Value::String(self.mutator.clone()),
            self.value.clone(),
        ])
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct OvsDbMutate {
    pub(crate) table: String,
    pub(crate) conditions: Vec<OvsDbCondition>,
    pub(crate) mutations: Vec<OvsDbMutation>,
}

impl OvsDbMutate {
    pub(crate) fn to_value(&self) -> Value {
        let mut ret = Map::new();
        ret.insert("op".to_string(), Value::String("mutate".to_string()));
        ret.insert("table".to_string(), Value::String(self.table.clone()));
        let condition_values: Vec<Value> =
            self.conditions.iter().map(|c| c.to_value()).collect();
        ret.insert("where".to_string(), Value::Array(condition_values));
        let mutations: Vec<Value> = self
            .mutations
            .as_slice()
            .iter()
            .map(|m| m.to_value())
            .collect();
        ret.insert("mutations".to_string(), Value::Array(mutations));
        Value::Object(ret)
    }
}
