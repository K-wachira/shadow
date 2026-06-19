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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatToolCall {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default = "function_type")]
    pub r#type: String,
    pub function: ChatToolFunctionCall,
}

impl Default for ChatToolCall {
    fn default() -> Self {
        Self {
            id: None,
            r#type: function_type(),
            function: ChatToolFunctionCall::default(),
        }
    }
}

fn function_type() -> String {
    "function".into()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatToolFunctionCall {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

impl Default for ChatToolFunctionCall {
    fn default() -> Self {
        Self {
            name: String::new(),
            arguments: Value::Object(serde_json::Map::new()),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_tool() -> ToolDefinition {
        ToolDefinition::new(
            "test_tool",
            "A test tool",
            serde_json::json!({"type":"object","properties":{}}),
            |_args| Box::pin(async move { Ok("result".to_string()) }),
        )
    }

    #[test]
    fn tool_schema_has_correct_name() {
        let tool = make_test_tool();
        let schema = tool.schema();
        assert_eq!(schema.function.name, "test_tool");
    }

    #[test]
    fn tool_schema_has_correct_type() {
        let tool = make_test_tool();
        let schema = tool.schema();
        assert_eq!(schema.r#type, "function");
    }

    #[test]
    fn tool_schema_has_description() {
        let tool = make_test_tool();
        let schema = tool.schema();
        assert_eq!(schema.function.description, "A test tool");
    }

    #[test]
    fn tool_schema_has_parameters() {
        let tool = make_test_tool();
        let schema = tool.schema();
        assert!(schema.function.parameters.is_object());
    }

    #[test]
    fn tool_schema_serializes_correctly() {
        let tool = make_test_tool();
        let schema = tool.schema();
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("\"type\":\"function\""));
        assert!(json.contains("\"name\":\"test_tool\""));
    }

    #[test]
    fn chat_tool_call_default_has_function_type() {
        let call = ChatToolCall::default();
        assert_eq!(call.r#type, "function");
    }

    #[test]
    fn chat_tool_call_default_id_is_none() {
        let call = ChatToolCall::default();
        assert!(call.id.is_none());
    }

    #[test]
    fn chat_tool_call_default_function_has_empty_name() {
        let call = ChatToolCall::default();
        assert!(call.function.name.is_empty());
    }

    #[test]
    fn chat_tool_call_default_arguments_is_empty_object() {
        let call = ChatToolCall::default();
        assert!(call.function.arguments.is_object());
        assert!(call.function.arguments.as_object().unwrap().is_empty());
    }

    #[test]
    fn chat_tool_call_serializes_with_defaults() {
        let call = ChatToolCall::default();
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("\"type\":\"function\""));
    }

    #[test]
    fn chat_tool_call_serializes_with_id() {
        let call = ChatToolCall {
            id: Some("call-123".to_string()),
            r#type: "function".to_string(),
            function: ChatToolFunctionCall {
                name: "test".to_string(),
                arguments: serde_json::json!({"key":"val"}),
            },
        };
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("\"id\":\"call-123\""));
    }

    #[test]
    fn chat_tool_call_deserializes_from_json() {
        let json = r#"{"id":"call-1","type":"function","function":{"name":"weather","arguments":{"city":"London"}}}"#;
        let call: ChatToolCall = serde_json::from_str(json).unwrap();
        assert_eq!(call.id, Some("call-1".to_string()));
        assert_eq!(call.function.name, "weather");
        assert_eq!(call.function.arguments["city"], serde_json::json!("London"));
    }

    #[test]
    fn chat_tool_call_deserializes_without_id() {
        let json = r#"{"type":"function","function":{"name":"test","arguments":{}}}"#;
        let call: ChatToolCall = serde_json::from_str(json).unwrap();
        assert!(call.id.is_none());
        assert_eq!(call.function.name, "test");
    }

    #[tokio::test]
    async fn tool_definition_executes_handler() {
        let tool = ToolDefinition::new(
            "echo",
            "Echoes input",
            serde_json::json!({"type":"object","properties":{"msg":{"type":"string"}}}),
            |args| {
                let msg = args["msg"].as_str().unwrap_or("").to_string();
                Box::pin(async move { Ok(format!("echo: {}", msg)) })
            },
        );
        let result = tool
            .execute(serde_json::json!({"msg": "hello"}))
            .await
            .unwrap();
        assert_eq!(result, "echo: hello");
    }

    #[tokio::test]
    async fn tool_definition_handler_with_no_args() {
        let tool = ToolDefinition::new(
            "ping",
            "Pings",
            serde_json::json!({"type":"object"}),
            |_| Box::pin(async move { Ok("pong".to_string()) }),
        );
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert_eq!(result, "pong");
    }
}
