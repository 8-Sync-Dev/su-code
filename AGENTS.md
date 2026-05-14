# AGENTS.md — Hướng dẫn cho AI agent làm việc với `su-code`

> File này dành cho AI tool (forge, claude-code, cursor, opencode, aider, …)
> Đây là **install + usage guide** dạng deterministic — đọc xong là biết phải làm gì.

---

## 1. Repo này là gì

`su-code` = source của binary **`8sync`** — coding harness CLI cho:
- **CachyOS / Arch Linux** (target chính, hỗ trợ Manjaro/EndeavourOS)
- **Kitty terminal** (cần `allow_remote_control yes`)
- **Helix editor** (`hx` hoặc `helix`)
- **Forge AI engine** (forgecode.dev)

Stack: **Rust** (single workspace, 1 binary `8sync` ≈ 1.3 MB stripped).

---

## 2. Cài đặt cho user

```bash
git clone https://github.com/8-Sync-Dev/su-code.git
cd su-code
bash scripts/bootstrap.sh        # cài rustup + build + install vào ~/.local/bin
```

Sau đó:
```bash
8sync setup                      # cài full môi trường (idempotent)
# đóng & mở lại kitty 1 lần      # bật allow_remote_control
forge login                      # paste API key của forge
8sync doctor                     # verify
```

**Quy tắc**: KHÔNG chạy `8sync setup` mà chưa hỏi user trước — nó sẽ `sudo pacman -S` nhiều package. Luôn chạy `8sync setup --dry-run` trước.

---

## 3. Build từ source (cho dev đóng góp)

```bash
cargo build --release            # cần Rust >= 1.85 (workspace lock 1.95 stable)
./target/release/8sync --version
# binary ở target/release/8sync
```

Phụ thuộc Rust crates: `clap`, `anyhow`, `owo-colors`, `dirs`, `serde`, `toml`, `semver`, `which`, `indoc`, `rust-embed`, `serde_json`, `urlencoding`. **Không có `reqwest`** — HTTP qua `curl` shell-out để giữ binary nhỏ.

---

## 4. Cấu trúc dự án

```
su-code/
├── Cargo.toml · rust-toolchain.toml · .cargo/config.toml    workspace + LTO=fat
├── README.md · AGENTS.md (file này) · LICENSE
├── scripts/bootstrap.sh                                     installer (rustup → cargo build → install)
├── crates/cli/                                              binary `8sync`
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                                          clap subcommand router
│       ├── ui.rs                                            colored print helpers
│       ├── env_detect.rs                                    OS/terminal/tool detection
│       ├── pkg.rs                                           pacman/paru idempotent install
│       ├── assets.rs                                        embedded asset reader (rust-embed)
│       └── verbs/                                           1 module mỗi verb
│           ├── root.rs · flow.rs · setup.rs · doctor.rs · up.rs
│           ├── here.rs (`8sync .` + sub: ls/to/new/rm/mv/wipe/kick)
│           ├── ai.rs · end.rs · ship.rs · run.rs
│           ├── bg.rs (Wallhaven/yandere/safebooru + opacity + tint + rotate)
│           ├── look.rs (5 presets: neon/ice/mint/dark/dim)
│           ├── find.rs (rg+fzf+helix) · note.rs (agents/NOTES.md)
│           ├── skill.rs · shot.rs · diff_img.rs · pdf_img.rs
│           └── mcp.rs (stub)
└── assets/                                                  bundled vào binary qua rust-embed
    ├── configs/                                             kitty.conf, helix-config.toml, fish-config.fish, ...
    ├── presets/                                             5 kitty preset .conf (neon_glass, ice_glass, ...)
    ├── skills/                                              karpathy, image-routing, 8sync-cli + 00-force-load.md
    └── wallpapers/wallpapers.toml                           URL list cho default wallpaper
```

---

## 5. Toàn bộ verb (20 verb flat, không sub-sub)

### Vibe loop (daily, dùng liên tục)
| Verb | Mô tả |
|---|---|
| `8sync .` | Mở/attach session: kitty 3-pane + forge trong abduco |
| `8sync ai [prompt]` | AI session (resume hoặc one-shot, wrap forge) |
| `8sync find <kw>` | rg/fd + fzf preview → enter mở helix tại `file:line` |
| `8sync note "msg" [-t tag]` | Append `agents/NOTES.md` |
| `8sync run [dev\|build\|test\|fmt\|lint]` | Project command theo recipe |
| `8sync ship "msg"` | `git add -A && commit && push && gh pr create` |
| `8sync end` | AI capture knowledge → `agents/*.md` |

### Session mgmt (sub của `.`)
`8sync . ls` / `to <n>` / `new <n> [cmd]` / `rm <n>` / `wipe` / `kick`

### Look & feel
| Verb | Mô tả |
|---|---|
| `8sync bg <kw>` | Wallhaven search → tải về `~/.local/share/8sync/wallpapers/` → set |
| `8sync bg -s yandere\|safebooru <kw>` | Đổi source |
| `8sync bg /path` | Set từ file local |
| `8sync bg https://...` | Tải URL → set |
| `8sync bg 0.7` / `+` / `-` / `off` | Opacity / nudge / clear |
| `8sync bg tint 0.5` | Background tint |
| `8sync bg pick` | fzf picker với icat preview |
| `8sync bg rotate on [N]` / `off` / `now` | Systemd-user timer đổi bg mỗi N phút |
| `8sync look <preset>` | neon\|ice\|mint\|dark\|dim (5 preset) |

