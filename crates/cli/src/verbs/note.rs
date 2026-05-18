use anyhow::Result;
use clap::Args as ClapArgs;
use std::io::Write;
use std::path::PathBuf;

use crate::ui;

#[derive(ClapArgs, Debug)]
#[command(
    after_help = indoc::indoc! {"
        EXAMPLES
          8sync note \"switched state lib to zustand\"
          8sync note --tag arch \"cache layer will use valkey\"
          8sync note --tag bug  \"login fails when password contains double-quote\"
          8sync note --tag idea \"add dark mode toggle in settings\"
          8sync note --tag todo \"write integration test for /api/login\"
          8sync note                                  open agents/NOTES.md in $EDITOR

        WHERE IT'S WRITTEN
          Notes are appended to <repo>/agents/NOTES.md with a timestamp + tag.
          AI tools (forge / claude-code / aider) read NOTES.md via AGENTS.md anchor
          on the next session, so use this to leave context for future-you.

        COMMON TAGS
          arch, bug, idea, todo, learn, perf, security
    "}
)]
pub struct Args {
    /// Note content. Omit to open agents/NOTES.md in $EDITOR.
    pub message: Vec<String>,

    /// Optional category tag: arch | bug | idea | todo | learn | perf | security | ...
    #[arg(long, short = 't', default_value = "")]
    pub tag: String,
}

pub fn run(a: Args) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_root(&cwd).unwrap_or(cwd);
    let agents_dir = root.join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    let notes = agents_dir.join("NOTES.md");
    if !notes.exists() {
        std::fs::write(&notes, "# NOTES (8sync managed — append-only)\n\n")?;
    }

    if a.message.is_empty() {
        // open in editor
        let editor = if which::which("hx").is_ok() {
            "hx"
        } else if which::which("helix").is_ok() {
            "helix"
        } else if let Ok(e) = std::env::var("EDITOR") {
            return open_editor(&e, &notes);
        } else {
            "vi"
        };
        return open_editor(editor, &notes);
    }

    let msg = a.message.join(" ");
    let tag = if a.tag.is_empty() { String::new() } else { format!("[{}] ", a.tag) };
    let ts = timestamp();
    let entry = format!("- {} {}{}\n", ts, tag, msg);

    let mut f = std::fs::OpenOptions::new().append(true).open(&notes)?;
    f.write_all(entry.as_bytes())?;
    ui::ok(&format!("appended → {}", notes.display()));
    Ok(())
}

fn open_editor(editor: &str, file: &PathBuf) -> Result<()> {
    std::process::Command::new(editor).arg(file).status()?;
    Ok(())
}

fn find_root(start: &std::path::Path) -> Option<PathBuf> {
    let markers = [".git", "Cargo.toml", "package.json", "pyproject.toml", "go.mod"];
    let mut p = start.to_path_buf();
    loop {
        for m in &markers {
            if p.join(m).exists() {
                return Some(p);
            }
        }
        if !p.pop() {
            return None;
        }
    }
}

fn timestamp() -> String {
    // ISO-ish without external crate
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("[{}]", secs)
}
