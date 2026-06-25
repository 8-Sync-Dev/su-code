# Research — omp customization · memory/"never-forget" · platform · submodule-ref · eval%

**Date:** 2026-06-25 · **Method:** đọc omp docs (`omp://`) + repos (gstack, gsd-pi, agent-reach) + su-code source. Mọi claim cite nguồn; chỗ không khả thi nói thẳng.
**Tóm tắt:** vài ý là ngộ nhận (GGUF fine-tune, "training dự án") — đính chính + chỉ cơ chế THẬT (Mnemopi memory + cbm + spine). Phần khả thi → plan có điểm chạm.

---

## 0. Verdict nhanh (mỗi câu hỏi 1 dòng)

| Câu hỏi | Verdict |
|---|---|
| Auto tương thích từng platform | su-code **đã** an toàn cho phần nó đăng ký (PATH-resolved, không path tuyệt đối); full Windows/macOS = **ngoài scope** (su-code target CachyOS/Arch theo `AGENTS.md`). Lỗi gsd-workflow là của gsd-pi trên Windows, không phải su-code. |
| Gắn TẤT CẢ skill repo làm submodule auto-pull | **Ngộ nhận:** submodule **PIN SHA**, KHÔNG auto-latest. Skill repos đã auto-latest qua `harness up --pull`/`skill update`. Reference repos nên submodule (hoặc `read` on-demand). |
| agent-reach | Capability layer (đọc internet) — thêm làm **skill/MCP**, KHÔNG phải agent-team engine. |
| eval bao nhiêu %? | `harness eval` = 3-task suite, giờ in **`%`** (đã code). KHÔNG phải "team quality % theo dự án" — đó cần fixtures riêng. |
| Lệnh training/pretrain/fine-tune dự án | **KHÔNG tồn tại + không khả thi.** omp = inference; local model là **ONNX q4, KHÔNG GGUF**, chỉ cho title/memory/auto-classifier. |
| GGUF super-lightweight enhance speed | omp dùng **ONNX q4** (transformers.js), không GGUF; chỉ tăng tốc 3 task phụ, KHÔNG phải coding agent chính. |
| "Remind/recall smart, ý thức sâu, không quên" | **CÓ THẬT** = **Mnemopi memory backend** (`memory.backend: mnemopi`) — đang **OFF**. Đây là câu trả lời đúng, không phải fine-tune. |
| gstack "agent team đâu"? | **Đúng — gstack KHÔNG có team tự động**; nó là 23 slash-command + bạn tự mở nhiều session. Team THẬT = omp `task` subagents + `irc` → `/gs` đã dùng. |
| Custom command / workflow automation cao | omp cho `.omp/commands/*.md` (native, prio 100) + extensions + hooks + custom-tools + MCP. su-code **đã dùng đúng base** → omp update không conflict. |

---

## 1. Memory / "ý thức sâu, không bao giờ quên" — cơ chế THẬT (ưu tiên #1)

Bạn muốn agent "học dự án, nhớ mãi, kết hợp nhiều skill/tool". Cách ĐÚNG **không phải** train/fine-tune model — mà là **3 tầng nhớ** đã có sẵn:

1. **Mnemopi (omp memory backend)** — `omp://mnemosyne-memory-backend.md`. Local SQLite long-term memory:
   - auto-recall vào block `<memories>` ở turn đầu · auto-retain mỗi vài turn · scoping `per-project-tagged` (ghi project + recall cả global).
   - polyphonic recall (vector + graph + fact + temporal, RRF) · embeddings (bge/e5) · `/memory view|stats|diagnose|clear|enqueue`.
   - **ĐANG OFF** (`memory.backend` default `off`). Bật = câu trả lời cho "không quên".
2. **codebase-memory-mcp (cbm)** — knowledge graph code (search_graph/trace_path), re-index mỗi `harness`. = "ý thức cấu trúc" về dự án.
3. **Spine `agents/`** — STATE (plan sống) · KNOWLEDGE (`validated/failure`) · PLAYBOOKS (procedural, Voyager-style) · DECISIONS. = bài học + quy trình tái dùng.

→ **Kết hợp 3 tầng = "deep awareness không quên" thật sự**, do accumulation (memory) chứ không phải trained weights.

