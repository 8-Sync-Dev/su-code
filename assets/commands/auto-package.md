---
name: sx-auto-package
argument-hint: '[<name for the packaged command>] [--skill] [--force]'
description: Turn the workflow you just performed in this session into ONE reusable slash command — harvest the real steps from the transcript, drop the dead ends, parameterise what varied, and write it out.
---

# /sx-auto-package — package what just happened into one command

`$ARGUMENTS` = optional name for the new command. `--skill` also emits a companion skill when the
procedure carries real domain knowledge. `--force` overwrites.

The point: a procedure that worked once is worth exactly nothing if reproducing it means re-deriving
it. This harvests the **actual** steps from this session — not a remembered idealisation of them.

## 1. Harvest the real trace (do not reconstruct from memory)

Pull from the concrete record, in this order:

1. **`todo` list** — the phase/task decomposition that survived contact with reality.
2. **The transcript** — `history://` lists agents; `history://<id>` is the markdown log. Extract the
   commands actually executed and their outputs.
3. **`git diff` / `git log`** for this session's range — the files that really changed.
4. **Subagent artifacts** — `agent://<id>` for anything delegated.

Write the raw ordered step list first. Include the failures; you need them for step 3.

## 2. Separate signal from thrash

For each harvested step, classify:

- **Essential** — the workflow fails without it. Keep.
- **Discovery** — how you *found* the answer (greps, exploratory reads). Drop; replace with the
  answer as a constant or a parameter.
- **Dead end** — tried and reverted. Drop from the steps, but keep it as a one-line **Pitfalls**
  entry so the next run does not repeat it. This is the highest-value part of packaging.
- **Environmental** — true only of this machine (a path, a version, a distro). Parameterise or probe.

Deleting the discovery phase is usually what turns a 40-step session into a 6-step command.

## 3. Parameterise what varied

Anything that would differ on the next run becomes `$ARGUMENTS` or a probe. Hardcode only what is
genuinely invariant. If a value came from a real check (a package name verified with `dnf repoquery`,
a line number from a symbol lookup), re-probe it in the command rather than freezing a stale literal.

## 4. Add the gates that were implicit

You verified things by judgement during the session; the command cannot. Make each check explicit
and mechanical — an exact command plus the expected result. A packaged workflow with no acceptance
check is a script that fails silently.

## 5. Emit

```bash
8sync skill new --command <name> '<description>'     # then fill the body
8sync skill new <name> '<description>'               # only with --skill
```

Body structure: purpose → prerequisites (with probes) → numbered steps → acceptance checks →
pitfalls harvested in step 2 → explicit non-goals.

## 6. Prove it

Re-run the packaged command against the same situation and confirm it reaches the same end state.
A package you have not replayed is a guess.

```bash
8sync harness            # deploy the new command
```

## Refuse when

- The session was a one-off investigation with no repeatable procedure.
- The workflow is already one `8sync` verb — say which.
- Fewer than ~3 non-trivial steps survive step 2. That is a shell alias, not a command.
