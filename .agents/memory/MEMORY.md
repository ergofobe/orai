<!-- agent-memory v1.0 -->

# Memory Index — orai

## Project Facts
- [2026-04-11] Rust CLI tool for OpenRouter AI models (prompt, chat, tui subcommands)
- [2026-04-11] Default model: openrouter/free
- [2026-04-11] Max agentic loop iterations: 25
- [2026-04-11] SSE responses may come from non-streaming requests — handled in parse_sse_response()
- [2026-04-11] Uses tokio + reqwest with rustls-tls (no OpenSSL dependency)
- [2026-04-11] Uses ratatui + crossterm for TUI backend
- [2026-04-11] PDF conversion: pdftoppm → ImageMagick → lopdf text extraction fallback chain
- [2026-04-11] Repo: ~/src/orai, GitHub: ergofobe/orai
- [2026-04-11] Current version: 0.2.0
- See `build-and-release.md` for CI, release process, and distribution
- See `openrouter-api.md` for API patterns and gotchas

## Gotchas
- [2026-04-11] macOS sed -i requires '' arg (BSD vs GNU) — handled in release.yml
- [2026-04-11] GITHUB_TOKEN can't push to other repos — need HOMEBREW_TAP_TOKEN PAT (not yet set)
- [2026-04-11] cross tool fails for Android targets (missing libunwind) — use cargo-ndk instead
- [2026-04-11] Android target is aarch64-linux-android, NOT aarch64-unknown-linux-musl (gives "unexpected e_type")
- [2026-04-11] Homebrew formula must be updated manually until HOMEBREW_TAP_TOKEN is set in CI
- [2026-04-11] Tool safety: shell() and write() require confirmation unless -y flag is set