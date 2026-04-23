use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "orai", version, about = "CLI tool for OpenRouter AI models")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, default_value = "openrouter/free", global = true)]
    pub model: String,

    #[arg(short, long, global = true)]
    pub attach: Vec<String>,

    #[arg(
        short = 'y',
        long,
        global = true,
        help = "Auto-approve all tool confirmations"
    )]
    pub yes: bool,

    #[arg(long, global = true, help = "Disable web_search server tool")]
    pub no_web_search: bool,

    #[arg(long, global = true, help = "Disable datetime server tool")]
    pub no_datetime: bool,

    #[arg(
        long,
        global = true,
        default_value = "auto",
        help = "Search engine: auto, native, exa, firecrawl, parallel"
    )]
    pub search_engine: String,

    #[arg(
        long,
        global = true,
        default_value_t = 5,
        help = "Max search results per query"
    )]
    pub max_search_results: u32,

    #[arg(
        long,
        global = true,
        help = "Disable native tools (read, write, shell, web_fetch)"
    )]
    pub no_native_tools: bool,

    #[arg(
        long,
        global = true,
        default_value_t = 30,
        help = "Shell command timeout in seconds"
    )]
    pub shell_timeout: u64,
}

#[derive(Subcommand)]
pub enum Commands {
    Prompt {
        #[arg(trailing_var_arg = true, required = true)]
        prompt: Vec<String>,

        #[arg(long, help = "Disable streaming")]
        no_stream: bool,

        #[arg(long, help = "System message to inject before the user message")]
        system: Option<String>,
    },
    Chat,
    Tui,
}
