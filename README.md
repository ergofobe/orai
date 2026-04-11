# orai

CLI tool for OpenRouter AI models. Chat with any model, use native tools, attach files — all from your terminal.

## Install

```bash
curl -fsSL https://ergofobe.github.io/orai/install.sh | sh
```

**macOS with Homebrew:**

```bash
brew install ergofobe/orai/orai
```

**Options:**

| Flag | Description |
|------|-------------|
| `--glibc` | Use glibc binary instead of musl (Linux) |
| `--force` | Install even if already up to date |
| `--dry-run` | Show what would be installed, don't install |

## Requirements

- `OPENROUTER_API_KEY` environment variable (get one at [openrouter.ai](https://openrouter.ai))

## Usage

```bash
export OPENROUTER_API_KEY=your-key-here

# One-shot prompt (default model: openrouter/free)
orai prompt "Explain Rust's borrow checker"

# Use a specific model
orai -m anthropic/claude-3.5-sonnet prompt "Review this code"

# Attach files
orai -a diagram.png prompt "What does this diagram show?"
orai -a report.pdf prompt "Summarize this report"

# Interactive chat (simple REPL)
orai chat

# Full TUI with markdown rendering
orai tui

# Auto-approve all tool calls (dangerous)
orai -y prompt "Fix the bugs in src/main.rs"

# Disable server tools
orai --no-web-search --no-datetime prompt "What time is it?"

# Disable native tools
orai --no-native-tools prompt "Tell me a joke"
```

## Subcommands

| Command | Description |
|---------|-------------|
| `prompt` | One-shot: send a prompt, get a response, exit |
| `chat` | Interactive REPL conversation |
| `tui` | Full terminal UI with markdown rendering |

## Global Options

| Flag | Default | Description |
|------|---------|-------------|
| `-m, --model` | `openrouter/free` | Model to use |
| `-a, --attach` | — | Attach file(s), repeatable |
| `-y, --yes` | false | Auto-approve all tool confirmations |
| `--no-web-search` | false | Disable web_search server tool |
| `--no-datetime` | false | Disable datetime server tool |
| `--search-engine` | `auto` | Search engine: auto, native, exa, firecrawl, parallel |
| `--max-search-results` | 5 | Max search results per query |
| `--no-native-tools` | false | Disable read/write/shell/web_fetch tools |
| `--shell-timeout` | 30 | Shell command timeout in seconds |

## Tools

### Native Client Tools

The model can call these tools on your machine:

| Tool | Description | Confirmation |
|------|-------------|-------------|
| `read(path)` | Read a file from disk | No |
| `write(path, content)` | Write content to a file | **Yes** |
| `shell(command)` | Execute a shell command | **Yes** |
| `web_fetch(url)` | Fetch content from a URL | No |

`write` and `shell` require confirmation unless `-y` is set. In the TUI, you can approve individual calls or approve all remaining calls.

### OpenRouter Server Tools

Enabled by default, executed server-side by OpenRouter:

| Tool | Description |
|------|-------------|
| `openrouter:web_search` | Search the web for current information |
| `openrouter:datetime` | Get the current date and time |

## File Attachments

Attach images, PDFs, and text files:

```bash
# Via -a flag
orai -a photo.png prompt "Describe this image"
orai -a doc.pdf prompt "Summarize this document"

# Via +filename syntax in chat/tui
You: Explain this +diagram.png and +notes.md
```

**PDF handling:** PDFs are automatically converted to images (using `pdftoppm` or ImageMagick). If neither is available, text is extracted with `lopdf`.

## Versioning

- `0.x.0` — new features release (bump minor)
- `0.x.1+` — bug fix release (bump patch)

## License

MIT