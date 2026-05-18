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
bash scripts/bootstrap.sh        # cài rustup (nếu thiếu) + build + install vào ~/.local/bin
```

Sau đó:
```bash
8sync setup                      # harness slim + hỏi y/N từng personal profile
# hoặc:
8sync setup --yall               # cài full harness + ALL profiles, không hỏi
8sync setup --no-profile         # chỉ harness (không hỏi profile)
8sync setup --profile alexdev    # apply bundle cá nhân hóa của alexdev

forge login                      # paste API key của forge
8sync doctor                     # verify
```

**Quy tắc an toàn**:
- `8sync setup --dry-run` xem trước không thay đổi gì.
- Mọi `pacman -S` / AUR install đều **transactional**: snapshot pkg mới trước khi install, nếu fail sẽ `pacman -Rns` rủi ro những pkg đã cài được trong batch đó (xem `pkg::pacman_install_safe` / `aur_install_safe`).
- Re-run setup là idempotent: đã cài → skip.

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
│           ├── profile.rs   (load/resolve/apply assets/profiles/*.toml + state)
│           ├── sec.rs       (WARP VPN + ufw firewall toggle: on/off/status/toggle)
│           ├── find.rs (rg+fzf+$EDITOR) · note.rs (agents/NOTES.md)
│           └── skill.rs · shot.rs · diff_img.rs · pdf_img.rs
└── assets/                                                  bundled vào binary qua rust-embed
    ├── configs/                                             helix config + theme + kitty/8sync.session + 8sync/{global,skills}.toml
    ├── profiles/                                            7 personal profile TOML (vietnamese, hw-cooling, hw-lianli, displaylink, apps-personal, warp, alexdev-bundle)
    └── skills/                                              karpathy, image-routing, 8sync-cli + 00-force-load.md
```

---

## 5. Toàn bộ verb (13 verb flat sau khi slim-down cho HyDE)

### Vibe loop (daily, dùng liên tục)
| Verb | Mô tả |
|---|---|
| `8sync .` | Mở/attach session. Nếu kitty có `allow_remote_control yes` → 3-pane; nếu không → soft 1-pane + forge trong abduco |
| `8sync ai [prompt]` | AI session (resume hoặc one-shot, wrap forge) |
| `8sync find <kw>` | rg/fd + fzf preview → mở bằng `$EDITOR` (fallback hx/helix/vi) tại `file:line` |
| `8sync note "msg" [-t tag]` | Append `agents/NOTES.md` |
| `8sync run [dev\|build\|test\|fmt\|lint]` | Project command theo recipe |
| `8sync ship "msg"` | `git add -A && commit && push && gh pr create` |
| `8sync end` | AI capture knowledge → `agents/*.md` |

### Session mgmt (sub của `.`)
`8sync . ls` / `to <n>` / `new <n> [cmd]` / `rm <n>` / `wipe` / `kick`

### Security (VPN + Firewall)
| Verb | Mô tả |
|---|---|
| `8sync sec` | Status WARP + ufw |
| `8sync sec on \| off \| toggle` | Bật/tắt/flip cả 2 |
| `8sync sec warp [on\|off\|status]` | Chỉ điều khiển WARP |
| `8sync sec ufw [on\|off\|status]` | Chỉ điều khiển ufw |

**Lưu ý**: "Look & feel" (wallpaper/theme/kitty layout) đã **delegate cho HyDE** — dùng `hydectl wallpaper next` và `hydectl theme set <name>`.

### Lifecycle
| Verb | Mô tả |
|---|---|
| `8sync setup` | Stage A (harness slim: helix/lazygit/abduco/gh + forge + configs + skills) + Stage B (hỏi y/N từng profile) |
| `8sync setup --yall` | Auto-yes — cài harness + ALL profiles (bundle `alexdev`) không prompt |
| `8sync setup --no-profile` | Chỉ harness, không hỏi profile |
| `8sync setup --profile <name>` | Apply 1 profile cụ thể non-interactive |
| `8sync setup --dry-run` | Preview, không thay đổi gì |
| `8sync setup profile list\|show\|apply <name>` | Quan lý profile sau khi setup |
| `8sync up` | Self-update binary + forge (KHÔNG chạy `pacman -Syu` — user tự lo) |
| `8sync doctor` | Health check (HyDE detect, kitty remote, gh hard-check, sec status, profiles applied) |
| `8sync flow` | Workflow help theo thứ tự dùng |
| `8sync help` | Cheatsheet |

### AI tooling
| Verb | Mô tả |
|---|---|
| `8sync skill [add\|sync]` | Quản lý skill cho forge |
| `8sync shot <url\|file>` | Render web/file → PNG (cho image-routing) |
| `8sync diff-img [ref]` | Git diff → PNG |
| `8sync pdf-img <file>` | PDF page → PNG |

---

## 5b. Profile system (Stage B của setup)

7 built-in profile trong `assets/profiles/*.toml`:

| Profile | Nội dung | Cần AUR helper |
|---|---|---|
| `vietnamese` | fcitx5 + Unikey | no |
| `hardware-cooling` | coolercontrol + openrgb + liquidctl | no |
| `hardware-lianli` | `lianli-linux-git` (yay/paru auto-pulls deps) | **yes** |
| `displaylink` | evdi-dkms | no |
| `apps-personal` | bitwarden | no |
| `warp` | `cloudflare-warp-bin` + enable warp-svc + config DoH/MASQUE/malware DNS | **yes** |
| `alexdev` | bundle: extends cả 6 profile trên | yes (qua warp/lianli) |

User có thể thay/thêm profile trong `~/.config/8sync/profiles/*.toml` (override built-in).

State luư ở `~/.config/8sync/profile.toml`:
```toml
applied = ["vietnamese", "hardware-cooling", ...]
last_setup = "epoch:..."
```

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
