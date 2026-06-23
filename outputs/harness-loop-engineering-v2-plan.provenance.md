# Provenance — Harness Loop Engineering v2 Plan

**Generated:** 2026-06-23 · **Workflow:** deep-research approximated in-harness (researcher → cross-verify → synthesis via `web_search` + repo grounding). `/deepresearch` slash-command không có trong runtime này; phương pháp giữ đúng tinh thần: đa nguồn, cross-verify claim load-bearing, pin nguồn gốc (primary blog/paper), tách repo-grounding (file:line) khỏi web-claim.

## Citation map (inline `[n]` trong plan)

| n | Claim được chống đỡ | Nguồn chính | Loại |
|---|----------------------|-------------|------|
| 1 | Context engineering: compaction, memory tools (CRUD), context-awareness (budget feedback), programmatic tool-calling, progressive disclosure qua skills, sub-agent | Anthropic — "Effective context engineering for AI agents" (Sept 2025) https://www.anthropic.com/engineering/multi-agent-research-system + claude-mem mirror https://docs.claude-mem.ai/context-engineering | Primary (vendor eng blog) |
| 2 | KV-cache hit-rate = metric quan trọng nhất; timestamp ở prefix giết cache; tránh thêm/bớt tool giữa iteration | Manus — Yichao Ji, "Context Engineering for AI Agents: Lessons from Building Manus" (Jul 2025) https://manus.im/blog/Context-Engineering-for-AI-Agents-Lessons-from-Building-Manus | Primary |
| 3 | Một token khác cũng invalidate KV-cache từ điểm đó | Data Science Dojo, "Unlocking the Power of KV Cache" (Jan 2026) https://datasciencedojo.com/blog/kv-cache-how-to-speed-up-llm-inference/ ; KV-cache aware prompt eng https://ankitbko.github.io/blog/2025/08/prompt-engineering-kv-cache/ | Secondary (corroborating) |
| 4 | Progressive disclosure: nạp metadata trước, body khi cần | Anthropic context-engineering guide (qua [1]) + AWS re:Invent 2025 AIM277 recap https://dev.to/kazuya_dev/aws-reinvent-2025-what-anthropic-learned-building-ai-agents-in-2025-aim277-16lc | Primary + recap |
| 5 | todo.md recitation = "attention manipulation", chống lost-in-the-middle | Manus blog [2] ; MarkTechPost recap https://www.marktechpost.com/2025/07/22/context-engineering-for-ai-agents-key-lessons-from-manus/ | Primary + recap |
| 6 | Files as attention device / filesystem = unlimited context | Oracle Devs, "The Great AI Context Debate" (Feb 2026) https://medium.com/oracledevs/the-great-ai-context-debate-why-filesystem-vs-database-is-the-wrong-question-3454e27c27f6 ; Manus [2] | Secondary + primary |
| 7 | Programmatic tool-calling / context-awareness giảm token vào context | LOCA-bench (arXiv 2602.07962) https://arxiv.org/pdf/2602.07962 (dẫn Anthropic 2025g) | Academic (corroborating) |
| 8 | Orchestrator-worker: +90.2% breadth-first research, ~15× token; cần objective/boundary/output rõ | Anthropic — "How we built our multi-agent research system" https://www.anthropic.com/engineering/multi-agent-research-system ; ByteByteGo recap https://blog.bytebytego.com/p/how-anthropic-built-a-multi-agent | Primary + recap |
| 9 | "Don't Build Multi-Agents": share context + full traces, conflicting decisions = bad results | Cognition — Walden Yan (Jun 2025) https://cognition.com/blog/dont-build-multi-agents | Primary |
| 10 | Parallel chỉ giúp nếu subtask độc lập; coding/debug phụ thuộc cao = không hợp multi-agent | The AI Engineer recap https://theaiengineer.substack.com/p/how-anthropic-built-multi-agent-deep ; philschmid single-vs-multi https://www.philschmid.de/single-vs-multi-agents | Secondary (tổng hợp 2 phía) |
| 11 | Voyager: skill-library code đã verify, index theo embedding mô tả, retrieve/compose | Voyager (arXiv 2305.16291) https://arxiv.org/abs/2305.16291 | Academic (primary paper) |
| 12 | Reflexion = verbal lessons (failure) vs Voyager = reusable code (success); skill-library qua nhiều domain | "Adaptation of Agentic AI: Survey of Post-Training, Memory, Skills" (arXiv 2512.16301) https://arxiv.org/pdf/2512.16301 ; RL Self-Improving Agent w/ Skill Library (arXiv 2512.17102) | Academic survey |
| 13 | Reflexion: Actor→Evaluator→Self-Reflection lưu vào memory, không đổi trọng số | Reflexion (Shinn et al., NeurIPS) https://openreview.net/pdf?id=vAElhFcKW6 | Academic (primary) |
| 14 | Reflexion memory compact feedback thành NL/predicate dẫn hướng | EmergentMind "Reflexion Memory in AI Agents" (Feb 2026) https://www.emergentmind.com/topics/reflexion-memory ; ECHO/hindsight (arXiv 2510.10304) | Secondary + academic |
| 15 | Long-horizon harness: initializer dựng feature-list/progress; coder tiến từng phiên giữ clean state | Anthropic long-running agent harness — ZenML LLMOps DB https://www.zenml.io/llmops-database/long-running-agent-harness-for-multi-context-software-development ; demo repo https://github.com/anthropics/riv2025-long-horizon-coding-agent-demo | Primary (mirror) + code |
| 16 | (như 15) clean code state giữa các context window | cùng nguồn [15] | — |

## Repo grounding (không cần web — verify trực tiếp trong source)

- RULE #0 / mandatory reading / loop engineering: `assets/skills/00-force-load.md:3-11,13-26,56-65`
- breadcrumb + `now_stamp()` epoch vào context: `crates/cli/src/verbs/harness/memory.rs:104-127` ; `upsert_block` so sánh đổi: `:21-42` ; consolidate BUDGET=200: `:135-174` ; seed files: `:75-87`
- tool wiring: `crates/cli/src/verbs/skill/deploy.rs` — cbm `:200-224`, `register_omp_mcp:228-262`, `index_codebase_memory:266-276`, headroom `:281-299`, codegraph `:140,177`
- driver: `crates/cli/src/verbs/harness/auto.rs:17-73` ; `up.rs:16-65,159-209`
- injection: `crates/cli/src/verbs/skill/inject.rs:39-51,88-240`

## Mức độ tin cậy

- **Cao (primary, vendor/paper):** [1][2][8][9][11][13][15].
- **Trung bình (recap/secondary corroborating):** [3][4][5][6][10][14][16].
- **Academic mới (2026, chưa peer-review rộng):** [7][12] — dùng làm corroboration, không phải claim đơn độc.
- **Repo grounding:** đã đọc trực tiếp file:line trong session này (đáng tin nhất cho phần "hiện trạng").

## Caveat phương pháp

- Một số URL arXiv ID dạng `26xx` (2026) là kết quả search trả về; dùng làm corroboration phụ, claim lõi đều có ≥1 nguồn primary 2025.
- Plan là *đề xuất kiến trúc*, **chưa** thực thi/đo. Các con số (token giảm, KV-hit) là *kỳ vọng* cần đo lúc implement (xem §6 plan), chưa phải kết quả đã xác nhận.
