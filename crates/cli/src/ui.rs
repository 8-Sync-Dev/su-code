use owo_colors::OwoColorize;

pub fn ok(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

pub fn warn(msg: &str) {
    println!("{} {}", "!".yellow().bold(), msg.yellow());
}

pub fn err(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg.red());
}

pub fn info(msg: &str) {
    println!("{} {}", "·".blue().bold(), msg);
}

pub fn skip(name: &str, reason: &str) {
    println!(
        "{} {} {}",
        "↷".bright_black().bold(),
        name.bright_black(),
        format!("({})", reason).bright_black()
    );
}

pub fn step(msg: &str) {
    println!("\n{} {}", "▶".cyan().bold(), msg.bold());
}

pub fn header(title: &str) {
    println!(
        "\n{}\n{}\n",
        title.bold().cyan(),
        "─".repeat(title.len()).cyan()
    );
}

pub fn prompt_yes_no(question: &str, default: bool) -> bool {
    use std::io::{self, Write};
    let hint = if default { "[Y/n]" } else { "[y/N]" };
    print!("{} {} {} ", "?".magenta().bold(), question, hint);
    io::stdout().flush().ok();
    let mut buf = String::new();
    if io::stdin().read_line(&mut buf).is_err() {
        return default;
    }
    match buf.trim().to_lowercase().as_str() {
        "" => default,
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    }
}
