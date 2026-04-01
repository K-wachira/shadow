use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct ShadowMind {
    pub meta: Meta,
    pub surface: HashMap<String, Belief>,
    pub behavioural: HashMap<String, Belief>,
    pub mental_model: HashMap<String, Belief>,
    pub values: HashMap<String, Belief>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
    pub version: u32,
    pub last_updated: String,
    pub log_range: Option<LogRange>,
    pub total_logs_considered: u32,
    pub rewrite_trigger: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogRange {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Belief {
    pub value: String,
    pub confidence: f32,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Operation {
    pub op: OpKind,
    pub value: String,
    pub trigger: String,
    pub date: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpKind {
    Create,
    Update,
    Delete,
}
