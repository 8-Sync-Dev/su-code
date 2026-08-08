# Session, memory, compaction

## 1. Compaction triggers (compaction.md)

Six paths:
1. manual `/compact [instructions]`
2. **overflow recovery** — same-model assistant error detected as context overflow, newer than the
   last compaction. Context promotion tried first; then context-full compaction with
   `reason: "overflow"`, `willRetry: true`. **Handoff strategy is never used for overflow.**
3. **incomplete-output recovery** — same-model assistant message with `stopReason === "length"`.
   Handoff **is** allowed here.
4. **threshold maintenance** — after a successful turn whose adjusted tokens exceed
   `resolveThresholdTokens(...)`
5. **mid-turn maintenance** — before the next provider request at a safe tool-loop boundary when
   `compaction.midTurnEnabled !== false`. Mid-turn suppresses handoff session resets and falls back
   to context-full.
6. **idle** — `runIdleCompaction()`, `reason: "idle"`, never auto-continues

On success (post-turn) and `compaction.autoContinue !== false`, an agent-authored developer
auto-continue prompt from `prompts/system/auto-continue.md` is scheduled. Mid-turn never schedules
one (the core loop already owns the next request).

## 2. Strategies

| `compaction.strategy` | Behavior |
| --- | --- |
| `snapcompact` (**default**) | Local, deterministic. Discarded history is serialized, whitespace-collapsed, and printed onto model-aware PNG frames with bundled pixel fonts. **No model, no API key, no network** — safe for overflow recovery. **Requires a vision-capable current model** (`model.input` includes `"image"`), else falls back to context-full with a warning. |
| `context-full` | LLM summarization into a `CompactionEntry` |
| `handoff` | Generates a handoff document, starts a **new session**, injects it as a visible `custom_message` with `customType: "handoff"`. Writes **no** `CompactionEntry`. |
| `shake` | Inline local reduction: eligible tool results and large fenced/XML blocks replaced with recoverable `artifact://` references, protected recent-token window + minimum-savings threshold. Falls through to context-full when it cannot reclaim enough (idle shake excepted). |
| `off` | disabled |

Snapcompact detail: frame shape resolves from the **model id** (Claude `11on16-bw`;
Gemini/GPT/Codex `8on22-bw`; Kimi/GLM `8on16-bw`); `maxFrames` default `80` is an upper limit only;
both chronological edges stay verbatim text around a foveated (HQ/LQ/HQ) imaged middle; later
compactions re-render from bounded source text (`Archive.text`), not old PNGs. Keys:
`snapcompact.shape` (`auto`), `snapcompact.systemPrompt` (`none`|`agents-md`|`all`),
`snapcompact.toolResults` (`false`).

## 3. Thresholds and defaults (settings-schema)

```
compaction.enabled            true
compaction.strategy           snapcompact
compaction.midTurnEnabled     true
compaction.thresholdPercent   -1     # -1 = reserve-based default
compaction.thresholdTokens    -1     # >0 wins over percentage
compaction.reserveTokens      unset  # effective = max(16384, 15% of ctx window); small windows use the 15%
compaction.keepRecentTokens   20000
compaction.autoContinue       true
compaction.remoteEnabled      true
compaction.remoteEndpoint     undefined
compaction.remoteStreamingV2Enabled  true
compaction.v2RetainedMessageBudget   64000
compaction.handoffSaveToDisk  false
compaction.idleEnabled        false
compaction.idleThresholdTokens 200000
compaction.idleTimeoutSeconds 300
compaction.supersedeReads     true
compaction.dropUseless        true
branchSummary.enabled         false
branchSummary.reserveTokens   16384
contextPromotion.enabled      false
```

A positive `thresholdTokens` takes precedence over `thresholdPercent`; otherwise the reserve-based
threshold is used.

## 4. What survives a compaction — the rule that governs harness design

The **system prompt is rebuilt every request** and is never part of the compacted region. So
anything rendered into the system prompt survives compaction by construction:

