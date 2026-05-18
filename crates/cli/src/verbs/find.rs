use anyhow::Result;
use clap::Args as ClapArgs;
use std::process::Command;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync find auth                       grep + fzf preview, mở match bằng helix
          8sync find 'fn main'                  pattern có khoảng trắng
          8sync find --files login              chỉ tìm filename (fd)
          8sync find --type rs Result           giới hạn theo loại file
    "}
)]
pub struct Args {
    /// Keyword / pattern (regex)
    pub query: Vec<String>,

    /// Chỉ tìm tên file (fd), không phải nội dung
    #[arg(long, short = 'f')]
    pub files: bool,

    /// Lọc theo loại file (rs / ts / py / ...)
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// Chỉ in kết quả, không mở editor
    #[arg(long)]
    pub no_open: bool,
}

pub fn run(a: Args) -> Result<()> {
    if a.query.is_empty() {
        ui::warn("usage: 8sync find <keyword>  (try -h)");
        return Ok(());
    }
    let q = a.query.join(" ");

    if a.files {
        let mut fd = Command::new("fd");
        if let Some(t) = &a.r#type {
            fd.args(["-e", t]);
        }
        fd.arg(&q);
        let out = fd.output()?;
        let s = String::from_utf8_lossy(&out.stdout);
        if s.trim().is_empty() {
            ui::info(&format!("no file matched `{}`", q));
            return Ok(());
        }
        print!("{}", s);
        if !a.no_open {
            pipe_to_fzf_then_open(&s)?;
        }
        return Ok(());
    }

    // content search via rg
    let mut rg = Command::new("rg");
    rg.args(["-n", "--column", "--no-heading", "--color=never", "--smart-case"]);
    if let Some(t) = &a.r#type {
        rg.args(["-t", &normalize_rg_type(t)]);
    }
    rg.arg(&q);
    let out = rg.output()?;
    let s = String::from_utf8_lossy(&out.stdout);
    if s.trim().is_empty() {
        ui::info(&format!("no match for `{}`", q));
        return Ok(());
    }
    print!("{}", s);

    if a.no_open {
        return Ok(());
    }

    // pipe to fzf for interactive selection
    pipe_to_fzf_then_open(&s)?;
    Ok(())
}

/// Map common extensions to rg's --type names (rs → rust, ts → typescript, ...)
fn normalize_rg_type(t: &str) -> String {
    match t {
        "rs"  => "rust",
        "ts"  => "typescript",
        "tsx" => "typescript",
        "js"  => "js",
        "jsx" => "js",
        "py"  => "py",
        "go"  => "go",
        "md"  => "md",
        other => other,
    }.to_string()
}

fn pipe_to_fzf_then_open(stdout: &str) -> Result<()> {
    if which::which("fzf").is_err() {
        return Ok(());
    }
    use std::io::Write;
    let mut child = Command::new("fzf")
        .args([
            "--ansi",
            "--prompt=8sync find > ",
            "--height=50%",
            "--reverse",
            "--delimiter=:",
            "--preview", "bat --color=always --highlight-line {2} {1} 2>/dev/null || sed -n {2}p {1}",
            "--preview-window", "right:60%:wrap",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdout.as_bytes());
    }
    let out = child.wait_with_output()?;
    let pick = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if pick.is_empty() {
        return Ok(());
    }
    // pick = "path:line:col:text"
    let parts: Vec<&str> = pick.splitn(4, ':').collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let file = parts[0];
    let line = parts[1];
    let editor = pick_editor();
    let target = format!("{}:{}", file, line);
    Command::new(editor).arg(&target).status()?;
    Ok(())
}

/// Honor `$EDITOR` / `$VISUAL` first; fall back to helix/hx/vi.
fn pick_editor() -> String {
    if let Ok(e) = std::env::var("VISUAL") {
        if !e.is_empty() && which::which(&e).is_ok() { return e; }
    }
    if let Ok(e) = std::env::var("EDITOR") {
        if !e.is_empty() && which::which(&e).is_ok() { return e; }
    }
    if which::which("hx").is_ok()    { return "hx".to_string(); }
    if which::which("helix").is_ok() { return "helix".to_string(); }
    "vi".to_string()
}
