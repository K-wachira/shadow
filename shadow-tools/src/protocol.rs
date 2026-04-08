use futures::future::BoxFuture;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Serialize, Clone, Debug)]
pub struct ChatTool {
    pub r#type: &'static str,
    pub function: ToolFunctionSchema,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolFunctionSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatToolCall {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "function_type")]
    pub r#type: String,
    pub function: ChatToolFunctionCall,
}

fn function_type() -> String {
    "function".into()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChatToolFunctionCall {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

type ToolHandler =
    Arc<dyn Fn(Value) -> BoxFuture<'static, color_eyre::Result<String>> + Send + Sync>;

#[derive(Clone)]
pub struct ToolDefinition {
    schema: ChatTool,
    handler: ToolHandler,
}

impl ToolDefinition {
    pub fn new<F, Fut>(
        name: impl Into<String>, description: impl Into<String>, parameters: Value, handler: F,
    ) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = color_eyre::Result<String>> + Send + 'static,
    {
        let name = name.into();
        Self {
            schema: ChatTool {
                r#type: "function",
                function: ToolFunctionSchema {
                    name,
                    description: description.into(),
                    parameters,
                },
            },
            handler: Arc::new(move |args| Box::pin(handler(args))),
        }
    }

    pub fn schema(&self) -> ChatTool {
        self.schema.clone()
    }

    pub(crate) async fn execute(&self, args: Value) -> color_eyre::Result<String> {
        (self.handler)(args).await
    }
}
