# Stream Events Reference

This file lists all Responses streaming events currently handled by `ox`.

## Handled Events

1. `response.output_text.delta`

- Behavior: emits `CoreEvent::AgentTextDelta`.
- Representative full event:

```json
{
  "type": "response.output_text.delta",
  "response_id": "resp_123",
  "output_index": 0,
  "item_id": "msg_1",
  "content_index": 0,
  "delta": "Hello"
}
```

- Fields used by `ox`: `delta`.

2. `response.output_item.done`

- Behavior:
  - `item.type == "message"`: appends raw `item` to history.
  - `item.type == "reasoning"`: appends raw `item` to history.
  - `item.type == "function_call"`:
    - emits `CoreEvent::AgentToolCallStart`.
    - appends raw `item`.
    - executes tool.
    - appends `function_call_output`.
    - emits `CoreEvent::AgentToolCallEnd`.
- Representative full events:

```json
{
  "type": "response.output_item.done",
  "response_id": "resp_123",
  "output_index": 0,
  "item": {
    "type": "message",
    "id": "msg_1",
    "role": "assistant",
    "status": "completed",
    "content": [
      { "type": "output_text", "text": "Part A" },
      { "type": "output_text", "text": " + Part B" }
    ]
  }
}
```

```json
{
  "type": "response.output_item.done",
  "response_id": "resp_123",
  "output_index": 0,
  "item": {
    "id": "rs_1",
    "type": "reasoning",
    "status": "completed",
    "summary": [{ "type": "summary_text", "text": "thinking..." }]
  }
}
```

```json
{
  "type": "response.output_item.done",
  "response_id": "resp_123",
  "output_index": 0,
  "item": {
    "type": "function_call",
    "id": "fc_123",
    "status": "completed",
    "call_id": "call_123",
    "name": "read_file",
    "arguments": "{\"path\":\"README.md\"}"
  }
}
```

- Fields used by `ox`: full `item` for history, plus function-call `call_id`, `name`, `arguments`.

3. `response.completed`

- Behavior: marks stream as completed.
- Representative full event:

```json
{
  "type": "response.completed",
  "response": {
    "id": "resp_123",
    "object": "response",
    "created_at": 1736200000,
    "status": "completed",
    "error": null,
    "incomplete_details": null,
    "model": "gpt-5.1",
    "usage": {
      "input_tokens": 100,
      "input_tokens_details": { "cached_tokens": 20 },
      "output_tokens": 50,
      "output_tokens_details": { "reasoning_tokens": 10 },
      "total_tokens": 150
    }
  }
}
```

- Fields used by `ox`: `response.status` (completion/failure safety).

4. `response.done`

- Behavior: treated as completion alias (same as `response.completed`).
- Representative full event:

```json
{
  "type": "response.done",
  "response": {
    "id": "resp_123",
    "status": "completed"
  }
}
```

- Fields used by `ox`: `response.status` (same handling as `response.completed`).

5. `response.failed`

- Behavior: records failure message from payload.
- Representative full event:

```json
{
  "type": "response.failed",
  "response": {
    "id": "resp_123",
    "status": "failed",
    "error": {
      "type": "invalid_request_error",
      "code": "rate_limit_exceeded",
      "message": "try again later"
    }
  }
}
```

- Fields used by `ox`: `response.error.message`, `response.error.code`, `response.status`.

6. `error`

- Behavior: records failure message from `code`/`message`.
- Representative full event:

```json
{
  "type": "error",
  "event_id": "event_123",
  "code": "server_error",
  "message": "internal error"
}
```

- Fields used by `ox`: `code`, `message`.

## Final Stream Outcome Checks

1. If any failure was recorded (`response.failed` or `error`), return an error.
2. If stream ended without completion (`response.completed` or `response.done`), return:
   - `stream closed before response.completed`
3. Otherwise continue normal loop behavior based on whether tool calls were produced.

## Catch-All

Any unrecognized event is mapped to:

- `StreamEvent::Ignored`
- No-op behavior
