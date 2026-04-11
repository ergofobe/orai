# AGENTS.md

## Project Overview

orai is a Rust CLI tool for interacting with OpenRouter AI models from the command line. It supports one-shot prompts, REPL chat, and a full TUI interface with streaming, tool calling, file attachments, and markdown rendering.

## Build & Run

```bash
cargo build                # Debug build
cargo build --release      # Release build
cargo run -- <subcommand>  # Run with args
cargo test                 # Run tests
cargo clippy -- -D warnings # Lint
cargo fmt --check          # Check formatting
```

## Architecture

```
src/
├── main.rs           # Entry point, env validation, subcommand dispatch
├── cli.rs            # Clap derive CLI definitions (prompt/chat/tui subcommands)
├── client.rs         # OpenRouter API client (streaming + non-streaming)
├── stream.rs         # SSE line parsing, tool call delta accumulation
├── tools/
│   ├── mod.rs        # Tool registry, OpenAI function definitions, agentic loop
│   ├── native.rs     # Client-side tools: read, write, shell, web_fetch
│   ├── server.rs     # OpenRouter server tool configuration (web_search, datetime)
│   └── confirm.rs   # Confirmation prompts for dangerous operations
├── attachment.rs     # File loading, type detection, PDF-to-image conversion
├── prompt.rs         # One-shot prompt subcommand
├── chat.rs           # Interactive REPL chat subcommand
├── markdown.rs       # pulldown-cmark → ratatui styled spans converter
└── tui/
    ├── mod.rs        # TUI app state, event loop, agentic integration
    ├── render.rs     # ratatui layout and rendering
    └── input.rs      # Textarea widget, key bindings, +filename parsing
```

## Key Design Decisions

- **Async runtime:** tokio
- **HTTP:** reqwest with rustls-tls (no OpenSSL dependency)
- **TUI:** ratatui + crossterm backend
- **Streaming:** SSE parser accumulates tool_call deltas; agentic loop re-requests on `finish_reason: "tool_calls"`
- **Tool safety:** `shell()` and `write()` require user confirmation unless `-y` flag is set
- **PDF conversion:** pdftoppm → ImageMagick → lopdf text extraction fallback chain
- **Default model:** `openrouter/free`
- **Max agentic loop iterations:** 25
- **Versioning:** 0.x.0 for features, 0.x.1+ for bug fixes

## Environment

- `OPENROUTER_API_KEY` — Required. API key for OpenRouter.

## Conventions

- Error handling: `anyhow` for application errors
- No `unwrap()` in production code — use `?` or explicit error handling
- All user-facing output uses styled display functions
- Tool results are JSON-serialized strings in OpenRouter tool message format
- Tests use `#[cfg(test)]` modules in the same file

## Distribution

- **Linux:** musl (universal) + glibc binaries via GitHub Releases
- **Android/Termux:** `aarch64-linux-android` + `x86_64-linux-android` binaries
- **macOS:** Homebrew tap (`ergofobe/orai/orai`) + direct binary download
- **Install script:** `https://ergofobe.github.io/orai/install.sh`
- **GitHub Actions:** `.github/workflows/release.yml` builds on tag push, `.github/workflows/pages.yml` deploys landing page

## Repositories

- **orai:** `~/src/orai` → https://github.com/ergofobe/orai
- **homebrew-orai:** `~/src/homebrew-orai` → https://github.com/ergofobe/homebrew-orai
  - Always use `~/src/homebrew-orai` (NOT `/tmp`) when updating the Homebrew formula
  - Update Formula/orai.rb with version + SHA256 after each release

## Release Process

1. Bump version in `Cargo.toml`
2. Commit and push to main
3. Tag: `git tag v0.x.y && git push origin v0.x.y`
4. Wait for CI to build all 8 targets and create GitHub Release
5. Download macOS binary, compute SHA256, update `~/src/homebrew-orai/Formula/orai.rb`
6. Commit and push homebrew-orai

## Known Issues

- OpenRouter may return SSE chunks even for `stream:false` requests (especially free models). `parse_sse_response()` handles this.
- `HOMEBREW_TAP_TOKEN` secret not set in GitHub — Homebrew formula must be updated manually from `~/src/homebrew-orai`
- macOS `sed -i` requires `''` arg (BSD sed) — handled in release.yml