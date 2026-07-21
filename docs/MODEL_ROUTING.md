# Model and protocol routing

Each logical model contains one or more upstream profiles in `.gateway/models.json`. A profile declares its base URL, upstream model name, native wire protocol, authentication mode, weight, and enabled state.

Supported wire protocols:

- OpenAI Responses
- OpenAI Chat Completions
- Anthropic Messages

## Request path

1. The incoming route determines the client protocol.
2. The gateway selects an enabled profile for the requested logical model using session affinity and profile weights.
3. If the selected upstream uses the same protocol, the gateway forwards the payload and stream directly after applying routing fields.
4. If protocols differ, the reusable Rust conversion layer translates the request, response, and SSE events.
5. Network errors, rate limits, and upstream server errors can fail over to another profile in the same model pool.

This allows a native Anthropic provider to serve Claude Code without conversion while the same logical model remains available to Codex through Responses. Providers that only support Chat Completions remain usable through cross-protocol conversion.

Routing telemetry contains model/profile identifiers, status, latency, and retry information only. Request content and API keys are never recorded.