### Lifecycle
| Verb | Mô tả |
|---|---|
| `8sync setup [--dry-run\|--minimal\|--no-warp\|--no-mobile\|--no-db]` | Cài full môi trường (1 lần) |
| `8sync up` | Update tool (chỉ cài nếu version mới hơn) |
| `8sync doctor` | Health check |
| `8sync flow` | Workflow help theo thứ tự dùng |
| `8sync help` | Cheatsheet |

### AI tooling
| Verb | Mô tả |
|---|---|
| `8sync skill [add\|sync]` | Quản lý skill cho forge |
| `8sync shot <url\|file>` | Render web/file → PNG (cho image-routing) |
| `8sync diff-img [ref]` | Git diff → PNG |
| `8sync pdf-img <file>` | PDF page → PNG |
| `8sync mcp` | MCP server (stub, phase 2) |

---

## 6. Session memory (project-level)

Khi user gõ `8sync .` trong project, `here.rs` seed:

```
<repo>/
├── AGENTS.md              ← do here.rs sinh, link tới agents/*
└── agents/                ← AI memory shared giữa các tool (forge/claude/cursor/aider/opencode)
    ├── PROJECT.md         facts (stack, entrypoint)
    ├── KNOWLEDGE.md       append-only: AI học được gì
    ├── DECISIONS.md       append-only: quyết định kiến trúc
    ├── PREFERENCES.md     append-only: user style
    ├── STATE.md           việc đang dở
    └── NOTES.md           `8sync note` append vào đây
```

**`agents/`** (visible folder, không phải `.gsd/` hidden) — cố ý đặt tên này để mọi AI tool đọc được qua `AGENTS.md` anchor.

`8sync end` yêu cầu AI output 4 XML block `<DECISIONS>`/`<KNOWLEDGE>`/`<PREFERENCES>`/`<STATE>` → 8sync parse & append vào file tương ứng.

---

## 7. Skill system (force-load)

Khi `8sync setup` chạy, 3 skill được copy vào `~/.forge/skills/`:

| Skill | Trigger | File |
|---|---|---|
| `karpathy-guidelines` | `always` | từ repo karpathy guidelines |
| `8sync-cli` | `always` | dạy AI dùng đúng tool 8sync |
| `image-routing` | `always` | dạy AI khi nào nên đọc image vs text |

Master force-load file: `~/.forge/skills/00-force-load.md` — forge tự đọc đầu tiên mỗi session.

---

## 8. Quy ước contribute

- **Cite code**: `crates/cli/src/verbs/bg.rs:330-373` (range), `bg.rs:330` (single line).
- **Không thêm dep nặng**: tránh `reqwest`, `tokio` cho phần nhỏ. Dùng shell-out (`curl`, `pkill`, `systemctl`) thay vì re-implement trong Rust.
- **Idempotent install**: mọi thao tác cài đặt trong `setup.rs`/`pkg.rs` phải an toàn khi chạy lần 2.
- **Smart-parse args**: 1 verb có thể nhận nhiều dạng input (vd `8sync bg 0.7` = opacity, `8sync bg /path` = file, `8sync bg cyberpunk` = search). Tránh tạo subcommand sâu.
- **Verb count target**: giữ ≤ 20 verb flat (hiện 20).
- **Binary size target**: < 2 MB stripped.
- **Help format**: mọi verb có `-h`/`--help` với `EXAMPLES` block (xem `setup.rs:7-15`).

---

## 9. Test nhanh khi PR

```bash
cargo build --release
./target/release/8sync --version
./target/release/8sync help
./target/release/8sync flow
./target/release/8sync doctor
./target/release/8sync bg                           # status, không side effect
./target/release/8sync look list
./target/release/8sync . -h                         # sub-action help
./target/release/8sync find --no-open --type rs "fn run"
```

Không có test suite chính thức (phase 1) — verify bằng smoke test trên.

---

## 10. Khi AI agent đụng repo này lần đầu

1. Đọc file này (`AGENTS.md`) đầu tiên.
2. Đọc `README.md` cho overview ngắn cho human.
3. Xem `crates/cli/src/main.rs` (subcommand map) → biết được verb nào dispatch về module nào.
4. Mỗi verb 1 file `crates/cli/src/verbs/<verb>.rs`. Mở file đúng tên là thấy logic.
5. Asset (configs, skills, presets, wallpaper URL) trong `assets/`. Embed qua `rust-embed` (xem `assets.rs:5`).
6. Khi muốn thêm verb mới: làm theo 3 bước
   - Tạo `crates/cli/src/verbs/<new>.rs` với `pub fn run(a: Args) -> Result<()>`.
   - Thêm `pub mod <new>;` trong `crates/cli/src/verbs/mod.rs`.
   - Thêm variant `<New>` trong enum `Cmd` của `main.rs` + match arm.

---

**Maintained by**: 8-Sync-Dev org · https://github.com/8-Sync-Dev/su-code
