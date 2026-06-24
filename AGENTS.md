# AGENTS.md — Hướng dẫn cho AI agent làm việc với `su-code`


<!-- 8sync:skills:begin -->
## 🚨 STEP 0 — CODE INTELLIGENCE FIRST (codegraph + codebase-memory-mcp; bắt buộc)

Mọi câu hỏi về code → dùng code-intelligence engine TRƯỚC grep/read (tiết kiệm ~99% token). Bạn (AI) **PHẢI**:

1. **codegraph** (local index): `codegraph index .` 1 lần/session; rồi `codegraph query/callers/callees/impact` thay cho `grep`/`rg`/`fd`/`Grep`/`Glob` và `Read` toàn file.
2. **codebase-memory-mcp** (MCP, auto-setup bởi `8sync harness`): `search_graph`, `semantic_query`, `trace_path`, `get_architecture`, `detect_changes`, `query_graph`, `get_code_snippet` — knowledge graph 158 ngôn ngữ, query sub-ms.
3. Tìm/hiểu/định vị code · impact · route→handler · dead code · architecture → ƯU TIÊN 2 engine trên. Chỉ `Read` raw file khi sắp SỬA nó (read-before-edit).
4. **BẮT BUỘC nén output lớn:** mọi output > ~50 dòng (log/diff/test/tool dump) phải qua `headroom` MCP (`headroom_compress`) TRƯỚC khi vào context (60–95% ít token). Dump cả khối lớn vào context = vi phạm.

Lý do: 5 query cấu trúc ≈ 3.4k token vs ≈ 412k token grep từng file (−99%). Dump cả file / grep mù = đốt token = bug.

## 🚨 STEP 1 — skills 2 tầng: CORE (đọc ngay) · SPECIALIST + on-demand (đọc khi cần)

Mỗi skill = 1 directory (Agent Skills open standard): `SKILL.md` có frontmatter `name`+`description`. Skill vendored ở `agents/skills/<name>/` (bản commit trong repo, mirror từ `~/.omp/skills/`). Mỗi skill liệt kê 1 lần.

### ⛔ CORE always-on — ĐỌC NGAY (body), trước tool call đầu tiên (không skip)

Nhỏ + dùng cho MỌI task. **Thứ tự = ưu tiên (đọc top-down).** Mở `SKILL.md` ở path dưới rồi mới gọi tool đầu tiên:

  1. `/home/alexdev/Projects/su-code/agents/skills/codegraph/SKILL.md`
  2. `/home/alexdev/Projects/su-code/agents/skills/karpathy-guidelines/SKILL.md`
  3. `/home/alexdev/Projects/su-code/agents/skills/ponytail/SKILL.md`
  4. `/home/alexdev/Projects/su-code/agents/skills/8sync-cli/SKILL.md`

### 🧩 SPECIALIST always-on — biết khả năng, đọc body KHI task khớp (progressive disclosure)

KHÔNG đọc body mỗi phiên (giữ prefix gọn, tiết kiệm KV-cache). Khi task khớp → mở `SKILL.md` tương ứng NGAY. **`impeccable` = design system CHUẨN, BẮT BUỘC mở body ngay khi có việc UI/design/redesign/audit** (kèm `references/house/*`); `assp` cho copy/offer; `taste` chống slop; `image-routing` khi xử lý ảnh/diff/PDF.

- `assp-skill` — `/home/alexdev/Projects/su-code/agents/skills/assp-skill/SKILL.md`
- `impeccable` — `/home/alexdev/Projects/su-code/agents/skills/impeccable/SKILL.md`
- `design-taste-frontend` — `/home/alexdev/Projects/su-code/agents/skills/taste-skill/SKILL.md`
- `image-routing` — `/home/alexdev/Projects/su-code/agents/skills/image-routing/SKILL.md`

### 🔎 On-demand — tên = trigger; mở `SKILL.md` của skill khi task khớp (mô tả ở frontmatter, KHÔNG nhồi ở đây)

