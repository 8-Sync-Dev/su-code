# Patterns — real, from 8syncdev's own repos

Every pattern below is mined from a repo under the `8syncdev` account (read via
`gh api`). Cited as `repo:path`. These are the opinionated extension of the
canonical base — the *specific* shape these decisions take in production here.

Repos: `8syncdev/ai-router-hub` (hosted AI gateway), `8syncdev/agentic-cloudgo-v1`
(agentic Go service), `8syncdev/8syncdev-pro-v2` (AI tutor + IELTS, Next.js
monorepo), `8syncdev/zus-work` (control-plane, Encore Go).

---

## A. Per-member key + metering (THE gateway invariant)

**`8syncdev/ai-router-hub:src/lib/gateway.ts`** — the canonical example. The
request pipeline is one explicit chain, in order:

1. `authenticate(header)` — strips `Bearer `, looks up the row by
   `key_hash = hashApiKey(token)` (the **raw key is never stored**, only its
   hash). Returns a `Caller { keyId, memberId, email, tokenLimit }`.
2. **Revocation is a status flip, never a delete:** the comment is load-bearing
   — *"Revocation is a status flip, never a delete: the usage ledger must keep
   pointing at a real member after access is taken away."* A revoked key →
   `403 key_revoked`; a suspended member → `403 member_suspended`.
3. `enforceQuota(caller)` — `tokensToday(memberId)` sums `input_tokens +
   output_tokens` for the member since `date_trunc('day', now())`; if
   `used >= tokenLimit` → `429 quota_exceeded` with the reset boundary in the
   message.
4. `pickProvider(model)` — **deliberately dumb:** *"the first active provider
   that claims the model, else the first active provider with no claim list.
   Anything cleverer (weights, health, failover) would be a routing policy
   nobody has asked for yet, and a wrong guess here is expensive to unwind."*

- `errorBody()` returns **OpenAI-shaped** `{error:{message,type,code}}` —
  "because every client already knows how to read it."
- Provider secrets are stored `secret_enc` and resolved via `decryptSecret()` —
  the **upstream key never leaves the proxy**; members only ever hold a
  minted per-member key.

