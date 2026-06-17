# Code Review Graph Workflow

Use this workflow for multi-file edits, architecture-sensitive tasks, and broad codebase exploration.

## 1. Detect Availability

- Treat code-review-graph as available when CLI, MCP tools, or IDE slash commands respond.
- Do not suggest reinstalling if the tool already works.
- Suggest installation only when the tool is missing or unusable.

## 2. Default Execution Flow

1. Run graph status.
2. Refresh the graph.
Use full build for cold start and incremental update after recent edits.
3. Query graph relationships.
Inspect impact radius, callers, callees, and affected flows.
4. Confirm with file-level evidence.
Use local file reads and targeted search before editing.

## 3. Recommended Commands

- `code-review-graph status`
- `code-review-graph build`
- `code-review-graph update`
- `code-review-graph detect-changes`
- `code-review-graph review-pr`
- `code-review-graph review-delta`

## 4. Fallback Rule

If graph tools are unavailable:

1. State that graph analysis was skipped.
2. Continue with workspace-grounded file inspection.
3. Keep assumptions explicit and low risk.
