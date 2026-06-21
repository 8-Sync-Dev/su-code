# Provenance — harness-selfimprove-research-brief

**Generated:** 2026-06-21 · **Workflow:** deep-research (researcher → verifier → reviewer, executed in-harness via web_search + URL read + repo grounding). The `/deepresearch` slash-command was not available in this runtime; methodology approximated faithfully (multi-source, cross-verified, primary-source-pinned).

## Search streams (queries run)
1. AGENTS.md open standard 2026 adoption/spec
2. Anthropic Agent Skills SKILL.md progressive disclosure + versioning
3. AI agent long-term memory persistence — file vs vector (Letta/Mem0)
4. Self-improving agents — reflexion / memory loop / production patterns
5. Claude Code plugins marketplace — version pinning / lockfile / auto-update
6. Automated git-commit AI secret leakage — pre-commit secret scanning
7. Agent memory consolidation / pruning / context rot / unbounded knowledge
8. Verifier pass: Gloaguen 2026 — 138-repo context-file study (primary source)

## Verification status
- **VERIFIED (primary, high confidence):** Gloaguen et al., *Evaluating AGENTS.md: Are Repository-Level Context Files Helpful for Coding Agents?* arXiv **2602.11988** (v1, 2026-02-12; ETH Zurich + LogicStar.ai). AGENTBENCH = 138 tasks from 5694 PRs / 12 repos. Claim "context files reduce success + >20% cost; LLM-generated duplicate docs; +2.7% when docs removed" confirmed across abstract `[1]`, arXiv HTML `[4]`, and 3 independent write-ups `[2][6][10-list]`.
- **VERIFIED (vendor docs):** Claude Code plugin pinning/lockfile/SHA — Claude Code Docs (plugin-dependencies, plugins-reference) `[10][11]`, corroborated by Morph + claudefa.st `[distribution]`.
- **VERIFIED (annual report):** GitGuardian State of Secrets Sprawl 2026 — 28.65M secrets, Claude-Code 3.2% vs 1.5% leak rate, gitleaks recommendation `[12]`; corroborated by Snyk + Help Net Security `[13]`.
- **CORROBORATED (multi-secondary, no single primary):** memory consolidation 4-lever model (Hindsight/Vectorize `[8]`, Steve Kinney `[14]`); context rot (Chroma-cited, memgraph/harness/TDS `[7][9]`); self-improving loop + verifiability constraint (o-mega/stackviv + arXiv 2603.* memory surveys `[15][16]`). Treated as INFERENCE-grade consensus, not single-source fact.
- **UNVERIFIED claim left out of brief:** specific "+68%/yr compounding" figure (o-mega) — illustrative, not load-bearing; excluded from recommendations.

## Source list (deduped, most-cited first)
- [1] arXiv 2602.11988 — Evaluating AGENTS.md (Gloaguen et al.) — https://arxiv.org/abs/2602.11988 · HTML: /html/2602.11988v1
- [2] Anthropic — Equipping agents for the real world with Agent Skills — anthropic.com/engineering/...agent-skills
- [3] Agent Skills overview/docs — platform.claude.com/docs/en/agents-and-tools/agent-skills/overview ; agentskills.io
- [4] anthropics/skills (GitHub) + The New Stack "Agent Skills" (2025-12-18)
- [5] AGENTS.md patterns/spec — blakecrosley.com/blog/agents-md-patterns ; asdlc.io/practices/agents-md-spec (TLS-failed on direct fetch; used search synthesis) ; 0xfauzi gist
- [6] Knowledge Activation — arXiv 2603.14805
- [7] Context rot — towardsdatascience.com/deep-dive-into-context-engineering ; memgraph.com/blog/ai-context-rot ; harness.io/blog/defeating-context-rot
- [8] Consolidation — hindsight.vectorize.io/blog/2026/05/21/agent-memory-consolidation ; stevekinney.com/writing/agent-memory-systems
- [9] Agentic workflow patterns 2026 — ai.plainenglish.io ; sitepoint definitive guide
- [10] Claude Code Docs — plugin-dependencies ; plugins-reference (code.claude.com/docs)
- [11] Morph / claudefa.st / ice-ice-bear — Claude Code marketplace deep dives
- [12] GitGuardian — State of Secrets Sprawl 2026 — blog.gitguardian.com/the-state-of-secrets-sprawl-2026
- [13] Snyk state-of-secrets ; Help Net Security (ggshield AI hook) ; gitleaks/gitleaks (GitHub)
- [14] Memory persistence — cognee.ai ; xelionlabs ; mem0.ai ; fast.io ; atlan.com (file-vs-vector, Letta/Mem0)
- [15] Self-improving — o-mega.ai/articles/self-improving-ai-agents-the-2026-guide ; stackviv.ai/blog/reflection-ai-agents
- [16] Memory governance/surveys — arXiv 2603.11768 (SSGM), 2603.07670, 2603.18718 ; 2512.13564

## Repo grounding (this codebase, observed)
- `crates/cli/src/verbs/skill/update.rs` — clones `--depth 1` HEAD, no SHA recorded (F2 gap).
- `crates/cli/src/verbs/harness/up.rs::commit_memory` — `git add` scoped to memory + `git commit`, no secret scan (F3 gap).
- `crates/cli/src/verbs/harness/memory.rs::seed_harness_memory` — `## Learnings` zone append-only, no budget (F4 gap).
- `inject_agents_md` / observed `CLAUDE.md` diff — full on-demand skill descriptions injected at root (F1 gap).
- `skills.toml` schema — `[name] src/when` only; no `rev`/`sha` (F2).

## Limitations
- 2026 sources are recent and partly secondary/marketing; figures cross-checked where load-bearing, flagged INFERENCE where consensus-only.
- One primary (asdlc.io) failed TLS on direct read; relied on search synthesis + corroborators.
- No runtime A/B on this repo — recommendations are evidence-grounded, not measured here.