- `alpha-research` — `agents/skills/alpha-research/SKILL.md`
- `api-and-interface-design` — `agents/skills/api-and-interface-design/SKILL.md`
- `autoresearch` — `agents/skills/autoresearch/SKILL.md`
- `browser-testing-with-devtools` — `agents/skills/browser-testing-with-devtools/SKILL.md`
- `ci-cd-and-automation` — `agents/skills/ci-cd-and-automation/SKILL.md`
- `code-review-and-quality` — `agents/skills/code-review-and-quality/SKILL.md`
- `code-simplification` — `agents/skills/code-simplification/SKILL.md`
- `context-engineering` — `agents/skills/context-engineering/SKILL.md`
- `contributing` — `agents/skills/contributing/SKILL.md`
- `debugging-and-error-recovery` — `agents/skills/debugging-and-error-recovery/SKILL.md`
- `deep-research` — `agents/skills/deep-research/SKILL.md`
- `deprecation-and-migration` — `agents/skills/deprecation-and-migration/SKILL.md`
- `docker` — `agents/skills/docker/SKILL.md`
- `documentation-and-adrs` — `agents/skills/documentation-and-adrs/SKILL.md`
- `doubt-driven-development` — `agents/skills/doubt-driven-development/SKILL.md`
- `eli5` — `agents/skills/eli5/SKILL.md`
- `frontend-ui-engineering` — `agents/skills/frontend-ui-engineering/SKILL.md`
- `full-flow` — `agents/skills/full-flow/SKILL.md`
- `git-workflow-and-versioning` — `agents/skills/git-workflow-and-versioning/SKILL.md`
- `gs` — `agents/skills/gs/SKILL.md`
- `idea-refine` — `agents/skills/idea-refine/SKILL.md`
- `incremental-implementation` — `agents/skills/incremental-implementation/SKILL.md`
- `interview-me` — `agents/skills/interview-me/SKILL.md`
- `jobs` — `agents/skills/jobs/SKILL.md`
- `last30days` — `agents/skills/last30days/SKILL.md`
- `literature-review` — `agents/skills/literature-review/SKILL.md`
- `ml-training-recipe` — `agents/skills/ml-training-recipe/SKILL.md`
- `modal-compute` — `agents/skills/modal-compute/SKILL.md`
- `observability-and-instrumentation` — `agents/skills/observability-and-instrumentation/SKILL.md`
- `paper-code-audit` — `agents/skills/paper-code-audit/SKILL.md`
- `paper-writing` — `agents/skills/paper-writing/SKILL.md`
- `peer-review` — `agents/skills/peer-review/SKILL.md`
- `performance-optimization` — `agents/skills/performance-optimization/SKILL.md`
- `planning-and-task-breakdown` — `agents/skills/planning-and-task-breakdown/SKILL.md`
- `ponytail-audit` — `agents/skills/ponytail-audit/SKILL.md`
- `ponytail-debt` — `agents/skills/ponytail-debt/SKILL.md`
- `ponytail-help` — `agents/skills/ponytail-help/SKILL.md`
- `ponytail-review` — `agents/skills/ponytail-review/SKILL.md`
- `preview` — `agents/skills/preview/SKILL.md`
- `replication` — `agents/skills/replication/SKILL.md`
- `research-review` — `agents/skills/research-review/SKILL.md`
- `runpod-compute` — `agents/skills/runpod-compute/SKILL.md`
- `security-and-hardening` — `agents/skills/security-and-hardening/SKILL.md`
- `senior-frontend` — `agents/skills/senior-frontend/SKILL.md`
- `senior-security` — `agents/skills/senior-security/SKILL.md`
- `session-log` — `agents/skills/session-log/SKILL.md`
- `session-search` — `agents/skills/session-search/SKILL.md`
- `shipping-and-launch` — `agents/skills/shipping-and-launch/SKILL.md`
- `social-growth` — `agents/skills/social-growth/SKILL.md`
- `source-comparison` — `agents/skills/source-comparison/SKILL.md`
- `source-driven-development` — `agents/skills/source-driven-development/SKILL.md`
- `spec-driven-development` — `agents/skills/spec-driven-development/SKILL.md`
- `test-driven-development` — `agents/skills/test-driven-development/SKILL.md`
- `using-agent-skills` — `agents/skills/using-agent-skills/SKILL.md`
- `watch` — `agents/skills/watch/SKILL.md`

