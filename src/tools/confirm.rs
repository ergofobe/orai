use std::io::{self, Write};

pub fn confirm_prompt(tool_name: &str, arguments: &str) -> bool {
    eprint!(
        "\n⚙ {}({})\nAllow? [y/N] ",
        tool_name,
        truncate_args(arguments)
    );
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

pub fn confirm_chat(tool_name: &str, arguments: &str) -> bool {
    confirm_prompt(tool_name, arguments)
}

pub fn confirm_tui(tool_name: &str, arguments: &str) -> bool {
    eprintln!("\n⚙ Tool call: {}({})", tool_name, truncate_args(arguments));
    eprint!("Allow? [y/N/a(ll)] ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    matches!(
        input.trim().to_lowercase().as_str(),
        "y" | "yes" | "a" | "all"
    )
}

fn truncate_args(args: &str) -> String {
    if args.len() > 100 {
        format!("{}...", &args[..100])
    } else {
        args.to_string()
    }
}
