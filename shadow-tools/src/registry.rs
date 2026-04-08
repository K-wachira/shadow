use crate::fetch;
use crate::protocol::ChatTool;
use crate::protocol::ChatToolCall;
use crate::protocol::ToolDefinition;
use crate::search;
use crate::weather;
use color_eyre::eyre::eyre;
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct ToolRegistry {
    definitions: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();
        registry.register(weather::tool());
        registry.register(search::tool());
        registry.register(fetch::tool());
        registry
    }

    pub fn register(&mut self, tool: ToolDefinition) {
        self.definitions
            .insert(tool.schema().function.name.clone(), tool);
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn schemas(&self) -> Vec<ChatTool> {
        self.definitions
            .values()
            .map(ToolDefinition::schema)
            .collect()
    }

    pub async fn execute(&self, call: &ChatToolCall) -> color_eyre::Result<String> {
        let tool = self
            .definitions
            .get(&call.function.name)
            .ok_or_else(|| eyre!("unknown tool: {}", call.function.name))?;
        tool.execute(call.function.arguments.clone()).await
    }
}
