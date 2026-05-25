use owo_colors::OwoColorize;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Global tee target. When set (via `set_log_file`), every `ui::*` call also
/// writes a stripped (ANSI-free) line into the file. Used by `--yall` mode in
/// `8sync setup` to produce a track-able install log.
static LOG_FILE: OnceLock<Mutex<Option<File>>> = OnceLock::new();

/// Open `path` for append; route all subsequent ui::* messages there too.
/// Returns the resolved path so callers can echo it.
pub fn set_log_file(path: PathBuf) -> std::io::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    LOG_FILE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map(|mut g| *g = Some(f))
        .ok();
    Ok(path)
}

/// Flush + close the log file. Idempotent.
pub fn close_log_file() {
    if let Some(m) = LOG_FILE.get() {
        if let Ok(mut g) = m.lock() {
            if let Some(f) = g.as_mut() {
                let _ = f.flush();
            }
            *g = None;
        }
    }
}

fn log_line(prefix: &str, msg: &str) {
    if let Some(m) = LOG_FILE.get() {
        if let Ok(mut g) = m.lock() {
            if let Some(f) = g.as_mut() {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = writeln!(f, "[{ts}] {prefix} {msg}");
                let _ = f.flush();
            }
        }
    }
}

pub fn ok(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
    log_line("OK", msg);
}

pub fn warn(msg: &str) {
    println!("{} {}", "!".yellow().bold(), msg.yellow());
    log_line("WARN", msg);
}

pub fn err(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg.red());
    log_line("ERR", msg);
}

pub fn info(msg: &str) {
    println!("{} {}", "·".blue().bold(), msg);
    log_line("INFO", msg);
}

pub fn skip(name: &str, reason: &str) {
    println!(
        "{} {} {}",
        "↷".bright_black().bold(),
        name.bright_black(),
        format!("({})", reason).bright_black()
    );
    log_line("SKIP", &format!("{name} ({reason})"));
}

pub fn step(msg: &str) {
    println!("\n{} {}", "▶".cyan().bold(), msg.bold());
    log_line("STEP", msg);
}

pub fn header(title: &str) {
    println!(
        "\n{}\n{}\n",
        title.bold().cyan(),
        "─".repeat(title.len()).cyan()
    );
    log_line("===", title);
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