- `SYSTEM.md` / `APPEND_SYSTEM.md` text
- discovered context files (`<repo-rules>`)
- always-apply rule bodies (`<generic-rules>`) — plus sticky `RULES.md`, which the docs describe as
  "re-attached near the current turn" so it keeps its hold across long sessions (context-files.md)
- the rulebook listing (`<domain-rules>`) and the skill name/description list
- the project/environment footer
- Memory Guidance block, when a memory backend is active

What does **not** survive: ordinary conversation turns before `firstKeptEntryId`, tool results in
the compacted region, and one-shot injections (magic-keyword notices, `/skill:<name>` injections,
TTSR `<system-interrupt>` messages, `before_agent_start` messages) — these are conversation
entries, not prompt blocks.

TTSR rules are a special case: they are **re-evaluated on every stream**, so their steering power is
unaffected by compaction even though the injected text is not preserved.

### Rebuild order (`buildSessionContext`)
1. latest compaction on the active path → one `compactionSummary` message
2. kept entries from `firstKeptEntryId` to the compaction point
3. later entries on the path
4. `branch_summary` entries → `branchSummary` messages
5. `custom_message` entries → `custom` messages

`compactionSummary` and `branchSummary` render through static templates
(`compaction-summary-context.md`, `branch-summary-context.md`) as **user** messages; `custom`
messages pass through as **developer** messages with raw content, no template.

### Cut-point rules
Only entries since the previous compaction are considered. Valid cut points: message entries with
roles `user`, `assistant`, `bashExecution`, `hookMessage`, `branchSummary`, `compactionSummary`;
`custom_message` entries; `branch_summary` entries. **Hard rule: never cut at `toolResult`.**
Non-message metadata entries immediately before the cut (`model_change`,
`thinking_level_change`, labels) are pulled into the kept region.
Split turns (cut not at a user-turn start) produce two summaries merged as
`<history summary>\n\n---\n\n**Turn Context (split turn):**\n\n<turn prefix summary>`.

### Pre-compaction pruning
- protect newest **40 000** tool-output tokens; require ≥**20 000** total estimated savings
- never blank a result below **50** tokens (`MIN_PRUNE_TOKENS`) — the `[Output truncated - N tokens]`
  placeholder costs ~8 tokens
- **never prune `skill` tool results, `read` results of `skill://` paths, or reads of the active
  plan reference file**
- useless-result elision (`compaction.dropUseless`, default on) blanks flagged results to
  `[Uneventful result elided]`; the flag never reaches provider wire formats and pairs are never
  removed from history (only blanked in place)

### File-operation context
Compaction tracks `read(path)` → read set, `write(path)`/`edit(path)` → modified set, and appends a
grouped, prefix-folded `<files>` tree with `(Read)`/`(Write)`/`(RW)` markers, capped at 20 files.
Legacy `<read-files>`/`<modified-files>` tags are stripped and re-appended, so old summaries
self-heal.

### Display transcript
Compaction no longer visually restarts the conversation. The TUI renders every path entry
chronologically with each compaction shown inline as `── 📷 compacted · ctrl+o ──`. Only the **LLM
context** resets at the boundary.

### Extension touchpoints
`session_before_compact` → `{ cancel: true }` or a full `{ compaction: CompactionResult }`.
`session.compacting` → `{ prompt?, context?: string[], preserveData? }` — `context` lines are
injected as `<additional-context>` entries into the summarizer input. `session_compact` fires after.

## 5. Session entry types (session.md)

`CompactionEntry`: `type: "compaction"`, `summary`, `shortSummary?`, `firstKeptEntryId`,
`tokensBefore`, `details?`, `preserveData?`, `fromExtension?`.
`BranchSummaryEntry`: `type: "branch_summary"`, `fromId`, `summary`, `details?`, `fromExtension?`.

`custom` entries are **opaque, non-LLM** records keyed by `customType`. `buildSessionContext` does
not turn them into model messages; subsystem replay code consumes specific `customType` values.
**Core reserves** names such as `tool_execution_start` and `session_exit`; extensions must use a
reverse-domain / package-qualified id or core replay logic may misread the data as lifecycle state.