### Quy tắc bất biến

- **Code-intelligence FIRST** (codegraph + codebase-memory-mcp) cho mọi câu hỏi explore code (Step 0). Bypass = bug.
- **Output > ~50 dòng → BẮT BUỘC `headroom_compress`** trước khi vào context — không dump thô.
- Đọc body **CORE** (codegraph → karpathy → ponytail → 8sync-cli) TRƯỚC tool call đầu tiên. **SPECIALIST** (assp · impeccable · taste · image-routing) đọc body KHI task khớp — `impeccable` bắt buộc ngay khi có việc UI/design.
- Skill **on-demand**: chỉ mở khi description khớp task hiện tại — đừng đọc thừa.
- Nếu skill có `scripts/` → ưu tiên invoke script đó thay vì viết lại logic.
- Khi áp dụng skill, **cite** rõ: ví dụ `agents/skills/<name>/SKILL.md:line`.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (mục Unreleased) + ghi học được vào `agents/KNOWLEDGE.md`.
- **Doc-hygiene**: chạy `8sync harness audit` khi đụng vùng có docs — path lệch→fix, doc rác/superseded→xóa (thêm doc phải kèm xóa cái cũ), oversized→trim.
- **Loop / STATE spine**: đọc `agents/STATE.md` đầu phiên; rewrite ở mỗi phase-boundary (Goal·Checklist·Current·Next). Context gần đầy → handoff vào STATE + bài học vào KNOWLEDGE rồi reinit. Đo loop: `8sync harness bench`.
- **Loop discipline (C/D/E)**: implementer↔verifier qua `task` (verifier chạy build/test ĐỘC LẬP, verify-gate TRƯỚC commit); FAIL → ghi `failure:` vào KNOWLEDGE, đọc đầu phiên để khỏi lặp; quy trình `validated:` → distill vào `agents/PLAYBOOKS.md` (index theo `When:`); autonomy L1 report · L2 assisted · L3 unattended — không tự `push`/PR ở L3 mặc định.
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

Stack: **Rust** (single workspace, 1 binary `8sync` ≈ 3.8 MB stripped — gồm 15 bundled skill, nặng nhất là `impeccable` ~2 MB scripts/reference).

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
    └── skills/                                              8 bundled (codegraph, karpathy, assp-skill, impeccable, taste-skill, 8sync-cli, image-routing, last30days) + 00-force-load.md
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
| `8sync clean --deep` | + gỡ orphan pkgs + build cache thuần (go-build/tsc/node-gyp). **KHÔNG** đụng model (huggingface/torch), Playwright/Puppeteer/Electron binary, hay cache tải gói (uv/pip/yarn/pnpm/deno) — chỉ report + gợi ý lệnh xoá tay |
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
| `8sync harness [init\|up\|help]` | **bare `8sync harness`** = ONE idempotent command: deploy/update skill + mirror (additive, KHÔNG đè skill đã sửa) + inject + seed memory + consolidate + codegraph index. **init**: full bootstrap (progress UI) + **managed `.gitignore`** (ignore `.codegraph/`/`.cache/`/`.env*`, keep `agents/`+`agents/skills/`) + **gitleaks pre-commit hook**; `--force` re-mirror đè hết. **up**: light refresh (`--pull` re-pull skill; `--commit` git-commit memory — gitleaks scan trước, abort nếu rò secret; `--loop`/`--timer` chạy nền; tự consolidate `## Learnings` >200 dòng → `agents/archive/`). **help**: cheatsheet |
| `8sync skill [add <spec>\|gen \|list\|update]` | Quản lý skill: `add` clone GitHub (collection-aware) / `builtin:<name>` / **`<url>@<ref>` để pin commit/tag** (ghi `rev` vào `skills.toml` = lockfile, reproducible); `update [name]` re-pull theo `src` (git dedup theo URL, honor `rev` pin); `gen` fuse N skill |
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