**`8syncdev/ai-router-hub:src/lib/auth.ts`** — the admin/session side:
`credentialComplaint()` rejects `username == password` and `< 12` chars as a
**boot error** (the original ask was "admin account is probably
username=password" — a real failure mode), `ensureAdmin()` bootstraps from env
on first use and never overwrites, sessions are signed cookies with a TTL.

**`8syncdev/agentic-cloudgo-v1:be/iam/auth_gate.go`** + **`login_throttle.go`**
— the control-plane analogue: auth gate + login throttling on the Go side.

> **Pattern:** authenticate → enforce quota → resolve provider, in that order,
> upstream keys vaulted, revocation = status flip, errors OpenAI-shaped.

---

## B. Model abstraction boundary + multi-provider routing (Go)

**`8syncdev/agentic-cloudgo-v1:be/engine/models/factory.go`** — the provider
abstraction. Dispatch is by **`llm_providers.api_flavor` (DB-driven), NOT by
provider name:** flavor `openai` → eino-ext openai component with the
provider's `base_url` (Mistral / z.ai / 9Router are OpenAI-compatible);
flavor `gemini` → eino-ext gemini component. Per-(provider,model,key) factory
cache; `WithTools` returns a fresh bound copy per call site.

Provider-specific quirks are **keyed, not branched:** z.ai GLM needs
`thinking:{type:"disabled"}` (without it, reasoning burns the whole budget
and returns **empty** content); gpt-5.x on 9router needs
`reasoning_effort=none` (default high effort → 2–3× slower). Both via
`ExtraFields`. A **60s call timeout** caps every chat-completion (eino-ext
openai defaults to *no* timeout — a hung upstream would hang the turn worker
forever).

**`8syncdev/agentic-cloudgo-v1:be/engine/models/models.go`** — `ResolvedModel`
(everything the factory needs: provider, flavor, base_url, key, temp,
max_tokens, thinking level, source primary|fallback). **Key-pool with a 429
circuit breaker:** `keyCacheTTL = 5s` — "worst-case hammer a bad key ≤5s before
re-query." `deadModels` marks a model bench-expired when the provider can't
serve it. **All values come from `agent_db` rows — never Go constants** (port
of the TS `llm.service.ts`).

**`8syncdev/8syncdev-pro-v2:docs/adr/ADR-008-llm-multi-router.md`** — the
*decision* behind a multi-LLM router (`@8sync/ai/router.ts`): route by
`task + complexity hint + tenant override`, because a single-LLM (Claude
Sonnet for everything) costs ~3× budget ($0.015 vs target $0.005/turn) and
serves the wrong model per task (Gemini Flash for classification is ~100×
cheaper than Sonnet; Gemini > Claude for Vietnamese). Vendor diversity avoids
one provider outage taking everything down. **The explicit cost of routing:
"golden eval mỗi PR" — a routing bug serves the wrong model and is a quality
regression, so eval is mandatory on every change.**

> **Pattern:** one provider-agnostic `ChatModel` interface; resolve
> task→model→key from DB (not code constants); route by task complexity;
> key-pool with circuit breaker; eval-gate the routing.

---

## C. RAG — embed, hybrid retrieve, rerank (Go + pgvector)

**`8syncdev/agentic-cloudgo-v1:be/kb/retrieve.go`** — **GraphRAG** pipeline:
hybrid seed (`pgvector` dense **+** `tsv` BM25 sparse) → promote child nodes
to canonical parent → 1-hop expansion → **one card per canonical entity**
(dedup of overlapping tall-block rows). All knobs are runtime-config
(`kb.k_seed=8`, `kb.k_bm25=4`, `kb.sim_floor=0.62`) read from DB. Result is
**epoch-cached** keyed by `(bot, query, kinds, topK)` — cache invalidates on
KB epoch change.

**`8syncdev/agentic-cloudgo-v1:be/kb/rerank.go`** — **LLM rerank, fail-open.**
If `kb.rerank_pool > topK`, a small LLM re-scores candidates against the exact
query and cuts to topK. **Knob-gated, default OFF (pool=0).** On
resolve/model/parse/timeout error: *"giữ nguyên thứ tự graph-score và cắt
TopK như cũ, không bao giờ chặn lượt chat"* — keep graph order, never block the
turn. GraphRAG = recall; rerank = precision.

**`8syncdev/agentic-cloudgo-v1:be/kb/embed.go`** — `EmbedPending` embeds every
active node missing an embedding, in batches (`kb.embed_batch`, default 32),
pgvector literal `[x,y,…]`. `ReembedAll` is **idempotent + self-healing** —
after an embedder/dimension swap nulls the vectors (migration 8), it only
touches `embedding IS NULL` rows, safe to call repeatedly.

**`8syncdev/8syncdev-pro-v2:docs/adr/ADR-006-vector-pgvector-first.md`** — the
**migration-triggered** vector-store choice: **pgvector in Encore SQLDB Phase
1–2 → Pinecone Serverless Phase 3.** Drivers: 1 DB / 1 backup / transactional
insert (`BEGIN; INSERT course; INSERT embedding; COMMIT;`) / JOIN with
business tables / query 10–30ms p95 in-VPC. **Triggers to migrate:** ≥ 10M
vectors **or** query p95 > 200ms — measured weekly via tracing metrics.
pgvector HNSW `m=16 ef_construction=64`, recall@10 ≥ 0.95. Embedding model:
`text-embedding-3-large` (3072-d, MRL truncation).

> **Pattern:** hybrid retrieve (dense+sparse) → canonical dedup → 1-hop expand;
> rerank fail-open and knob-gated; pgvector-first with an explicit, measured
> migration trigger, not a religion.

---

## D. Agent loop — streaming, guardrails, per-call tracing

**`8syncdev/agentic-cloudgo-v1:be/engine/pipeline_stream.go`** — **SSE
streaming.** `StreamEvent` frames are typed `rag | tool | token | done |
error`; each carries the relevant slice (token text, tool-call record, RAG
docs, final result). Pre-grounding note goes **last** in the system prompt
(recency) to stop the model re-calling retrieval tools for bytes already in
the prompt — *"paying a whole ReAct round-trip for bytes already in prompt."*
`redundantWhenGrounded` disables `searchProduct`/`searchFaq` once grounded.

**`8syncdev/agentic-cloudgo-v1:be/engine/model_attempts.go`** — **trace per
upstream call, not per turn.** `generateWithModelTrace` wires Eino's native
`ChatModel` callbacks (`ModelCallbackHandler` OnStart/OnEnd/OnError) via
`react.BuildAgentCallback` so *"one record means one provider request inside
the ReAct loop, not one outer Agent.Generate."* Each `ModelAttempt` records
model name, key id, status, ms. The **repair pass runs as ONE plain model
call** — routing it back through `react.Agent` re-arms the whole ToolsNode
loop (+10.1s measured).

**`8syncdev/agentic-cloudgo-v1:be/engine/guardrails/guardrails.go`** —
deterministic in-loop checks, **LLM-first discipline:** *"ONLY numeric/state
checks against ground truth (actual tool calls, working memory, envelope
self-report) — no prose-regex sanitizers."* `Claims` are validated against
**ACTUAL tool calls**, never self-report ("claims derive from ACTUAL tool
calls — never from self-report"). A violation triggers **one** retry with
coaching feedback, then a safe fallback.

**`8syncdev/agentic-cloudgo-v1:be/chat/trace_api.go`** — `TraceTurn` is the
full per-turn provenance payload (the dashboard debug view): model name,
api_key_id, pipeline level, input/reply text, `tool_calls`, `rag`,
`guardrail_violations`, `usage`, `latency_ms`, delivery timings,
first-meaningful/final accepted ms. This is the **evidence** the eval gate and
cost/quality dashboards run on.

> **Pattern:** stream typed SSE frames; trace every upstream call (not the
   turn); guardrails check ground truth with one retry + fallback; the trace
   record is the single source of truth for cost/quality/eval.

---

## E. Control-plane + topology

**`8syncdev/zus-work:backend`** — control-plane Encore Go: `gate/` (auth
provider: users, login/register, `//encore:authhandler`), `core/` (health,
identity, product, **native** bridge → Rust daemon), `shared/` (crypto
JWT/scrypt, ulid, pagination). It orchestrates **app lifecycle** (registry,
health, system info) by bridging to a native daemon over HTTP. The spine
lives in `su-code/` (STATE / KNOWLEDGE / DECISIONS / PLAYBOOKS).

**`8syncdev/ai-router-hub:backend-go/gate`** — Encore control-plane skeleton:
`auth.go` (`//encore:authhandler` parsing Bearer → `errs.Unauthenticated`
401), `envelope.go` (`Response{Success,Message,Result}` — "khớp convention
mind0/zus"), `whoami.go` (`//encore:api auth`). The Go gateway reuses the same
envelope convention as zus-work — **one control-plane shape across repos.**

**`8syncdev/agentic-cloudgo-v1:be/gateway/router.go`** — frozen legacy surface
as **raw wildcard endpoints with internal dispatch** (Encore's typed router
forbids static+sibling param paths, so each frozen prefix is one wildcard +
internal switch). *"Handlers are thin ADAPTERS over the new core — wire shapes
frozen, internals free."* Auth via `authz.GuardRaw`.

**`8syncdev/8syncdev-pro-v2:docs/01-ARCHITECTURE.md`** — the fullstack AI
topology: Turborepo monorepo, **Encore.ts** declarative infra (DB/PubSub/Cron/
Secrets/Cache in code, auto TS client for FE), **pgvector in Encore SQLDB**,
multi-LLM routing by cost, **"eval suite for AI"** as a first-class deliverable
("Mỗi feature ship ra ở chất lượng production: typed E2E, A11y AA, Lighthouse
95+, eval suite cho AI"). AI features spread across `services/chat` (RAG
tutor), `services/ielts` (rubric retrieval), `services/course`, `services/coding`.

> **Pattern:** control-plane = Encore (Go or TS), declarative infra, one
   envelope convention; legacy surfaces are thin frozen adapters over a free
   core; AI features are fullstack (FE → BE → model) with an eval suite as a
   ship gate.
