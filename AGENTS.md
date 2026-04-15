# Skeepy — Agent Bootstrap Protocol

## FIRST ACTION (mandatory for any agent)

Read `.gsd/STATE.md` before anything else.
That file tells you exactly where work is and what to do next.

If `.gsd/milestones/M001/slices/<active>/continue.md` exists → read it first and resume from "Next Action".

---

## Project Summary

Desktop notes aggregator for Windows. Local-first, provider-based architecture.
Stack: Rust + Tauri 2.x (shell) + Solid.js (UI) + SQLite FTS5 (storage).
Google Keep = one optional provider, not the core.

Goal: Run 24/7 without degrading the PC (< 50MB RAM idle, ~0% CPU idle).

---

## File Map

| File | Purpose |
|------|---------|
| `.gsd/STATE.md` | **Read first** — current position + next action |
| `.gsd/DECISIONS.md` | Append-only decisions register — respect before deciding |
| `.gsd/milestones/M001/M001-ROADMAP.md` | Milestone slices + dependency graph + boundary map |
| `.gsd/milestones/M001/M001-CONTEXT.md` | Architecture decisions, domain model, acceptance criteria |
| `.gsd/milestones/M001/M001-RESEARCH.md` | Pitfalls, APIs verified, don't-hand-roll list |
| `.gsd/milestones/M001/slices/S##/S##-PLAN.md` | Active slice task list |
| `.gsd/milestones/M001/slices/S##/continue.md` | Interrupted work resume point (delete after reading) |

---

## Session Protocol

1. **Read** `.gsd/STATE.md`
2. **Check** for `continue.md` in active slice → if found, resume from it (then delete it)
3. **Read** `DECISIONS.md` before any architectural choice
4. **Read** active slice `S##-PLAN.md` → find next incomplete task
5. **Execute** the task
6. **Verify** must-haves from the task plan
7. **Write** `T##-SUMMARY.md` with frontmatter
8. **Mark** task done in `S##-PLAN.md`
9. **Update** `STATE.md` with new position + next action
10. If context filling up mid-task → write `continue.md`, stop cleanly

---

## Decision Protocol

- If a decision must be made → check `DECISIONS.md` first
- If it conflicts with an existing decision → surface to user before proceeding
- If it's a new decision → append to `DECISIONS.md` (never edit existing rows)

---

## Verification Ladder (use strongest tier reachable)

1. Static: files exist, exports present, not stubs
2. Command: tests pass, build succeeds, lint clean
3. Behavioral: UI flows work, API responses correct
4. Human: ask user only when you genuinely cannot verify

"All steps done" is NOT verification. Check actual outcomes.
