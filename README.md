# su-code (`8sync`)

> **VI:** Coding harness terminal-first cho CachyOS/Arch + Kitty + Helix + [omp](https://omp.sh).
> Bạn vẫn dùng terminal như thường ngày; AI agent quan sát ngữ cảnh project, đọc memory `agents/*`, và thực thi lệnh khi bạn yêu cầu.
>
> **EN:** Terminal-first AI coding harness for CachyOS/Arch + Kitty + Helix + [omp](https://omp.sh).
> Keep your normal CLI workflow; AI agents observe project context, load `agents/*` memory, and execute tasks on demand.

- **Website / docs**: <https://8-sync-dev.github.io/su-code> (auto-deploy từ `docs/` qua [`.github/workflows/pages.yml`](.github/workflows/pages.yml))
- **Repo**: <https://github.com/8-Sync-Dev/su-code>
- **Discussions**: <https://github.com/orgs/8-Sync-Dev/discussions>
- **AI engine**: [omp](https://omp.sh) (oh-my-pi) — `8sync` wrap quanh `omp --continue` để giữ session per-project.

---

## TL;DR

```bash
# Cài lần đầu (Arch/CachyOS, đã có git)
git clone https://github.com/8-Sync-Dev/su-code.git
cd su-code
bash scripts/bootstrap.sh        # → ~/.local/bin/8sync
8sync setup --dry-run            # xem plan trước
8sync setup                      # cài harness + chọn profile y/N
8sync doctor                     # verify

# Dùng hằng ngày
cd <project>
8sync .                          # mở session (kitty 3-pane + omp trong abduco)
8sync ai "explain this codebase" # one-shot prompt; bỏ trống để resume session
8sync ship "feat: ..."           # add + commit + push + gh pr create
```

---

## Cài đặt

### 1. Bootstrap (máy mới)

`scripts/bootstrap.sh` cài rustup (nếu thiếu) → `cargo build --release --locked` → copy binary vào `~/.local/bin/8sync`.

```bash
git clone https://github.com/8-Sync-Dev/su-code.git
cd su-code
bash scripts/bootstrap.sh
```

Đảm bảo `~/.local/bin` trong `$PATH`:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc   # hoặc ~/.bashrc
```

### 2. `8sync setup` — cài phần còn lại

Stage A (harness, luôn idempotent):

- `pacman -S --needed helix lazygit abduco github-cli`
- omp CLI qua `curl -fsSL https://omp.sh/install | sh` (skip nếu đã có)
- ghi config: `~/.config/helix/`, `~/.config/kitty/8sync.session`, `~/.config/8sync/{global,skills}.toml`
- ghi skill: `~/.omp/skills/{karpathy-guidelines,8sync-cli,image-routing}/SKILL.md` + `00-force-load.md`

Stage B (profile cá nhân, opt-in y/N từng cái): `vietnamese`, `hardware-cooling`, `hardware-lianli`, `displaylink`, `apps-personal`, `warp`, hoặc bundle `alexdev`.

Cờ thường dùng:

| Cờ | Hiệu ứng |
|---|---|
| `8sync setup --dry-run` | In plan, không thay đổi gì |
| `8sync setup --no-profile` | Chỉ Stage A |
| `8sync setup --yall` | Stage A + apply tất cả profile, không hỏi |
| `8sync setup --profile <name>` | Stage A + apply 1 profile cụ thể |
| `8sync setup profile list \| show <n> \| apply <n>` | Quản lý profile sau setup |

### 3. Update

```bash
8sync up                         # self-update binary (GitHub release) + omp update
```

Hoặc rebuild thủ công từ source:

```bash
cd su-code && git pull
cargo build --release
install -m755 target/release/8sync ~/.local/bin/8sync
```

System packages (`pacman -Syu`) **không** tự chạy — bạn tự quyết khi nào update CachyOS rolling.

---

## Lệnh chính

### Vibe loop (hằng ngày)

| Lệnh | Mô tả |
|---|---|
| `8sync .` | Mở/attach session project hiện tại. Kitty có `allow_remote_control yes` → 3-pane; nếu không → soft 1-pane + `omp --continue` trong abduco |
| `8sync ai [prompt]` | Trống/`continue` → `omp --continue`; có prompt → `omp -p "..."` |
| `8sync find <kw>` | rg/fd + fzf preview → mở editor tại `file:line` |
| `8sync note "msg" [-t tag]` | Append `agents/NOTES.md` |
| `8sync run [dev\|build\|test\|fmt\|lint]` | Project runner theo recipe |
| `8sync ship "msg"` | `git add -A && commit && push && gh pr create` |

### Session quản lý (sub của `.`)

`8sync . ls` / `to <n>` / `new <n> [cmd]` / `rm <n>` / `wipe` / `kick`

### Skill system

| Lệnh | Mô tả |
|---|---|
| `8sync skill` | List skill global (`~/.omp/skills/`) + local project (`agents/skills/`) |
| `8sync skill add <github-url>` | Clone repo skill vào **cả** `~/.omp/skills/<name>/` (omp đọc) **và** `<project>/agents/skills/<name>/` (memory dự án). Rewrite block `<!-- 8sync:skills:* -->` trong `AGENTS.md` |
| `8sync skill add gh:owner/repo` | Short form |
| `8sync skill add path:/abs/path` | Symlink từ local dir |
| `8sync skill sync` | Refresh `~/.omp/skills/00-force-load.md` từ asset bundle |

Idempotent: chạy lại `add` cùng URL → `git pull --ff-only` thay vì clone lại.

### Lifecycle

| Lệnh | Mô tả |
|---|---|
| `8sync setup` | Cài harness + profile (xem mục Cài đặt) |
| `8sync up` | Self-update binary + `omp update` |
| `8sync doctor` | Health check (kitty remote, omp, helix, gh, configs, profiles, WARP/ufw) |
| `8sync flow` | Workflow help theo thứ tự dùng |
| `8sync help` | Cheatsheet (alias của `8sync` không tham số) |

### AI tooling

| Lệnh | Mô tả |
|---|---|
| `8sync shot <url\|file>` | Render web/file → PNG (cho image-routing skill) |
| `8sync diff-img [ref]` | Git diff → PNG |
| `8sync pdf-img <file>` | PDF page → PNG |

### Security

`8sync sec [on\|off\|toggle\|status]` — bật/tắt cùng lúc Cloudflare WARP VPN + ufw firewall. Sub: `sec warp …`, `sec ufw …`.

Mọi verb hỗ trợ `-h` / `--help` với block `EXAMPLES` chi tiết.

---

## Memory project

Khi `8sync .` lần đầu trong project, file/folder sau được seed:

```
<repo>/
├── AGENTS.md                    ← anchor cho mọi AI tool, chứa block force-load skills
└── agents/                      ← memory shared (omp/claude-code/cursor/opencode/aider)
    ├── PROJECT.md               facts cố định (stack, entrypoint)
    ├── KNOWLEDGE.md             append-only: AI học được gì
    ├── DECISIONS.md             append-only: quyết định kiến trúc
    ├── PREFERENCES.md           append-only: style user
    ├── STATE.md                 việc đang dở
    ├── NOTES.md                 quick notes via `8sync note`
    └── skills/                  project-local skills (clone qua `8sync skill add <url>`)
```

`omp` tự quản session memory (`retain` / `recall` / auto-compact) — bạn **không** sửa tay `agents/*.md`. `8sync note` là exception duy nhất (append vào `NOTES.md`).

---

## Documentation site

Trang web tĩnh trong `docs/index.html`, deploy tự động qua GitHub Pages:

- **Source**: [`docs/index.html`](docs/index.html)
- **Workflow**: [`.github/workflows/pages.yml`](.github/workflows/pages.yml) (trigger: push `main` hoặc workflow_dispatch)
- **URL**: <https://8-sync-dev.github.io/su-code>

Sửa `docs/index.html` → push `main` → Pages tự rebuild trong ~1 phút.

---

## Stack & contribute

Rust workspace 1 binary (`8sync` ≈ 1.3 MB stripped). Toolchain pin tại `rust-toolchain.toml`.

Bố cục source:

```
crates/cli/src/
├── main.rs                       clap router
├── ui.rs · env_detect.rs · pkg.rs · assets.rs
└── verbs/                        1 file / 1 verb
    ├── root.rs flow.rs setup.rs doctor.rs up.rs selfup.rs
    ├── here.rs ai.rs ship.rs run.rs find.rs note.rs
    ├── skill.rs shot.rs diff_img.rs pdf_img.rs
    ├── profile.rs sec.rs
assets/                           embed vào binary qua rust-embed
├── configs/                      kitty.session, helix-config, fish-config, 8sync/*.toml
├── presets/                      kitty preset themes
├── skills/                       built-in skills (karpathy, 8sync-cli, image-routing)
└── wallpapers/
```

Khi thêm verb mới: tạo `verbs/<new>.rs` với `pub fn run(a: Args) -> Result<()>`, thêm `pub mod <new>;` vào `verbs/mod.rs`, và variant `<New>` + match arm trong `main.rs`.

Smoke test:

```bash
cargo build --release
./target/release/8sync --version
./target/release/8sync help
./target/release/8sync flow
./target/release/8sync doctor
./target/release/8sync skill
```

Xem [`AGENTS.md`](AGENTS.md) cho hướng dẫn chi tiết dành cho AI agent / contributor.

---

## License

MIT. See [`LICENSE`](LICENSE).

`#8sync #AIAgent #VibeCoding #omp #CodingHarness #TerminalWorkflow #DeveloperTools #RustLang #KittyTerminal #HelixEditor #ArchLinux #CachyOS #OpenSource`
