# Keyword Rules

Apply keyword routing when matching signals appear in user requests.
Use these rules with `SKILL.md` and `references/skill-registry.md`.

## `codebase`, `explore repo`, `read codebase`, `scan repository`

- Build structural context before proposing changes.
- Prefer graph-first analysis when available.
- Confirm all conclusions with direct workspace evidence.

## `fix`, `resolve`, `sua loi`, `patch`

- Diagnose root cause before patching.
- Preserve compatibility and keep patch scope minimal.
- Validate with relevant checks before finalizing.

## `debug`, `trace`, `diagnose`, `reproduce`

- Reproduce issue and gather runtime evidence first.
- Test hypotheses and isolate root cause before edits.
- Use browser runtime tools for UI issues.

## `refactor`, `cleanup`, `simplify`

- Preserve behavior unless change is explicitly requested.
- Reduce complexity and duplication with minimal risk.
- Avoid speculative abstractions and large architectural shifts.

## `performance`, `slow`, `Core Web Vitals`, `LCP`, `INP`, `CLS`, `bundle`, `rerender`

- Measure first, then optimize proven bottlenecks.
- Prioritize render cost, bundle weight, waterfalls, and main-thread work.
- Re-measure after optimization to confirm impact.

## `security`, `OWASP`, `XSS`, `CSRF`, `auth`, `authz`, `secrets`, `sanitization`

- Validate trust boundaries and input/output safety.
- Verify auth/authz paths and token/session handling.
- Review dependency risk and avoid unsafe client data exposure.

## `responsive`, `mobile`, `tablet`, `breakpoints`, `fluid`

- Verify layout adaptability across viewport sizes.
- Validate overflow, spacing, typography scaling, and touch targets.
- Ensure loading/empty/error states stay usable on smaller screens.

## `design`, `redesign`, `layout`, `wireframe`, `ui refresh`

- Inspect existing tokens/components before introducing new visuals.
- Improve hierarchy, consistency, and interaction clarity.
- Ensure accessibility and responsiveness remain intact.

## `state`, `store`, `query`, `cache`, `server state`

- Respect existing state architecture before introducing new patterns.
- Keep state ownership clear and avoid duplicated sources of truth.
- Validate loading/error/empty transitions and stale-data behavior.

## `table`, `grid`, `virtualized`, `infinite list`

- Prefer established table/virtualization patterns.
- Validate performance with large datasets and interaction responsiveness.
- Preserve accessibility semantics for tabular/list content.

## `browser automation`, `open website`, `fill form`, `click button`, `screenshot`, `scrape page`

- Use `agent-browser` as primary automation path.
- Use loop: `open` -> `snapshot -i` -> interact -> wait -> re-snapshot.
- Selector priority: refs -> semantic locators -> CSS/Text/XPath fallback.

## `optimize prompt`, `improve prompt`, `rewrite prompt`

- Use prompt optimization workflow only.
- Do not execute implementation code when request is prompt-only.
## Subagent Delegation

- For specialized deep review/audit roles, use `rules/subagent-routing.mdc` and `references/subagent-registry.md`.
- Keep implementation in skill workflow; use subagents as focused specialists.