**Bật Mnemopi** (`~/.omp/agent/config.yml`, `omp://config-usage.md` §4):
```yaml
memory:
  backend: mnemopi
mnemopi:
  scoping: per-project-tagged
  polyphonicRecall: true
  llmMode: smol        # extraction/consolidation = model API đang dùng (KHÔNG tải model local)
  noEmbeddings: true   # FTS-only recall — 0 dependency local, chạy được trên máy yếu
```
**Quyết định (chốt): KHÔNG local model** — máy nào cũng chạy. `llmMode: smol` dùng **model API đang chạy** cho extraction/consolidation; `noEmbeddings: true` né tải embedding model local (bge/e5 ~vài trăm MB) → recall = full-text search, đủ tốt + machine-agnostic. *(Tùy chọn nâng recall semantic mà vẫn 0-local: `mnemopi.embeddingApiUrl/embeddingApiKey` = embeddings API remote — tốn chút API.)*
**Tradeoff:** recall inject ~tới `injectionTokenLimit` (5000) token/phiên — đánh đổi token lấy trí nhớ (cân nhắc vì vừa tối ưu token). Đề xuất: `8sync harness` ensure block này (idempotent) + `8sync doctor` báo memory backend on/off.

---

## 2. GGUF / training / fine-tune — đính chính (không khả thi như hình dung)

- `omp://local-models.md`: local model là **`@huggingface/transformers` + onnxruntime-node, quant `q4` (ONNX)** — **KHÔNG GGUF**. Chỉ 3 task phụ: **session-title**, **Mnemopi memory extraction/consolidation**, **`auto` thinking classifier**. Coding agent chính vẫn gọi API online.
- `omp://mnemosyne-memory-backend.md` (nguyên văn): *"The backend does NOT download or run a local GGUF LLM."*
- **KHÔNG có** cơ chế train/pretrain/fine-tune trên dự án. LLM weights không đổi.
- **Chốt: KHÔNG dùng local model** (máy yếu vẫn chạy) — Mnemopi extraction/consolidation đi qua **model API đang dùng** (`mnemopi.llmMode: smol`), KHÔNG tải ONNX/GGUF. (`providers.memoryModel`/`tinyModel` ONNX q4 chỉ là tùy chọn cho ai máy mạnh + muốn offline — KHÔNG mặc định.)
- Nếu thật sự muốn GGUF/local LLM cho agent chính: phải chạy một **OpenAI-compatible server riêng** (llama.cpp/ollama/vLLM) rồi trỏ omp provider vào (`omp://providers.md`, `omp://local-models.md` chỉ ONNX) — đó là **inference local**, vẫn KHÔNG phải fine-tune theo dự án.

→ Mục tiêu thật của bạn ("nhớ dự án sâu") đạt được bằng **§1 (memory)**, không bằng training.

---

## 3. Custom command + workflow automation trên đúng base omp

`omp://slash-command-internals.md` + `omp://config-usage.md`:
- **Native slash-command** quét `<cwd>/.omp/commands/*.md` (project, prio 100) + `~/.omp/agent/commands/*.md` (user). **Đây chính là cách `/gs` deploy** (`ensure_gs_command`) → su-code **đã dùng đúng**.
- Điểm custom khác (mạnh hơn, cho automation cao): **extensions** (`extensions/`, prio 90, TS) · **hooks** (`hooks/pre|post/*`, `omp://hooks.md`) · **custom-tools** (`tools/*`, `omp://custom-tools.md`) · **MCP** (`~/.omp/agent/mcp.json`).
- **"omp update không conflict":** su-code chỉ ghi vào **config dirs của omp** (`.omp/commands`, `~/.omp/skills`, `mcp.json`) — KHÔNG patch core omp → update an toàn (`omp://config-usage.md` §1, §6). **Đang đúng base.** ✓
- Nâng automation: cân nhắc **hook `agent_start`** (seed/recall) + **custom-tool** cho thao tác lặp thay vì để agent gõ shell. Đó là cách "automation cao" mà omp cho phép — plan riêng nếu cần.

---

## 4. gstack vs gsd-pi vs su-code — "agent team đâu?"

- **gstack KHÔNG có team agent tự động.** Nó = 23 slash-command (mỗi cái 1 persona) + 8 power-tool; "12 parallel workers" trong README là **bạn tự mở 12 phiên Claude Code**, gstack không tự điều phối. → cảm nhận của bạn đúng.
- **gsd-pi** = automation cao (milestone/slice/task · auto mode · worktree · `.gsd/` memory) — chạy local CLI riêng (`gsd`).
- **su-code** đã hấp thụ cả hai: `/gs auto` (≈ gsd-pi auto/slice/worktree) + team THẬT qua **omp `task` subagents + `irc`** (không phải persona giả). Team thật sống ở `task` (explore/plan/reviewer/designer/oracle/...) — `omp://tools/task.md`, `omp://task-agent-discovery.md`, `omp://tools/irc.md`.
- Muốn "custom được như gsd-pi": dùng đúng các điểm omp ở §3 + `/gs` handshake (v0.24.0) — KHÔNG cần clone gsd-pi engine.

