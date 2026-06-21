# Research Brief — Production-grade self-improving harness for `8sync` (real-world)

**Date:** 2026-06-21 · **Method:** deep-research (multi-source web + repo grounding, verifier pass on load-bearing claims) · **Scope:** validate & harden the harness self-learning system (memory portability, `skill update`, `harness up --pull/--commit`, the engineering loop) against 2026 best practices. Sources + verification in the `.provenance.md` sidecar; inline `[n]` map there.

---

## TL;DR (what the evidence changes)

The features we shipped are directionally correct (durable file memory + git portability + skill auto-update), but 2026 research surfaces **5 concrete real-world gaps**. Priority order:

| # | Gap | Evidence | Fix (P0/P1/P2) |
|---|---|---|---|
| 1 | **Injected context is too fat** — `harness init` writes a ~38-line on-demand skill-description wall into root `AGENTS.md`/`CLAUDE.md` | Gloaguen 2602.11988: context files *reduce* success, **+20% cost**; LLM-generated ones mostly **duplicate docs** `[1]`. Context-rot lit: "remember everything → remember nothing" `[7][9]` | **P0** trim injected block to names-only; lean budget + `doctor` lint |
| 2 | **`skill update` has no version pinning / lockfile** — clones `--depth 1` HEAD, non-reproducible | Claude Code plugin marketplace: semver + **SHA pin = reproducible**; "dependency tracks latest → upstream changes under you without warning" `[10][11]` | **P0** record resolved SHA in `skills.toml`; `add#ref` to pin |
| 3 | **`harness up --commit` is a secret-leak vector** | GitGuardian 2026: Claude-Code commits leak secrets **3.2% vs 1.5%** baseline; `.gitignore` insufficient; **gitleaks pre-commit = 5-min high-ROI** `[12][13]` | **P0** scan before auto-commit; `init` offers gitleaks hook; `doctor` checks it |
| 4 | **Memory grows unbounded** — `KNOWLEDGE.md`/`DECISIONS.md` never consolidated | 4-lever consolidation (importance/merge/decay/eviction); context rot from stale accumulation `[8][14]` | **P1** size budget + consolidation pass |
| 5 | **"Loop" persists context but never *verifies*** | Reflexion = Generate→Critique→Refine; **verifiability constraint**: no improvement beyond what's objectively verifiable `[15][16]` | **P2** tag learnings validated-by-test; reflect only where a verifier exists |

---

## Findings

### F1 — Context files help ONLY when minimal; auto-generated bloat hurts
The keystone study (Gloaguen et al., *Evaluating AGENTS.md*, arXiv **2602.11988**, ETH Zurich + LogicStar.ai; AGENTBENCH = 138 real Python SWE tasks) found across 4 frontier models that repository context files **reduce task success vs no context and raise inference cost >20%**; developer-written files give ~+4% only when minimal/precise, still ~19% cost `[1]`. Mechanism: **LLM-generated files mostly restate existing README/docs** — with docs removed they helped +2.7% `[1]`. Corroborated by the AGENTS.md "command-first, minimal, schema-free" consensus `[3][4][6]` and the context-rot literature: performance degrades as input grows even within the window; "agents that remember everything remember nothing useful" `[7][8][9]`.

**→ su-code:** `harness init` injecting a full on-demand skill-description block into root `AGENTS.md`/`CLAUDE.md` is precisely the redundant auto-bloat the study penalizes (the descriptions already live in each `SKILL.md`). The lean always-on chain (8 names) is fine; the on-demand wall is not. Agent Skills are *designed* for progressive disclosure — name+description metadata is enough to trigger; the full text loads on demand `[2,skills][3,skills]`.

### F2 — Distribution standard is pin-by-SHA + lockfile, not "always latest"
Claude Code's plugin marketplace (the closest production analog to our skill registry) ships **version pinning (semver), SHA pins for reproducible installs, version-range constraints, and lockfile auto-install**; SHA pinning is the **recommended approach for production**, and "by default a dependency tracks latest, so an upstream release can change it under you without warning" `[10][11]`. Custom Skills also **do not sync across surfaces** — each surface manages its own copy `[2,skills]`.

**→ su-code:** `skill update` re-clones HEAD with no record of the resolved commit ⇒ two machines can end up on different skill versions; updates are unauditable and irreproducible. Our vendored `agents/skills/` is actually a *content* lockfile (strongest form) for the project copy, but the global `~/.omp/skills` copy and the registry have no pin. Recording the resolved SHA closes this.