Khi `8sync harness init` (hoặc `8sync setup`) chạy, **15 skill bundled** được deploy vào `~/.omp/skills/` theo [Agent Skills open standard](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview). 8 skill **always-on** đọc theo đúng thứ tự ưu tiên (codegraph → karpathy → ponytail → assp → impeccable → taste → 8sync-cli → image-routing); phần còn lại on-demand; `encore-deploy` tech-gated; `social-growth` opt-in:

| Skill | Trigger | Mô tả |
|---|---|---|
|`codegraph`|`always`|semantic code intelligence (binary + SKILL.md) — STEP 0, mọi explore code|
|`karpathy-guidelines`|`always`|kỷ luật engineering Karpathy-style|
|`ponytail`|`always`|"laziest senior dev" — YAGNI, làm ít nhất, xoá > thêm|
|`assp-skill`|`always`|brand DNA 8 Sync Dev + ASSP validate-before-build (UI copy, landing/pricing, feature mới)|
|`impeccable`|`always`|**design system CHUẨN — BẮT BUỘC cho mọi UI/design/redesign/audit**; có `scripts/` + `references/house/*` (frontend-agent-workflow + clouds-f orchestration + keyword routers)|
|`taste-skill`|`always`|anti-slop frontend taste cho landing/portfolio/redesign|
|`8sync-cli`|`always`|dạy AI ưu tiên verb 8sync hơn shell thô|
|`image-routing`|`always`|chọn image vs text reads để tiết kiệm token|
|`code-review-and-quality` · `senior-security` · `senior-frontend`|on-demand|review/quality/security/frontend chuyên sâu|
|`full-flow`|on-demand|self-driving fix/dev/verify loop (Encore + Next)|
|`encore-deploy`|tech-gated|deploy runbook — chỉ hiện khi project dùng Encore|
|`last30days`|on-demand|research social recency (Reddit/X/YouTube/HN…)|
|`social-growth`|opt-in|social/branding/leads — bật bằng `8sync skill add builtin:social-growth`|

**External skill packs** (best-effort, `harness init` tự clone vào `~/.omp/skills/`): [`ponytail`](https://github.com/DietrichGebert/ponytail) (full: audit/debt/review/help) + [`addyosmani/agent-skills`](https://github.com/addyosmani/agent-skills) (24 production-grade eng skills). Offline thì skip; bundled vẫn đủ mạnh.

Master force-load file: `~/.omp/skills/00-force-load.md` — omp đọc đầu tiên mỗi session.

**Project-local skills**: `8sync skill add <https://github.com/owner/repo>` clone vào **cả** `~/.omp/skills/<name>/` (global) **và** `<repo>/agents/skills/<name>/` (per-project). Sau đó rewrite block giữa các sentinel `8sync:skills:begin` / `8sync:skills:end` trong `AGENTS.md` với mandatory language + description từ frontmatter — AI bắt buộc đọc trước khi sửa code.

Repo chưa theo spec (không có `SKILL.md`)? 8sync fallback: phát hiện `CLAUDE.md` / `README.md` / `AGENTS.md` và liệt kê file đó làm entrypoint kèm warning.

---

## 8. Quy ước contribute

- **Cite code**: `crates/cli/src/verbs/bg.rs:330-373` (range), `bg.rs:330` (single line).
- **Không thêm dep nặng**: tránh `reqwest`, `tokio` cho phần nhỏ. Dùng shell-out (`curl`, `pkill`, `systemctl`) thay vì re-implement trong Rust.
- **Idempotent install**: mọi thao tác cài đặt trong `setup.rs`/`pkg.rs` phải an toàn khi chạy lần 2.
- **Smart-parse args**: 1 verb có thể nhận nhiều dạng input (vd `8sync bg 0.7` = opacity, `8sync bg /path` = file, `8sync bg cyberpunk` = search). Tránh tạo subcommand sâu.
- **Verb count target**: giữ ≤ 22 verb flat (hiện 21, thêm `harness`).
- **Binary size target**: < 4 MB stripped (tăng từ 2 MB khi bundle `impeccable` — skill frontend nặng ~2 MB). Skill mới chỉ bundle nếu thật sự always-on; còn lại để `8sync skill add`.
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