---

## 5. Submodule auto-pull — đính chính + thiết kế đúng

- **Hiện trạng:** `reference/gstack` + `reference/gsd-pi` = submodule (pinned SHA, deinit). Skill sources (addyosmani, ponytail) = **clone qua `external.rs`** (không submodule). feynman đã cắt.
- **Ngộ nhận:** git submodule **PIN một SHA**; muốn mới nhất phải `git submodule update --remote` (thủ công). **Không** "auto-pull liên tục". Auto-latest của skill = `8sync harness up --pull` / `8sync skill update` (re-pull HEAD theo `skills.toml`) — **đã có**.
- **Thiết kế đúng:**
  - **Reference repos để đọc** (gstack, gsd-pi, **agent-reach**): submodule (pin + `git submodule update --remote --merge` khi muốn refresh) **HOẶC** đơn giản `read https://github.com/...` on-demand (0 disk, luôn HEAD). Token-lean hơn → ưu tiên on-demand đọc.
  - **Skill repos feed harness** (addyosmani, ponytail): giữ qua manifest + `harness up --pull` (đã auto-latest). Đừng submodule (chống lại auto-latest + phình repo).
- **agent-reach:** thêm làm **skill** (`8sync skill add https://github.com/Panniantong/agent-reach`) — nó có `agent_reach/skill/SKILL.md` + MCP (`integrations/mcp_server.py`). Bổ trợ research/`last30days`. (Cài CLI cần Python/pip — tech-gated, không bundle.)

---

## 6. eval team % + đo chất lượng

- **Đã code:** `harness eval` in `score: N/M passed (X%)` (`eval.rs:114`). 3/3 = **100%**.
- **Giới hạn (thật):** đây là **3 fixture cố định** (fix-failing-test/add-fn-with-test/locate-symbol) = SIGNAL loop chạy được, model+network, non-deterministic — **KHÔNG** = "team tốt bao nhiêu % cho DỰ ÁN NÀY".
- **Muốn % theo dự án thật:** cần (a) fixtures lấy từ chính repo (bug thật đã fix, symbol thật) + (b) rubric chấm (build/test/lint pass · doc-hygiene · regression) → % tổng hợp. = feature mới `harness eval --project` (đề xuất, chưa làm).
- `harness bench` đo token/KV-cache (khác trục: hiệu suất context, không phải chất lượng).

---

## 7. Plan thực thi (ưu tiên · ponytail-gated)

| # | Việc | Khả thi | Touch point |
|---|---|---|---|
| P1 | **eval `%`** | ✅ DONE | `eval.rs:114` |
| P2 | **Bật Mnemopi** (deep recall/không quên) qua `8sync harness` + báo ở `doctor` | ✅ cao, reversible | `harness/*` ensure `~/.omp/agent/config.yml` block §1; `doctor.rs` |
| P3 | **agent-reach làm skill** (tech-gated) | ✅ | `8sync skill add` / external.rs |
| P4 | **Submodule-ref policy** (reference = submodule/on-demand; skill = manifest auto-latest) + thêm agent-reach vào reference | ✅ | `.gitmodules` (tùy), doc |
| P5 | **`harness eval --project`** (% theo dự án thật) | ⚠️ feature lớn | `eval.rs` + project fixtures |
| P6 | hook `agent_start` / custom-tool cho automation cao | ⚠️ tùy nhu cầu | `~/.omp/agent/hooks/`, `tools/` |
| — | ❌ GGUF fine-tune / training command | KHÔNG khả thi | — (đính chính §2) |
| — | ❌ Full Windows/macOS port | ngoài scope | — (§0) |

**P2 là đòn bẩy lớn nhất** cho đúng điều bạn mô tả ("ý thức sâu, không quên"). Tradeoff token (~5k recall/phiên) — cần bạn chốt vì vừa tối ưu token.

---

## 8. Một dòng

"Không quên + ý thức sâu" = **bật Mnemopi memory + cbm + spine** (P2), KHÔNG phải GGUF/fine-tune (§2 không khả thi). Team thật = omp `task`+`irc` (gstack không có). su-code đã ở **đúng base omp** để custom command/automation mà không conflict update; submodule auto-pull là ngộ nhận — skill đã auto-latest qua manifest.
