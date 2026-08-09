# base.md — canonical upstreams for encore-eino-go

Two upstreams layer in one Go codebase: **Encore** (infrastructure-as-code platform) owns services/DB/cron/pubsub/auth/secrets; **Eino** (ByteDance's Go LLM framework) owns LLM orchestration. This file distils only the load-bearing concepts — the things that break everything if wrong. It links out; it does not vendor a README.

---

## 1. Encore (Go SDK) — infrastructure platform

- **Docs:** https://encore.dev/docs  · **Go SDK reference:** https://encore.dev/docs/Go/  · **Repo:** https://github.com/encoredev/encore
- **Current stable:** **v1.57.x** (latest release **v1.57.13** at time of writing). The 8syncdev go-kit pins `encore.dev v1.57.5`. Encore.ts exists as a sibling runtime; this skill is **Go-first**.

### Load-bearing concepts

**Application = a directory with `encore.app`.** Minimal `encore.app`:
```json
{ "id": "my-app-id" }
```
`encore run` / `encore test` / `encore deploy` all operate from this directory. The `id` is the app's Encore Cloud identifier. Encore provisions local Postgres, infra, and the dev server from here.

**Service = a Go package marked `//encore:service`.** A service is the unit of deployment and the call boundary. An optional `initService() (*Service, error)` runs once at startup (load secrets, warm caches, bootstrap pools) before the first request. Services call each other through the **generated client** (`encore.gen.go`), not by URL — this is why the codebase call graph *is* the dependency map.

**API = an annotated function.** The `//encore:api` directive is the route declaration — there is no manual router registration. Encore generates the HTTP handler, request/response (de)serialization, and a typed Go client.
```go
//encore:api public method=POST path=/foo.Bar
func Bar(ctx context.Context, req *BarReq) (*BarResp, error)
```
Access levels (the field after `//encore:api`):
- `public` — callable by anyone, no auth.
- `auth` — gated by the auth handler; Encore injects the auth data into `ctx`.
- `private` — callable only from other Encore services, cron, or pubsub (not the public internet).
- `raw` — the handler receives raw `*http.Request`/`http.ResponseWriter` (SSE, file upload, websockets). Combine: `//encore:api auth raw method=GET path=/...`.

**`encore.gen.go` is generated — never hand-edit.** Encore writes it from your `//encore:api`/`//encore:service` annotations on every `encore run`/`build`/`test`. It contains the typed clients (`encore.gen` package), request/response types, and service constructors. It is **gitignored** in 8syncdev repos (regen-only). Changing an API signature regenerates it; all callers must then recompile against the new types. Treat a stale `encore.gen.go` as a build error, not a file to fix by hand.

**Typed SQL via `encore.dev/storage/sqldb`.** A named DB is a value: `var db = sqldb.Named("core_db")`. The name maps to an Encore-managed Postgres (local dev uses Encore's built-in instance; the connection string is infra config, not code). Queries are typed Go: `db.QueryRow(ctx, q, args...).Scan(...)`, `db.Exec`, `db.Query` (cursor). **Migrations** are versioned `.up.sql` files in `<service>/migrations/`, applied in lexical/numeric order. Schema change = a new migration file, never an in-app `ALTER`. Local: `encore run` applies migrations automatically.

**CRON — `encore.dev/cron`.** `cron.NewJob("id", cron.JobConfig{Every: 1*cron.Minute, Endpoint: MyFunc})`. `Endpoint` must be a `//encore:api private` func Encore can call. `Every` accepts `cron.Minute`/`cron.Hour`/`cron.Day`/`cron.Week`/`cron.Month` or a crontab string.

**PubSub — `encore.dev/pubsub`.** Publish: `topic := pubsub.NewTopic[*Evt]("name", pubsub.TopicConfig{DeliveryGuarantee: pubsub.AtLeastOnce})`; `topic.Publish(ctx, ev)` returns a message ID. Subscribe: `pubsub.NewSubscription(topic, "sub-id", pubsub.SubscriptionConfig[*Evt]{AckDeadline: d, Handler: fn})`. `DeliveryGuarantee` is `AtLeastOnce` or `AtMostOnce`. **`AckDeadline` must exceed the handler's worst-case latency** — a handler still running at deadline is redelivered. Topics/subscriptions are declared in infra config for non-local environments.

**Secrets — `encore.dev/secrets`-style `var secrets struct{…}`.** Declare `var secrets struct { APIKey string }`; set values out-of-band via `encore secret set --type prod,dev,pr,local APIKey`. Read `secrets.APIKey` at runtime; an unset secret is the empty string (handle gracefully so startup never hard-fails on a missing optional secret). Never log secret values. For multi-valued secrets, store JSON and unmarshal in `initService`.

**Auth handler — `//encore:authhandler`.** Exactly one per app. `func(ctx, *AuthParams) (auth.UID, *UserData, error)` — Encore fills `AuthParams` from headers/cookies (struct tags `header:`/`cookie:`). Runs before **any** `//encore:api auth` endpoint. On failure return `&errs.Error{Code: errs.Unauthenticated, …}` — Encore treats other codes as a hard abort even on `public` endpoints. `auth.UID` + your custom `*UserData` are injected into `ctx` for authorized handlers.

**Errors — `encore.dev/beta/errs`.** Return `&errs.Error{Code: errs.<Code>, Message: …}` to map to HTTP status (`Unauthenticated`→401, `NotFound`→404, `ResourceExhausted`→429, `Internal`→500). A bare `error` becomes 500.

**Logging — `encore.dev/rlog`.** Structured logger tied to the request trace: `rlog.Info/Warn/Error("msg", "key", val)`.

**Lifecycle:** `encore run` (local dev server + infra), `encore test` (runs `go test` with real local DB/infra provisioned), `encore build` (compile), `encore deploy` (push to Encore Cloud or self-hosted). Env coupling: secrets, DB URLs, pubsub topics/subscriptions, and cron are resolved per-environment (local/dev/pr/prod) from Encore's infra config, not from `.env` files.

---

## 2. Eino — LLM application framework (Go)

- **Docs:** https://www.cloudwego.io/docs/eino/  · **Repo:** https://github.com/cloudwego/eino  · **Components (eino-ext):** https://github.com/cloudwego/eino-ext  · **Examples:** https://github.com/cloudwego/eino-examples
- **Current stable:** **v0.9.x** (latest release **v0.9.13** at time of writing). The 8syncdev go-kit pins `github.com/cloudwego/eino v0.9.5` + `eino-ext` openai `v0.1.13` / gemini `v0.1.32`.

### Load-bearing concepts

Eino is "LangChain for Go": components + composition + an Agent Development Kit (ADK). The model is **components → compose into a graph/agent → stream or generate**.

**Components are interfaces; implementations live in `eino-ext`.** Core component packages under `github.com/cloudwego/eino/components/`:
- `model` — `model.ChatModel` and the tool-calling-capable `model.ToolCallingChatModel`. Implementations: `eino-ext/components/model/openai` (OpenAI-compatible: Mistral, z.ai, 9Router, local), `.../gemini`, `.../ark`, `.../ollama`.
- `tool` — `tool.BaseTool` + `tool.InvokableTool`; build from plain Go funcs with `components/tool/utils.InferTool(name, desc, fn)`.
- `Retriever`, `Embedder`, `ChatTemplate`, `Indexer`, `Loader`, etc.

**Schema is `github.com/cloudwego/eino/schema`.** `schema.Message` (role/content/tool calls/usage), `schema.Role`, `schema.StreamReader[T]` (the lazy streaming reader). Usage/token accounting lives on `m.ResponseMeta.Usage`.

**Composition — `github.com/cloudwego/eino/compose`.** Wire components into a runnable graph: `compose.NewChain`/`NewGraph`, **lambda nodes** (`compose.AddLambda` — inline Go funcs as graph steps), branch nodes, and `compose.ToolsNodeConfig` to attach a toolset to a model. Graphs run standalone or can be exposed *as a tool* to an agent.

**Agent — `github.com/cloudwego/eino/flow/agent/react`.** The ready-made ReAct loop:
```go
agent, err := react.NewAgent(ctx, &react.AgentConfig{
    ToolCallingModel: cm,                     // a model.ToolCallingChatModel
    ToolsConfig: compose.ToolsNodeConfig{
        Tools: toolset,
        ExecuteSequentially: true,            // false = parallel (eino default)
    },
    MaxStep:            maxStep,
    ToolReturnDirectly: map[string]struct{}{}, // tool names that end the loop
})
```
`agent.Generate(ctx, msgs) (*schema.Message, error)` (one shot) or `agent.Stream(ctx, msgs) (*schema.StreamReader[*schema.Message], error)` (token-by-token). The loop stops on a content-only message by default.

**Callbacks** (`github.com/cloudwego/eino/callbacks`) hook every component event — token, tool-call start/end, finish — for tracing/telemetry without touching component code. Pass them via the agent/chain config or context.

**Streaming is pull-based.** A `StreamReader[T]` yields chunks via `Recv()`; relay them (e.g. to SSE frames) until EOF. Backpressure = how fast you consume.

**Why Eino composes cleanly inside Encore:** an Eino agent is just a Go object. You construct it in an Encore service's `initService` (or lazily per turn from a DB-resolved config), call `Generate`/`Stream` from an `//encore:api` handler, and let Encore own everything else (auth, DB pool, secrets for the model key, pubsub for async turns, SSE via a `raw` endpoint). The LLM turn is one function call; Encore is the host.

---

## How the two layer together (the seam)

| Concern | Owner | Mechanism |
|---|---|---|
| HTTP route, auth, request lifecycle | Encore | `//encore:api` + `//encore:authhandler` |
| LLM key / model config | Encore secrets + DB → resolved `Config` | `var secrets`, `initService`, `sqldb` |
| Build the ChatModel | Eino (`eino-ext` component) | `openaimodel.NewChatModel` / `geminimodel.NewChatModel` |
| Build tools | Eino (`utils.InferTool`) | Go funcs → `tool.BaseTool` |
| Orchestrate the turn | Eino (`flow/agent/react`) | `react.NewAgent` → `Generate`/`Stream` |
| Persistence / RAG vectors | Encore (`sqldb`, `pgvector`) | typed SQL + migrations |
| Async long turns | Encore PubSub | entrypoint ACKs + publishes; worker calls the agent |
| Telemetry / tracing | Encore `rlog` + Eino callbacks | both attach to the same `ctx` |
