use crate::config::WireProtocol;
use linguafranca::anthropic::convert::stream::{
    AnthropicMessagesToOpenResponsesStream, OpenResponsesToAnthropicMessagesStream,
};
use linguafranca::anthropic::{request::AnthropicRequest, response::AnthropicResponse};
use linguafranca::chat_completions_openai::convert::stream::{
    ChatCompletionsToOpenResponsesStream, OpenResponsesToChatCompletionsStream,
};
use linguafranca::chat_completions_openai::{
    request::ChatCompletionsOpenAiRequest, response::ChatCompletionsOpenAiResponse,
};
use linguafranca::config::ConversionConfig;
use linguafranca::error::{ConversionError, ConversionResult};
use linguafranca::open_responses::{
    request::OpenResponsesRequest, response::OpenResponsesResponse,
};
use linguafranca::stream::dynamic::DynStreamTransform;
use linguafranca::traits::{FromOpenResponses, IntoOpenResponses};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

fn config() -> Option<ConversionConfig> {
    Some(ConversionConfig {
        strip_encrypted_reasoning: true,
    })
}

fn into_open<T>(value: Value) -> Result<ConversionResult<T::Target>, String>
where
    T: DeserializeOwned + IntoOpenResponses,
{
    let source: T = serde_json::from_value(value).map_err(|e| e.to_string())?;
    source
        .into_open_responses(config())
        .map_err(|e| e.to_string())
}

fn from_open<T>(open: ConversionResult<T::Source>) -> Result<Value, String>
where
    T: FromOpenResponses + Serialize,
{
    let mut result = T::from_open_responses(open.value, config()).map_err(|e| e.to_string())?;
    result.warnings.splice(0..0, open.warnings);
    serde_json::to_value(result.value).map_err(|e| e.to_string())
}

pub fn convert_request(
    value: Value,
    source: WireProtocol,
    target: WireProtocol,
) -> Result<Value, String> {
    if source == target {
        return Ok(value);
    }
    match (source, target) {
        (WireProtocol::OpenaiChat, WireProtocol::OpenaiResponses) => {
            serialize_result(into_open::<ChatCompletionsOpenAiRequest>(value)?)
        }
        (WireProtocol::OpenaiChat, WireProtocol::AnthropicMessages) => {
            from_open::<AnthropicRequest>(into_open::<ChatCompletionsOpenAiRequest>(value)?)
        }
        (WireProtocol::OpenaiResponses, WireProtocol::OpenaiChat) => {
            from_open::<ChatCompletionsOpenAiRequest>(ConversionResult {
                value: serde_json::from_value::<OpenResponsesRequest>(value)
                    .map_err(|e| e.to_string())?,
                warnings: vec![],
            })
        }
        (WireProtocol::OpenaiResponses, WireProtocol::AnthropicMessages) => {
            from_open::<AnthropicRequest>(ConversionResult {
                value: serde_json::from_value::<OpenResponsesRequest>(value)
                    .map_err(|e| e.to_string())?,
                warnings: vec![],
            })
        }
        (WireProtocol::AnthropicMessages, WireProtocol::OpenaiResponses) => {
            serialize_result(into_open::<AnthropicRequest>(value)?)
        }
        (WireProtocol::AnthropicMessages, WireProtocol::OpenaiChat) => {
            from_open::<ChatCompletionsOpenAiRequest>(into_open::<AnthropicRequest>(value)?)
        }
        _ => Ok(value),
    }
}

