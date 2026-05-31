# AGENTS.md — Hướng dẫn cho AI agent làm việc với `su-code`


<!-- 8sync:skills:begin -->
## 🚨 STEP 0 — `codegraph` FIRST (mandatory, no exception)

`codegraph` là **core tool** cho mọi câu hỏi liên quan đến code trong repo này. Bạn (AI) **PHẢI**:

1. Chạy `codegraph index .` **1 lần** đầu session để build/refresh semantic index.
2. Dùng `codegraph search "<query>"` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob`.
3. Dùng `codegraph deps <file>` thay cho `Read` toàn file để hiểu dependency graph.
4. Dùng `codegraph callers <symbol>` / `codegraph defs <symbol>` thay cho find-references thủ công.

Lý do: ~35% rẻ hơn token, ~70% ít tool call hơn, 100% local. Dump cả file = đốt token vô ích.

## 🚨 STEP 1 — skills: always-on (đọc ngay) vs on-demand (đọc khi cần)

Mỗi skill = 1 directory (Agent Skills open standard): `SKILL.md` có frontmatter `name`+`description`. Skill vendored ở `agents/skills/<name>/` (bản commit trong repo, mirror từ `~/.omp/skills/`). Mỗi skill liệt kê 1 lần.

### ⛔ Always-on — ĐỌC NGAY, trước tool call đầu tiên (không skip)

  1. `/home/alexdev/Projects/su-code/agents/skills/codegraph/SKILL.md`
  2. `/home/alexdev/Projects/su-code/agents/skills/8sync-cli/SKILL.md`
  3. `/home/alexdev/Projects/su-code/agents/skills/image-routing/SKILL.md`
  4. `/home/alexdev/Projects/su-code/agents/skills/karpathy-guidelines/SKILL.md`

### 🔎 On-demand — CHỈ đọc khi task khớp mô tả (bỏ qua nếu không liên quan)

- **`last30days`** — `agents/skills/last30days/SKILL.md`
     _Use this skill when the user asks "what are people saying about X", "research <topic> recently", "what's trending on Reddit/X/YouTube about Y", pre-meeting/pre-call briefings, "last 30 days of Z", competitor scans, or any recency-grounded social research. It runs the `/last30days` agent skill (separately installed engine) that searches Reddit, X, YouTube, TikTok, Hacker News, Polymarket, GitHub, Bluesky and the web in parallel, scores by real engagement, and synthesizes one cited brief. Prefer it over ad-hoc WebSearch when the user wants what the community actually thinks RIGHT NOW._

### Quy tắc bất biến

- **`codegraph` FIRST** cho mọi câu hỏi explore code (Step 0). Bypass = bug.
- Đọc TẤT CẢ skill **always-on** TRƯỚC khi gọi tool đầu tiên.
- Skill **on-demand**: chỉ mở khi description khớp task hiện tại — đừng đọc thừa.
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Nếu skill có `references/` → đọc on-demand khi task chạm chủ đề.
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.
<!-- 8sync:skills:end -->

> File này dành cho AI tool (omp, claude-code, cursor, opencode, aider, …)
> Đây là **install + usage guide** dạng deterministic — đọc xong là biết phải làm gì.

---

## 1. Repo này là gì

`su-code` = source của binary **`8sync`** — coding harness CLI cho:
- **CachyOS / Arch Linux** (target chính, hỗ trợ Manjaro/EndeavourOS)
- **Kitty terminal** (cần `allow_remote_control yes`)
- **Helix editor** (`hx` hoặc `helix`)
- **omp** (oh-my-pi.sh) — AI engine, `~/.bun/bin/omp`

Stack: **Rust** (single workspace, 1 binary `8sync` ≈ 1.3 MB stripped).

---

## 2. Cài đặt cho user

```bash
# Khuyến nghị — one-liner, binary prebuilt (không cần git/rust/cargo):
curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/install.sh | sh

# Hoặc build từ source (contributor / arch chưa có prebuilt):
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

# omp được cài tự động bởi setup; cấu hình API key theo hướng dẫn omp
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
| `8sync .` | Mở/attach session. Nếu kitty có `allow_remote_control yes` → 3-pane; nếu không → soft 1-pane + omp trong abduco |
| `8sync ai [prompt]` | AI session (resume hoặc one-shot, wrap omp) |
| `8sync find <kw>` | rg/fd + fzf preview → mở bằng `$EDITOR` (fallback hx/helix/vi) tại `file:line` |
| `8sync note "msg" [-t tag]` | Append `agents/NOTES.md` |
| `8sync run [dev\|build\|test\|fmt\|lint]` | Project command theo recipe |
| `8sync ship "msg"` | `git add -A && commit && push && gh pr create` |

### Session mgmt (sub của `.`)
`8sync . ls` / `to <n>` / `new <n> [cmd]` / `rm <n>` / `wipe` / `kick`

