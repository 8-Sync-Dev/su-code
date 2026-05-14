# Karpathy Guidelines — software engineering baseline

**Read this skill before every non-trivial coding task.**

Original source: Andrej Karpathy's engineering principles for clear, simple, working software.

## Core principles

1. **Avoid over-engineering.** Solve the problem you have, not the one you might.
2. **Test before refactor.** No refactor without green tests.
3. **Read before write.** Read the file, its callers, its tests — then change.
4. **Make it work, then make it right, then make it fast.** In that order.
5. **Small steps, fast feedback.** Compile / test / run after every meaningful change.
6. **Boring code is good code.** Predictability beats cleverness.
7. **One change at a time.** Don't bundle refactor + feature + style.
8. **Explicit > implicit.** Types, names, error paths, side effects: all visible.
9. **Delete more than you add.** A diff that removes code is a strong diff.
10. **If unsure, write the test first.** The test forces clarity.

## Operational rules

- Before writing code: re-state the user's goal in one sentence.
- Before committing: run lint + test. If either fails, do not commit.
- Never claim "done" if any of: failing tests, partial implementation, unresolved errors.
- Never delete failing tests without a compelling reason explained in the diff.
- Don't dump huge tool output into context. Summarize, then read narrow slices.

## When uncertain

Pick the smaller, more obvious step. Ask the user. Don't invent ambition.
