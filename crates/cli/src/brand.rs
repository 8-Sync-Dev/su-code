//! Single source of truth for the CLI's identity. Edit the two literals below
//! (or set `SC_CMD` / `SC_NS` at build time) to rebrand the whole CLI in one
//! place — the invoked command name, help text, on-disk config/cache namespace,
//! AGENTS.md sentinels, and deployed-artifact filenames all follow from here.
//!
//! Default `CMD == NS == "8sync"` ⇒ every derived string is identical to the
//! historical hardcoding, so the default build + deploy are byte-for-byte
//! unchanged (safe cutover). The upstream identity `8-Sync-Dev` and the
//! `github.com` / `raw.githubusercontent.com` self-update URLs are NEVER
//! rebranded (renaming them would break `up`/install) — and since the literal
//! lowercase token `8sync` never appears inside `8-Sync-Dev` or those URLs,
//! `render` leaves them untouched automatically.
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// The invoked command name — clap `name`/`bin_name`, every help/EXAMPLES block,
/// terminal prose, error hints. Compile-time `&'static str` (clap needs it).
pub const CMD: &str = match option_env!("SC_CMD") {
    Some(v) => v,
    None => "8sync",
};

/// The on-disk namespace — `~/.config/<NS>`, `kitty/<NS>.conf`, AGENTS.md
/// `<NS>:skills` sentinels, and deployed artifact filenames (`<NS>-engine.ts`,
/// `<NS>-recall.ts`, `<NS>-harness-up` …). Kept beside `CMD` so a rename makes
/// the binary name and its persistent state paths agree from a single edit.
/// (`~/.cache/` is deliberately excluded — see the note below.)
pub const NS: &str = match option_env!("SC_NS") {
    Some(v) => v,
    None => "8sync",
};

/// The legacy sentinels an older (`8sync`-namespaced) binary wrote into AGENTS.md.
/// Parsers accept BOTH these and the current [`sentinel_begin`]/[`sentinel_end`]
/// so a rebranded binary can recognise + migrate existing project files.
pub const LEGACY_SENTINEL_BEGIN: &str = "<!-- 8sync:skills:begin -->";
pub const LEGACY_SENTINEL_END: &str = "<!-- 8sync:skills:end -->";

/// `<home>/.config/<NS>` — the user-config namespace dir.
pub fn config_dir(home: &Path) -> PathBuf {
    home.join(".config").join(NS)
}

// NOTE: the `~/.cache/` namespace intentionally stays literal `8sync` (NOT
// rebranded): it is derived/gitignored throwaway state, and the verbatim-deployed
// `8sync-gs` extension hard-codes `.cache/8sync/gs/` (rendering `.ts` code is out
// of scope). Keeping it literal avoids a split namespace and keeps the GS engine
// ⇄ dashboard state coupling intact.

/// A deployed-artifact filename/stem: `<NS>-<suffix>` (e.g. `ns_file("workflow.ts")`
/// → `8sync-workflow.ts` by default). One definition so the deploy target and the
/// migration shim that removes the old name agree.
pub fn ns_file(suffix: &str) -> String {
    format!("{NS}-{suffix}")
}

/// AGENTS.md skills-block opening sentinel — shared by the writer (`skill::inject`)
/// and every parser (`audit`, `doctor`) so the marker is defined once.
pub fn sentinel_begin() -> String {
    format!("<!-- {NS}:skills:begin -->")
}

/// AGENTS.md skills-block closing sentinel — pair of [`sentinel_begin`].
pub fn sentinel_end() -> String {
    format!("<!-- {NS}:skills:end -->")
}

/// Rebrand command references inside DEPLOYED text/markdown prose (bundled skills,
/// force-load, APPEND_SYSTEM, AGENTS templates, command `.md`) so the deployed
/// guidance speaks the active command name. Identity (borrowed, zero-copy) when
/// the defaults are in effect — the byte-identical safety gate.
///
/// Ordered, conservative staged replacement (order matters): `8sync-<w>` →
/// `<NS>-<w>` (filenames) · `8sync:` → `<NS>:` (sentinels) · `8sync/` → `<NS>/`
/// (paths) · `8sync.` → `<NS>.` (e.g. `8sync.conf`) · remaining bare `8sync` →
/// `<CMD>` (command tokens). `8-Sync-Dev` and http(s) URLs contain no lowercase
/// `8sync` token, so they are never rewritten. The bundled skill directory
/// `8sync-cli` is a FIXED identifier (not the on-disk namespace) and is
/// protected so its path/name survives. NEVER call this on `.ts`/binary
/// assets — only on human-readable prose.
pub fn render(s: &str) -> Cow<'_, str> {
    if CMD == "8sync" && NS == "8sync" {
        return Cow::Borrowed(s);
    }
    // Guard fixed identifiers the staged rules would otherwise rewrite (NUL
    // never occurs in these text assets): the bundled `8sync-cli` skill dir, and
    // the `.cache/8sync` namespace which deliberately stays literal (see the NS
    // note above — the verbatim engine `.ts` couples to it).
    const GUARD_CLI: &str = "\u{0}cli\u{0}";
    const GUARD_CACHE: &str = "\u{0}cache\u{0}";
    let out = s
        .replace("8sync-cli", GUARD_CLI)
        .replace("cache/8sync", GUARD_CACHE)
        .replace("8sync-", &format!("{NS}-"))
        .replace("8sync:", &format!("{NS}:"))
        .replace("8sync/", &format!("{NS}/"))
        .replace("8sync.", &format!("{NS}."))
        .replace("8sync", CMD)
        .replace(GUARD_CLI, "8sync-cli")
        .replace(GUARD_CACHE, "cache/8sync");
    Cow::Owned(out)
}