`custom_message` entries **do** become `custom` (developer) messages with `customType`, `content`,
`display`, `details`.

## 6. Memory backends (memory.md)

`memory.backend`: **`off` (default)** | `local` | `hindsight` | `mnemopi`.

### `local`
Project-scoped summaries + lessons built by a background pipeline at startup (skipped for subagents
and non-persisted sessions). Phase 1 per-session extraction uses the `default` role; Phase 2
consolidation uses `smol` (fallback `default`, then current/first registry model). Outputs:
`MEMORY.md`, `memory_summary.md`, `skills/`. `learned.md` is maintained separately and never
overwritten by consolidation. Output is secret-redacted before writing. Phase 2 uses a lease +
heartbeat so concurrent processes don't double-run.

Injected at session start as a **Memory Guidance** block sharing
`memories.summaryInjectionTokenLimit` (default `5000`).

`memory://` URLs: `memory://root` (injected summary) · `memory://root/MEMORY.md` ·
`memory://root/learned.md` · `memory://root/skills/<name>/SKILL.md`.

Tuning keys (`memories.*`): `maxRolloutAgeDays` 30 · `minRolloutIdleHours` 12 ·
`maxRolloutsPerStartup` 64 · `threadScanLimit` 300 · `maxRawMemoriesForGlobal` 200 ·
`stage1Concurrency` 8 · `rolloutPayloadPercent` 0.7 · `phase1InputTokenLimit` 4000 ·
`fallbackTokenLimit` 16000 · `summaryInjectionTokenLimit` 5000 (+ lease/retry/heartbeat seconds).

`learn` with the local backend saves lessons to `learned.md`: newest-first, deduplicated,
secret-redacted, **capped at 100 entries**, content ≤2 000 chars, context ≤400 chars, injected
**starting with the next session** (a `learn` call does not mutate the active prompt-cache prefix).
`recall`, `retain`, `reflect`, `memory_edit` are **not** available for `local`.

### `hindsight`
Remote, bank-scoped. Default endpoint `http://localhost:8888`; `HINDSIGHT_*` env overrides beat
`hindsight.*` settings. Default scoping `per-project-tagged` (shared bank + project tag; recall
includes tagged + untagged global). `hindsight.autoRecall: true` recalls on the first model turn;
auto-retain every `hindsight.retainEveryNTurns` (3) user turns. Exposes `recall`, `retain`,
`reflect`; **not** `memory_edit`. Subagents alias the parent's client/bank/scope for explicit calls
but run no automatic recall/retention. Recall is injected as **background context, not
instructions**, and is also available as extra context during compaction.
`/memory clear` does **not** delete the server-side bank.

### `mnemopi`
Local SQLite. Adds `memory_edit` (`approval="read"`, `strict`, `loadMode="discoverable"`).

### Tool registration gates
| Tool | Requires |
| --- | --- |
| `learn` | `autolearn.enabled = true` **and** `memory.backend ∈ {hindsight, mnemopi, local}`. `loadMode="essential"`, dynamic approval (`write` for skill/local calls, `read` for memory-only) |
| `manage_skill` | `autolearn.enabled = true` (independent of backend). `loadMode="essential"`, `approval="write"` |
| `recall` / `retain` / `reflect` | `memory.backend ∈ {hindsight, mnemopi}`. `loadMode="discoverable"` |
| `memory_edit` | `memory.backend = mnemopi`. `loadMode="discoverable"` |

Subagents do **not** discover or auto-receive `learn` / `manage_skill`; they may use them when
their requested-tools/frontmatter list names them explicitly. Restricted tool lists are never widened.

`autolearn.enabled` F · `autolearn.autoContinue` F (on = one extra capture turn at stop) ·
`autolearn.minToolCalls` `5`.

## 7. Branch summaries

Tied to `/tree` navigation, not token overflow; gated by `branchSummary.enabled` (default `false`).
Budget = `model.contextWindow - branchSummary.reserveTokens` (16384); newest → oldest until spent.
Stored as `BranchSummaryEntry`. Hooks: `session_before_tree` (`{cancel?, summary?}`), `session_tree`.