### F3 — AI-assisted auto-commit materially increases secret leakage
GitGuardian's State of Secrets Sprawl 2026: **28.65M new hardcoded secrets** hit public GitHub in 2025 (+34% YoY); **Claude-Code-assisted commits leaked at 3.2% vs a 1.5% baseline**; AI-service keys +81% `[12]`. `.gitignore` "does nothing to stop AI tools from reading those files and reproducing contents in code that *does* get committed" — the recommended control is a **gitleaks pre-commit hook** (regex + entropy, ~ms/commit, 5-min setup) backed by CI scanning `[12][13]`.

**→ su-code:** `harness up --commit` (and `--timer` looping it) automates commits — the exact risk surface. We already scope the commit to memory artifacts (good — limits blast radius), but the agent can write a key into `KNOWLEDGE.md`/`NOTES.md`. An unguarded auto-commit on a timer can push secrets unattended.

### F4 — Memory must be consolidated, not just persisted
Durable storage is "non-negotiable," and **file-based memory is superior for precision/content generation** (vs vector for fuzzy recall) `[for memory: durable + precise]` — so our git-file memory is a sound, even strong, choice (git adds free versioning/audit/rollback). But the same literature insists on **consolidation**: a 4-lever model — *importance* (what becomes memory), *merge* (unify facts), *decay* (confidence ages), *eviction* (remove) — because unbounded append causes context rot and "several conflicting versions of the same facts" `[8][14]`.

**→ su-code:** the new append-only `## Learnings` zone is right for capture but has no budget or consolidation; over months it rots. Need a size cap + a periodic summarize/dedup/archive step.

### F5 — A *self-improving* loop requires a verifier
Reflexion-style self-improvement = **Generate → Critique → Refine** with NL feedback in memory, no retraining; production guidance stresses **guided evaluation against specific criteria, not open-ended introspection**, and the hard **verifiability constraint: agents cannot reliably improve beyond what can be objectively verified** `[15][16]`.

**→ su-code:** `harness up` is a *memory/context* loop (re-inject + re-index + persist), not an *improvement* loop. That's an honest, useful scope — but to claim "self-improving" we must (a) keep capture, and (b) only mark a learning "validated" when a test/build/benchmark confirmed it, keeping unverified hypotheses separate. A metric-optimization loop is the `autoresearch` skill's job, not `harness`.

---

## Standard plan (real-world, phased)

**P0 — correctness/safety (do first)**
1. **Slim the injected context.** `harness init`/`inject_agents_md`: root block lists always-on chain (lean) + on-demand skills as **names only** (one line), not full descriptions; rely on progressive disclosure. Add a `doctor` lint: warn if the injected `8sync:skills` block exceeds a line budget (e.g. >40 lines) or duplicates `SKILL.md` text. *Evidence F1.*
2. **Pin skills.** Add `rev`/`sha` to each `skills.toml` entry; `skill add <url>#<ref>` pins; `skill update` records the resolved `git rev-parse HEAD`; `harness init` on a fresh machine checks out the pinned SHA. Keep vendored `agents/skills/` as the offline content-lock. *Evidence F2.*
3. **Guard auto-commit.** Before `harness up --commit` commits, run `gitleaks protect --staged` (if installed) and **abort on detection**; `harness init` offers to install a gitleaks pre-commit hook; `doctor` reports gitleaks presence. Never auto-commit on `--timer` without the scan passing. *Evidence F3.*

**P1 — durability**
4. **Bound + consolidate memory.** Cap the `## Learnings` zone (e.g. ~200 lines); a `harness up` consolidation step (or `8sync note --consolidate`) dedups/summarizes older entries into a dated archive (`agents/archive/`) or rolls them into `CHANGELOG`. Git history preserves the full trail (decay/eviction without data loss). *Evidence F4.*

**P2 — genuine improvement (optional, scoped)**
5. **Verifier-gated learnings.** Tag KNOWLEDGE entries `validated:` (a test/build confirmed) vs `hypothesis:`; only validated learnings are authoritative. Defer metric-optimization loops to the `autoresearch` skill — do **not** bolt a fake metric loop onto `harness`. *Evidence F5.*

**Non-goals / explicit decisions**
- No vector DB / external memory service: file+git is the correct precision-first, portable, auditable substrate for a single-dev coding harness `[memory: file-based superior for precision]`. Hybrid vector stacks solve fuzzy cross-session recall we don't need.
- Keep `harness up` default network-free + fast; pinning/scanning are opt-in or local-cheap.
- Honesty: the loop is a memory loop; "self-improving" is only truthful with F5's verifier gate.
