from litellm.responses.litellm_completion_transformation.transformation import (
    LiteLLMCompletionResponsesConfig,
)


def main() -> None:
    input_items = [
        {"type": "message", "role": "user", "content": "Run pwd"},
        {
            "type": "function_call",
            "call_id": "call_01",
            "name": "exec_command",
            "arguments": '{"cmd":"pwd"}',
        },
        {
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "Inspecting."}],
        },
        {
            "type": "message",
            "role": "assistant",
            "content": [{"type": "output_text", "text": "This should be quick."}],
        },
        {
            "type": "function_call_output",
            "call_id": "call_01",
            "output": "/workspace",
        },
    ]

    messages = LiteLLMCompletionResponsesConfig._transform_response_input_param_to_chat_completion_message(
        input=input_items
    )
    assert [message.get("role") for message in messages] == ["user", "assistant", "tool"]
    assert messages[1].get("tool_calls")[0].get("id") == "call_01"
    assert messages[2].get("tool_call_id") == "call_01"
    assert messages[1].get("content") == [
        {"type": "text", "text": "Inspecting."},
        {"type": "text", "text": "This should be quick."},
    ]
    print("TOOL_ADJACENCY_OK")


if __name__ == "__main__":
    main()