### Security (VPN + Firewall)
| Verb | Mô tả |
|---|---|
| `8sync sec` | Status WARP + ufw |
| `8sync sec on \| off \| toggle` | Bật/tắt/flip cả 2 |
| `8sync sec warp [on\|off\|status]` | Chỉ điều khiển WARP |
| `8sync sec ufw [on\|off\|status]` | Chỉ điều khiển ufw |

### Bluetooth (bluez)
| Verb | Mô tả |
|---|---|
| `8sync bt` | Status: rfkill / service / controller power / paired |
| `8sync bt on \| off` | Unblock + enable + power on / power off + stop |
| `8sync bt fix` | Troubleshoot adapter chết (rfkill, reload btusb, restart, power on, AutoEnable) |
| `8sync bt restart` | Restart bluetooth.service + power on |

### Clean / Optimize
| Verb | Mô tả |
|---|---|
| `8sync clean` | Reclaim disk (paccache/paru/journal/tmpfiles/thumbnails) + report CPU/GPU/RAM |
| `8sync clean --deep` | + gỡ orphan pkgs + xoá cache dev tái tạo được (uv/pip/go/…) |
| `8sync clean --ram` | + drop pagecache (nhẹ, cosmetic) |
| `8sync clean --gpu` | NVIDIA persistence mode + GPU summary |
| `8sync clean --watch [SECS]` | Loop foreground, clean mỗi SECS (default 3600) |
| `8sync clean --timer 1h \| off` | Cài/gỡ systemd user timer (loop định kỳ đúng cách, không phải bash loop) |

**Lưu ý**: "Look & feel" (wallpaper/theme/kitty layout) đã **delegate cho HyDE** — dùng `hydectl wallpaper next` và `hydectl theme set <name>`.

### Lifecycle
| Verb | Mô tả |
|---|---|
| `8sync setup` | Stage A (harness + PATH bootstrap zsh/bash/fish) + Stage B (curated y/N: dev-stack, nvidia, bluetooth, warp) |
| `8sync setup --community` | Auto-yes — Stage A + dev-stack + bluetooth (KHÔNG include warp) |
| `8sync setup --no-profile` | Chỉ harness, không hỏi profile |
| `8sync setup --profile <name>` | Apply 1 profile cụ thể non-interactive (cả community + personal) |
| `8sync setup --dry-run` | Preview, không thay đổi gì |
| `8sync setup profile list\|show\|apply <name>` | Quản lý profile sau khi setup (tag community/personal) |
| `8sync up` | Self-update binary + omp (KHÔNG chạy `pacman -Syu` — user tự lo) |
| `8sync doctor` | Health check (HyDE detect, kitty remote, gh hard-check, sec status, profiles applied) |
| `8sync flow` | Workflow help theo thứ tự dùng |
| `8sync help` | Cheatsheet |

### AI tooling
| Verb | Mô tả |
|---|---|
| `8sync skill [add <url>\|sync]` | Quản lý skill cho omp; `add` clone từ GitHub URL vào `~/.omp/skills/` và `agents/skills/`, ghi block force-load trong AGENTS.md |
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
└── agents/                ← AI memory shared giữa các tool (omp/claude/cursor/aider/opencode)
    ├── PROJECT.md         facts (stack, entrypoint)
    ├── KNOWLEDGE.md       append-only: AI học được gì
    ├── DECISIONS.md       append-only: quyết định kiến trúc
    ├── PREFERENCES.md     append-only: user style
    ├── STATE.md           việc đang dở
    └── NOTES.md           `8sync note` append vào đây
```

**`agents/`** (visible folder, không phải `.gsd/` hidden) — cố ý đặt tên này để mọi AI tool đọc được qua `AGENTS.md` anchor.

Session memory được `omp` tự quản (retain/recall/auto-compact). 8sync chỉ seed file khung — không capture nhân tạo. Quick notes vẫn ghi qua `8sync note`.

---

## 7. Skill system (force-load)

Khi `8sync setup` chạy, 3 skill bundled được copy vào `~/.omp/skills/` theo [Agent Skills open standard](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview) (SKILL.md với YAML frontmatter `name`+`description`):

| Skill | Trigger | Mô tả |
|---|---|---|
|`karpathy-guidelines`|`always`|kỷ luật engineering Karpathy-style|
|`8sync-cli`|`always`|dạy AI ưu tiên verb 8sync hơn shell thô|
|`image-routing`|`always`|chọn image vs text reads để tiết kiệm token|

Master force-load file: `~/.omp/skills/00-force-load.md` — omp đọc đầu tiên mỗi session.

**Project-local skills**: `8sync skill add <https://github.com/owner/repo>` clone vào **cả** `~/.omp/skills/<name>/` (global) **và** `<repo>/agents/skills/<name>/` (per-project). Sau đó rewrite block giữa các sentinel `8sync:skills:begin` / `8sync:skills:end` trong `AGENTS.md` với mandatory language + description từ frontmatter — AI bắt buộc đọc trước khi sửa code.

Repo chưa theo spec (không có `SKILL.md`)? 8sync fallback: phát hiện `CLAUDE.md` / `README.md` / `AGENTS.md` và liệt kê file đó làm entrypoint kèm warning.

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
