use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowMind {
    pub meta: Meta,
    pub surface: HashMap<String, Belief>,
    pub behavioural: HashMap<String, Belief>,
    pub mental_model: HashMap<String, Belief>,
    pub values: HashMap<String, Belief>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub version: u32,
    pub last_updated: String,
    pub log_range: Option<LogRange>,
    pub total_logs_considered: u32,
    pub rewrite_trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRange {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    pub value: String,
    pub confidence: f32,
    pub source_logs: Vec<String>,
    pub last_updated: String,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub op: OpKind,
    pub value: String,
    pub trigger: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OpKind {
    Create,
    Update,
    Delete,
}

impl Default for ShadowMind {
    fn default() -> Self {
        Self {
            meta: Meta {
                version: 1,
                last_updated: String::new(),
                log_range: None,
                total_logs_considered: 0,
                rewrite_trigger: String::new(),
            },
            surface: HashMap::new(),
            behavioural: HashMap::new(),
            mental_model: HashMap::new(),
            values: HashMap::new(),
        }
    }
}

impl Default for Belief {
    fn default() -> Self {
        Self {
            value: String::new(),
            confidence: 0.0,
            source_logs: Vec::new(),
            last_updated: String::new(),
            operations: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shadow_mind_has_empty_maps() {
        let mind = ShadowMind::default();
        assert!(mind.surface.is_empty());
        assert!(mind.behavioural.is_empty());
        assert!(mind.mental_model.is_empty());
        assert!(mind.values.is_empty());
    }

    #[test]
    fn default_shadow_mind_meta_version_is_one() {
        let mind = ShadowMind::default();
        assert_eq!(mind.meta.version, 1);
    }

    #[test]
    fn default_belief_has_zero_confidence() {
        let belief = Belief::default();
        assert_eq!(belief.confidence, 0.0);
        assert!(belief.value.is_empty());
        assert!(belief.source_logs.is_empty());
    }

    #[test]
    fn op_kind_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&OpKind::Create).unwrap(),
            "\"create\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::Update).unwrap(),
            "\"update\""
        );
        assert_eq!(
            serde_json::to_string(&OpKind::Delete).unwrap(),
            "\"delete\""
        );
    }

    #[test]
    fn op_kind_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<OpKind>("\"create\"").unwrap(),
            OpKind::Create
        );
        assert_eq!(
            serde_json::from_str::<OpKind>("\"update\"").unwrap(),
            OpKind::Update
        );
        assert_eq!(
            serde_json::from_str::<OpKind>("\"delete\"").unwrap(),
            OpKind::Delete
        );
    }

    #[test]
    fn belief_serializes_and_deserializes() {
        let belief = Belief {
            value: "test belief".into(),
            confidence: 0.75,
            source_logs: vec!["log1".into(), "log2".into()],
            last_updated: "2026-01-15".into(),
            operations: vec![Operation {
                op: OpKind::Create,
                value: "initial".into(),
                trigger: "init".into(),
                date: "2026-01-01".into(),
            }],
        };
        let json = serde_json::to_string(&belief).unwrap();
        let deserialized: Belief = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.value, belief.value);
        assert_eq!(deserialized.confidence, belief.confidence);
        assert_eq!(deserialized.source_logs, belief.source_logs);
        assert_eq!(deserialized.operations.len(), 1);
        assert_eq!(deserialized.operations[0].op, OpKind::Create);
    }

    #[test]
    fn shadow_mind_serializes_with_maps() {
        let mut mind = ShadowMind::default();
        mind.surface.insert(
            "key1".into(),
            Belief {
                value: "val1".into(),
                confidence: 0.5,
                source_logs: vec![],
                last_updated: "2026-01-01".into(),
                operations: vec![],
            },
        );
        let json = serde_json::to_string(&mind).unwrap();
        assert!(json.contains("key1"));
        assert!(json.contains("val1"));
    }

    #[test]
    fn meta_serializes_with_optional_range() {
        let meta = Meta {
            version: 1,
            last_updated: "2026-01-01".into(),
            log_range: Some(LogRange {
                from: "2026-01-01".into(),
                to: "2026-01-31".into(),
            }),
            total_logs_considered: 100,
            rewrite_trigger: "trigger".into(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("log_range"));
        assert!(json.contains("from"));
        assert!(json.contains("2026-01-01"));
    }

    #[test]
    fn meta_serializes_with_none_range() {
        let meta = Meta {
            version: 1,
            last_updated: "2026-01-01".into(),
            log_range: None,
            total_logs_considered: 0,
            rewrite_trigger: "init".into(),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("log_range"));
        assert!(json.contains("null"));
    }

    #[test]
    fn operation_serializes_all_fields() {
        let op = Operation {
            op: OpKind::Update,
            value: "updated value".into(),
            trigger: "evidence".into(),
            date: "2026-02-01".into(),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("update"));
        assert!(json.contains("updated value"));
        assert!(json.contains("evidence"));
        assert!(json.contains("2026-02-01"));
    }
}