pub fn convert_response(
    value: Value,
    source: WireProtocol,
    target: WireProtocol,
) -> Result<Value, String> {
    if source == target {
        return Ok(value);
    }
    match (source, target) {
        (WireProtocol::OpenaiChat, WireProtocol::OpenaiResponses) => {
            serialize_result(into_open::<ChatCompletionsOpenAiResponse>(value)?)
        }
        (WireProtocol::OpenaiChat, WireProtocol::AnthropicMessages) => {
            from_open::<AnthropicResponse>(into_open::<ChatCompletionsOpenAiResponse>(value)?)
        }
        (WireProtocol::OpenaiResponses, WireProtocol::OpenaiChat) => {
            from_open::<ChatCompletionsOpenAiResponse>(ConversionResult {
                value: serde_json::from_value::<OpenResponsesResponse>(value)
                    .map_err(|e| e.to_string())?,
                warnings: vec![],
            })
        }
        (WireProtocol::OpenaiResponses, WireProtocol::AnthropicMessages) => {
            from_open::<AnthropicResponse>(ConversionResult {
                value: serde_json::from_value::<OpenResponsesResponse>(value)
                    .map_err(|e| e.to_string())?,
                warnings: vec![],
            })
        }
        (WireProtocol::AnthropicMessages, WireProtocol::OpenaiResponses) => {
            serialize_result(into_open::<AnthropicResponse>(value)?)
        }
        (WireProtocol::AnthropicMessages, WireProtocol::OpenaiChat) => {
            from_open::<ChatCompletionsOpenAiResponse>(into_open::<AnthropicResponse>(value)?)
        }
        _ => Ok(value),
    }
}

fn serialize_result<T: Serialize>(result: ConversionResult<T>) -> Result<Value, String> {
    serde_json::to_value(result.value).map_err(|e| e.to_string())
}

pub struct StreamBridge {
    stages: Vec<Box<dyn DynStreamTransform>>,
}

impl StreamBridge {
    pub fn new(source: WireProtocol, target: WireProtocol) -> Option<Self> {
        use WireProtocol::*;
        let stages: Vec<Box<dyn DynStreamTransform>> = match (source, target) {
            (OpenaiChat, OpenaiResponses) => {
                vec![Box::new(ChatCompletionsToOpenResponsesStream::new())]
            }
            (AnthropicMessages, OpenaiResponses) => {
                vec![Box::new(AnthropicMessagesToOpenResponsesStream::new())]
            }
            (OpenaiResponses, OpenaiChat) => {
                vec![Box::new(OpenResponsesToChatCompletionsStream::new())]
            }
            (OpenaiResponses, AnthropicMessages) => {
                vec![Box::new(OpenResponsesToAnthropicMessagesStream::new())]
            }
            (OpenaiChat, AnthropicMessages) => vec![
                Box::new(ChatCompletionsToOpenResponsesStream::new()),
                Box::new(OpenResponsesToAnthropicMessagesStream::new()),
            ],
            (AnthropicMessages, OpenaiChat) => vec![
                Box::new(AnthropicMessagesToOpenResponsesStream::new()),
                Box::new(OpenResponsesToChatCompletionsStream::new()),
            ],
            _ => return None,
        };
        Some(Self { stages })
    }

    pub fn transform(&mut self, value: Value) -> Result<Vec<Value>, ConversionError> {
        let mut values = vec![value];
        for stage in &mut self.stages {
            let mut next = Vec::new();
            for value in values {
                next.extend(stage.transform_value(value)?);
            }
            values = next;
        }
        Ok(values)
    }

    pub fn flush(&mut self) -> Result<Vec<Value>, ConversionError> {
        let mut output = Vec::new();
        for index in 0..self.stages.len() {
            let values = self.stages[index].flush_value()?;
            for value in values {
                let mut pending = vec![value];
                for stage in self.stages.iter_mut().skip(index + 1) {
                    let mut next = Vec::new();
                    for value in pending {
                        next.extend(stage.transform_value(value)?);
                    }
                    pending = next;
                }
                output.extend(pending);
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_request_converts_to_anthropic_with_tools() {
        let converted = convert_request(
            serde_json::json!({
                "model": "codex-chat",
                "input": [{"role": "user", "content": [{"type": "input_text", "text": "hello"}]}],
                "tools": [{"type": "function", "name": "echo", "description": "echo", "parameters": {"type": "object"}}],
                "max_output_tokens": 128
            }),
            WireProtocol::OpenaiResponses,
            WireProtocol::AnthropicMessages,
        )
        .unwrap();
        assert_eq!(converted["messages"][0]["role"], "user");
        assert_eq!(converted["tools"][0]["name"], "echo");
    }

    #[test]
    fn same_protocol_is_lossless() {
        let original = serde_json::json!({"model": "x", "vendor_extension": {"keep": true}});
        assert_eq!(
            convert_request(
                original.clone(),
                WireProtocol::AnthropicMessages,
                WireProtocol::AnthropicMessages
            )
            .unwrap(),
            original
        );
    }
}
