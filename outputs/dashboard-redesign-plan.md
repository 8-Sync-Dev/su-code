# Plan — `8sync harness web` dashboard: fix + redesign (impeccable)

**Date:** 2026-06-29 · Status tracker. Driven by user screenshot review. FE redesign uses the **impeccable** design system (product register).

## Issues (from review)
1. **Layout xấu / khó dùng** — redesign lớn toàn bộ screen (impeccable). React-flow canvas Workflow bị bể (ô xám tí xíu).
2. **serena = off** dù đã registered + `uv` có → false-negative detection.
3. **Context >50% không auto-compact** — config đúng (`thresholdPercent:50`) nhưng dashboard giả định window 1M → % lệch omp thật.
4. **Workflow có templates mẫu nhưng không hiện** ("Saved workflows: none yet").
5. **Không có chỗ sửa model theo task/role** → cần trang Models (view+edit `models.toml`).
6. **Markdown không render** (State/Context hiện raw `## Goal` …).
7. **Tổ chức bố cục** — redesign nếu cần.
8. **Project switcher** — scan ALL project omp đang work; chấm **xanh=on** (omp đang chạy ở đó), **xám=off**; đổi qua lại, khỏi phải cd vào từng project gõ `harness web`.
9. **Rà soát all skills repo** — đã có skill FE chuẩn (impeccable/taste/senior-frontend) → dùng nó redesign.

## API contract (backend = web.rs; FE builds to this)
- `GET /api/engines` — serena `present:true` nếu **registered trong `~/.omp/agent/mcp.json`** (+ `uv`/`uvx` có), KHÔNG dựa `which serena` (serena chạy qua `uvx`, không có binary PATH).
- `GET /api/context` — đọc window THẬT của model active nếu biết; nếu không, giữ 1M nhưng `assumed:true`; trả `usedTok,windowTok,pct,thresholdPct,willCompact,assumed`.
- `GET /api/models` → `{ path, roles:{default,plan,smol,slow}, tasks:{plan,review,debug,code,trivial→model}, classes:[…] }` (reuse `crate::models::ModelConfig::load`).
- `POST /api/models` `{section:"roles"|"tasks", key, value}` → ghi `~/.config/8sync/models.toml` (reuse logic `harness/model.rs`); trả config mới.
- `GET /api/projects` → scan `~/.omp/agent/sessions/<slug>` → map slug→project path (reuse `session_slug`) → `[{name,path,active,lastModified}]`; `active=true` nếu có session sửa gần đây (≤30m) → chấm xanh, else xám.
- `GET /api/workflows/templates` → vài workflow mẫu (research→plan→build, review, qa) để FE hiện dưới "Saved workflows".

## FE (web/src, impeccable, product register)
- Render markdown (State/Memory/Context): headings/list/code/bold/checkbox.
- **Models page** (nav mới): roles+tasks, edit inline (POST), show path + available.
- **Project switcher** (header/sidebar top): list projects + chấm xanh/xám; click đổi project data.
- **Workflow templates** hiện + load được; fix react-flow canvas (full height, không bể).
- Redesign all screens: contrast ≥4.5:1, spacing rhythm, no glassmorphism-default, no over-round, empty states, responsive; browser-verify mọi trang (0 console error).

## Phases / track — DONE, shipped v0.29.0
- [x] BE: serena detection · context honesty (`assumed/willCompact`) · /api/models GET+POST · /api/projects · /api/workflows/templates
- [x] FE: markdown render (`markdown.tsx`) · Models page (inline edit) · project switcher (status dots, 24 projects) · workflow templates + fixed 560px canvas · full impeccable redesign (14 pages)
- [x] Integrate + `cargo build` (build.rs embeds FE) + browser-verify (0 console errors) + ship v0.29.0
- Bonus: model-routing philosophy locked — Opus=thinking (plan/review/debug/vision) · GLM=mechanical (code/edit/default/trivial); omp `config.yml` vision→opus.
- DEFERRED: full capability-scoring per-task router (TS engine) — documented as future target.
