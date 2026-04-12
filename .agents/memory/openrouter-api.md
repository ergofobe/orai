<!-- agent-memory v1.0 -->

# OpenRouter API — orai

## Facts
- [2026-04-11] Default model: openrouter/free (rotates among free models)
- [2026-04-11] /api/v1/models endpoint returns supported_parameters per model
- [2026-04-11] Check if "tools" is in supported_parameters before sending tools for a model
- [2026-04-11] Native client tools: read(), write(), shell(), web_fetch()
- [2026-04-11] Server tools: openrouter:web_search, openrouter:datetime (enabled by default)

## Gotchas
- [2026-04-11] OpenRouter may return SSE chunks even for stream:false requests (especially free models)
- [2026-04-11] parse_sse_response() in client.rs handles this — always use it
- [2026-04-11] openrouter/free model alias rotates; some free models don't support tools
- [2026-04-11] Tool call deltas accumulate in stream.rs; must handle partial JSON