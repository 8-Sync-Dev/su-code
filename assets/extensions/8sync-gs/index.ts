// 8sync-gs — the omp adapter for the native /gs agent-team engine.
//
// This is the ONLY module that touches omp. It registers the /gs command and
// the eight gs_* tools, wires the tool_call / tool_result / lifecycle hooks,
// routes + restores the coordinator model per stage, and renders the terminal
// widget. All decisions live in the pure modules (machine/policy/store); this
// file adapts them to omp's ExtensionAPI. 100% on omp core — no patching.
import type {
  ExtensionAPI,
  ExtensionCommandContext,
  ExtensionContext,
  ToolCallEvent,
  ToolResultEvent,
} from "@oh-my-pi/pi-coding-agent";
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, isAbsolute, join, relative, resolve } from "node:path";

import { DEFAULT_CONFIG, loadConfig } from "./config.ts";
import {
  actionApproved,
  advance,
  afterStage,
  applyVerify,
  attachAcceptanceEvidence,
  block,
  clearPending,
  counts,
  createRun,
  defineRequirements,
  emptyRisk,
  evaluateGate,
  fnv1a,
  findTask,
  planApproved,
  recordAgentRun,
  recordApproval,
  reopen,
  setPending,
  setPlan,
} from "./machine.ts";
import {
  agentsForStage,
  classifyBashCommand,
  isGitCommit,
  selectStageModel,
  stageRequirement,
  textTriggersSecurity,
  type ModelResolver,
  type ResolvedModel,
} from "./policy.ts";
import {
  clearActive,
  gsPaths,
  loadRuntime,
  resume,
  saveCheckpoint,
  saveRuntime,
} from "./store.ts";
import {
  assignmentHash,
  evidenceFromTaskDetails,
  type SingleResultLike,
  type TaskDetailsLike,
} from "./task-evidence.ts";
import { runVerify } from "./verify.ts";
import { runContractBenchmark } from "./benchmark.ts";
import { HELP, stageInstruction } from "./prompts.ts";
import { WIDGET_KEY, statusLine, widgetLines } from "./ui.ts";
import type {
  AcceptanceCriterion,
  GsAgent,
  GsConfig,
  GsModelRole,
  GsState,
  Mode,
  Plan,
  PlanTask,
  Requirement,
  RiskAssessment,
  Stage,
  ThinkingLevel,
  VerifyCommand,
} from "./types.ts";

export default function (pi: ExtensionAPI) {
  const { z } = pi.zod;
  pi.setLabel("8sync GS (native agent-team engine)");

  // ---- config -----------------------------------------------------------
  function readFile(path: string): string | undefined {
    return existsSync(path) ? readFileSync(path, "utf8") : undefined;
  }
  function loadGsConfig(root: string): GsConfig {
    try {
      return loadConfig(readFile, join(homedir(), ".config/8sync/gs.json"), join(root, ".omp/gs.json"));
    } catch (e) {
      pi.setLabel("entry", undefined);
      return DEFAULT_CONFIG;
    }
  }

  // ---- state helpers ----------------------------------------------------
  function state(root: string): GsState | undefined {
    return loadRuntime(root);
  }
  function persist(root: string, s: GsState): void {
    saveRuntime(root, s);
  }
  function text(s: string) {
    return { content: [{ type: "text" as const, text: s }] };
  }

  // ---- model routing ----------------------------------------------------
  interface ModelIndex {
    resolve: ModelResolver;
    modelForId: (id: string) => unknown | undefined;
    familyOf: (id: string) => string | undefined;
  }
  function buildModelIndex(ctx: ExtensionContext | ExtensionCommandContext): ModelIndex {
    const list = ctx.models.list();
    const byId = new Map<string, unknown>();
    const familyById = new Map<string, string>();
    for (const m of list) {
      const id = `${m.provider}/${m.id}`;
      byId.set(id, m);
      familyById.set(id, ctx.models.family(m));
    }
    const toResolved = (m: (typeof list)[number]): ResolvedModel => ({
      id: `${m.provider}/${m.id}`,
      provider: m.provider,
      model: m.id,
      family: ctx.models.family(m),
    });
    const resolve: ModelResolver = (selector) => {
      if (selector.startsWith("family:")) {
        const kw = selector.slice("family:".length).toLowerCase();
        const hit = list.find((m) => `${m.provider}/${m.id}`.toLowerCase().includes(kw));
        return hit ? toResolved(hit) : undefined;
      }
      const m = ctx.models.resolve(selector);
      return m ? toResolved(m) : undefined;
    };
    return {
      resolve,
      modelForId: (id) => byId.get(id),
      familyOf: (id) => familyById.get(id),
    };
  }

  /** Route the MAIN coordinator to the stage's model role (best-effort). */
  async function routeCoordinator(ctx: ExtensionCommandContext, s: GsState): Promise<void> {
    const idx = buildModelIndex(ctx);
    const role = (stageRequirement(s.stage).modelRole ?? "coordinator") as GsModelRole;
    const cfg = loadGsConfig(s.projectRoot);
    const thinking = cfg.thinking[s.stage];
    const sel = selectStageModel(role, cfg, idx.resolve, { thinking });
    if (!sel.model) return; // coordinator routing is best-effort; subagent gate is authoritative
    if (!s.originalCoordinator) {
      const cur = ctx.models.current();
      if (cur) {
        s.originalCoordinator = {
          id: `${cur.provider}/${cur.id}`,
          provider: cur.provider,
          model: cur.id,
          family: ctx.models.family(cur),
          requestedSelector: "session",
          isFallback: false,
          thinking: pi.getThinkingLevel() as ThinkingLevel | undefined,
        };
      }
    }
    const target = idx.modelForId(sel.model.id);
    if (target) {
      try {
        const ok = await pi.setModel(target as never);
        if (ok) {
          s.activeCoordinator = sel.model;
          if (thinking) pi.setThinkingLevel(thinking);
        }
      } catch {
        /* keep current model */
      }
    }
  }

  async function restoreCoordinator(ctx: ExtensionContext): Promise<void> {
    const s = state(ctx.cwd);
    if (!s?.originalCoordinator) return;
    const idx = buildModelIndex(ctx);
    const target = idx.modelForId(s.originalCoordinator.id);
    if (target) {
      try {
        await pi.setModel(target as never);
        if (s.originalCoordinator.thinking) pi.setThinkingLevel(s.originalCoordinator.thinking);
      } catch {
        /* ignore */
      }
    }
  }

  function refreshUi(ctx: ExtensionContext | ExtensionCommandContext, s: GsState): void {
    if (!ctx.hasUI) return;
    ctx.ui.setWidget(WIDGET_KEY, widgetLines(s), { placement: "aboveEditor" });
    ctx.ui.setStatus(WIDGET_KEY, statusLine(s));
  }
  function clearUi(ctx: ExtensionContext): void {
    if (!ctx.hasUI) return;
    ctx.ui.setWidget(WIDGET_KEY, undefined);
    ctx.ui.setStatus(WIDGET_KEY, undefined);
  }

  // =======================================================================
  // Tools
  // =======================================================================
  pi.registerTool({
    name: "gs_status",
    label: "GS: status",
    description: "Report the /gs run: stage, gate evidence, blockers, and the exact next action. Read-only.",
    parameters: z.object({}),
    async execute(_id, _p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run. Start one with /gs <goal>.");
      const c = counts(s);
      const gate = evaluateGate(s, loadGsConfig(s.projectRoot));
      const lines = [
        `GS ${s.slug} — ${s.stage} (${s.status}), rev ${s.revision}, mode ${s.mode}`,
        `Goal: ${s.goal}`,
        `Tasks ${c.tasksPassed}/${c.tasksTotal} (${c.tasksBlocked} blocked) · AC ${c.acPassed}/${c.acTotal}`,
        `Coordinator: ${s.activeCoordinator?.id ?? "—"}${s.activeCoordinator?.isFallback ? " (fallback)" : ""}`,
        gate.ok ? "Current gate: PASS (call gs_advance)" : `Current gate: ${gate.findings.map((f) => `${f.code} — ${f.message}`).join("; ")}`,
        s.pendingAction ? `Next: ${s.pendingAction.instruction}` : "Next: call gs_next",
      ];
      return text(lines.join("\n"));
    },
  });

  pi.registerTool({
    name: "gs_define",
    label: "GS: define",
    description:
      "Record normalized requirements, observable acceptance criteria, and a deterministic risk assessment for the clarify stage.",
    parameters: z.object({
      requirements: z.array(
        z.object({
          id: z.string(),
          text: z.string(),
          kind: z.enum(["functional", "nonfunctional", "constraint", "nongoal"]).default("functional"),
          required: z.boolean().default(true),
          status: z.enum(["open", "confirmed", "needs_confirmation"]).default("confirmed"),
        }),
      ),
      acceptance: z.array(
        z.object({
          id: z.string(),
          text: z.string(),
          requirements: z.array(z.string()).default([]),
          method: z.enum(["verify", "review", "uat", "manual"]).default("verify"),
          required: z.boolean().default(true),
        }),
      ),
      risk: z.object({
        trivial: z.boolean().default(false),
        externalUnknown: z.boolean().default(false),
        newArchitecture: z.boolean().default(false),
        security: z.boolean().default(false),
        destructive: z.boolean().default(false),
        outwardEffects: z.boolean().default(false),
        notes: z.array(z.string()).default([]),
      }),
      expectedRevision: z.number().optional(),
    }),
    async execute(_id, p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      if (s.stage !== "clarify") return text(`gs_define only applies in clarify (current: ${s.stage}).`);
      const requirements: Requirement[] = p.requirements.map((r) => ({ ...r }));
      const acceptance: AcceptanceCriterion[] = p.acceptance.map((a) => ({ ...a, status: "pending", evidence: [] }));
      // Auto-augment security risk from AC/requirement text so a security review
      // cannot be skipped by an under-declared risk assessment.
      const risk: RiskAssessment = { ...emptyRisk(), ...p.risk };
      const allText = [...requirements, ...acceptance].map((x) => x.text).join(" ");
      if (textTriggersSecurity(allText)) risk.security = true;
      try {
        defineRequirements(s, { requirements, acceptance, risk, expectedRevision: p.expectedRevision });
        s.approvals = s.approvals.filter((approval) => approval.what !== "requirements" && approval.what !== "plan" && approval.what !== "uat");
      } catch (e) {
        return text(`gs_define rejected: ${(e as Error).message}`);
      }
      persist(ctx.cwd, s);
      refreshUi(ctx, s);
      const gate = evaluateGate(s, loadGsConfig(s.projectRoot));
      return text(
        `Recorded ${requirements.length} requirements, ${acceptance.length} ACs. Risk: ${riskSummary(risk)}.\n` +
          (gate.ok
            ? s.mode === "assisted"
              ? "Clarify gate ready — ask the user to run /gs approve requirements."
              : "Clarify gate ready — call gs_advance."
            : `Clarify gate needs: ${gate.findings.map((f) => f.message).join("; ")}`),
      );
    },
  });

  pi.registerTool({
    name: "gs_plan",
    label: "GS: plan",
    description:
      "Record the structured plan (slices→tasks with acceptance IDs, file ownership, dependsOn, skills, and DIRECT-ARGV verify commands). Replaces any prior plan and its approvals.",
    parameters: z.object({
      hash: z.string().optional(),
      slices: z.array(
        z.object({
          id: z.string(),
          title: z.string(),
          tasks: z.array(
            z.object({
              id: z.string(),
              title: z.string(),
              acceptance: z.array(z.string()).default([]),
              ownership: z.array(z.string()).default([]),
              dependsOn: z.array(z.string()).default([]),
              skills: z.array(z.string()).default([]),
              verify: z
                .array(
                  z.object({
                    program: z.string(),
                    args: z.array(z.string()).default([]),
                    cwd: z.string().optional(),
                    timeoutSeconds: z.number().optional(),
                  }),
                )
                .default([]),
            }),
          ),
        }),
      ),
      expectedRevision: z.number().optional(),
    }),
    async execute(_id, p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      if (s.stage !== "plan") return text(`gs_plan only applies in plan (current: ${s.stage}).`);
      const plan: Plan = {
        hash: p.hash ?? `plan-${Date.now().toString(16)}`,
        createdAt: new Date().toISOString(),
        slices: p.slices.map((sl) => ({
          id: sl.id,
          title: sl.title,
          tasks: sl.tasks.map((t) => ({
            id: t.id,
            title: t.title,
            acceptance: t.acceptance,
            ownership: t.ownership,
            dependsOn: t.dependsOn,
            skills: t.skills,
            verify: t.verify,
            status: "pending" as const,
            attempts: 0,
            failStreak: 0,
            lastFailureHash: "",
            note: "",
          })),
        })),
      };
      try {
        setPlan(s, plan, p.expectedRevision);
      } catch (e) {
        return text(`gs_plan rejected: ${(e as Error).message}`);
      }
      persist(ctx.cwd, s);
      refreshUi(ctx, s);
      const gate = evaluateGate(s, loadGsConfig(s.projectRoot));
      return text(
        `Plan recorded: ${plan.slices.length} slices, ${plan.slices.reduce((n, x) => n + x.tasks.length, 0)} tasks.\n` +
          (gate.ok ? "Plan gate PASS — call gs_advance to enter plan_review." : `Plan gate needs: ${gate.findings.map((f) => f.message).join("; ")}`),
      );
    },
  });

  pi.registerTool({
    name: "gs_next",
    label: "GS: next action",
    description:
      "Compute and lease the next required action for the current stage: which gs-* agent to spawn, with which model, over which task IDs. Sets the pending action the tool_call hook enforces.",
    parameters: z.object({}),
    async execute(_id, _p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      const cfg = loadGsConfig(s.projectRoot);
      const idx = buildModelIndex(ctx);
      const action = computeNext(s, cfg, idx);
      if (!action) {
        if (s.status === "blocked") return text(`Run BLOCKED. ${lastAudit(s)}`);
        return text(`No agent action needed at ${s.stage}. Evaluate the gate and call gs_advance.`);
      }
      if (action.kind === "blocked") {
        block(s, action.instruction);
        persist(ctx.cwd, s);
        refreshUi(ctx, s);
        if (ctx.hasUI) ctx.ui.notify(`GS blocked: ${action.instruction}`, "error");
        return text(`BLOCKED: ${action.instruction}`);
      }
      setPending(s, action.pending);
      persist(ctx.cwd, s);
      refreshUi(ctx, s);
      return text(action.instruction);
    },
  });

  pi.registerTool({
    name: "gs_verify",
    label: "GS: verify (direct-argv gate)",
    description:
      "Run a task's plan-approved verify commands DIRECTLY (no shell). All must exit 0. Applies the doom-loop guard (2 identical failures warn, 3 block). The model cannot self-report a pass.",
    parameters: z.object({ taskId: z.string() }),
    async execute(_id, p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      if (s.stage !== "implement" || !planApproved(s)) {
        return text(`gs_verify is gated until an approved plan is in implement (current: ${s.stage}).`);
      }
      const task = findTask(s, p.taskId);
      if (!task) return text(`No task ${p.taskId}.`);
      const worker = s.stages.implement.agentRuns.find(
        (run) => run.agent === "gs-worker" && run.taskId === p.taskId && run.exitCode === 0,
      );
      if (!worker) return text(`Task ${p.taskId} has no matching successful gs-worker evidence.`);
      if (task.verify.length === 0) return text(`Task ${p.taskId} has no verify commands — a plan task must carry >=1 direct-argv check.`);
      const checked = validateVerifyCommands(task.verify, ctx.cwd);
      if (!checked.ok) return text(`gs_verify rejected before execution: ${checked.reason}`);
      const dangerous = checked.commands
        .map((command) => ({ command, classification: classifyDirectCommand(command), hash: verifyActionHash(command, ctx.cwd) }))
        .filter((item) => item.classification.destructive || item.classification.outward);
      const missing = dangerous.find((item) => !actionApproved(s, item.hash));
      if (missing) {
        return text(
          `gs_verify blocked before execution: ${missing.classification.reason}. ` +
            `Approve this exact one-shot action with /gs approve action ${missing.hash}`,
        );
      }
      for (const item of dangerous) consumeActionApproval(s, item.hash);
      if (dangerous.length > 0) persist(ctx.cwd, s);
      const cfg = loadGsConfig(s.projectRoot);
      const out = runVerify(checked.commands, ctx.cwd, cfg.limits.commandTimeoutSeconds);
      const outcome = applyVerify(s, p.taskId, out.passed, out.failureOutput, out.evidence, cfg);
      if (out.passed) attachAutomaticVerifyEvidence(s, task, out.evidence);
      persist(ctx.cwd, s);
      refreshUi(ctx, s);
      if (out.passed) return text(`VERIFIED ${p.taskId}: ${out.summary}. Continue the wave or call gs_advance.`);
      return text(`${outcome.message}\n\n${out.failureOutput}`);
    },
  });

  pi.registerTool({
    name: "gs_acceptance",
    label: "GS: acceptance evidence",
    description:
      "Attach objective evidence to an acceptance criterion (verify hash, review verdict, browser/smoke/api proof). Cannot approve final UAT — only the user's /gs approve uat does that.",
    parameters: z.object({
      acId: z.string(),
      evidence: z.object({
        kind: z.enum(["verify", "review", "browser", "smoke", "api", "note"]),
        ref: z.string(),
        detail: z.string().optional(),
      }),
      status: z.enum(["pending", "passed", "failed", "blocked", "skipped"]).default("passed"),
    }),
    async execute(_id, p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      if (s.stage !== "uat") return text(`gs_acceptance only applies in uat (current: ${s.stage}).`);
      const ac = s.acceptance.find((a) => a.id === p.acId);
      if (!ac) return text(`No acceptance criterion ${p.acId}.`);
      const compatible = evidenceCompatible(ac.method, p.evidence.kind);
      if (!compatible) return text(`AC ${p.acId} requires ${ac.method} evidence; ${p.evidence.kind} is incompatible.`);
      if (p.status === "passed" && !objectiveEvidenceExists(s, p.evidence.kind, p.evidence.ref, p.evidence.detail)) {
        return text(`AC ${p.acId} evidence ${p.evidence.kind}:${p.evidence.ref} is not present in recorded objective evidence.`);
      }
      s.approvals = s.approvals.filter((approval) => approval.what !== "uat");
      attachAcceptanceEvidence(s, p.acId, { ...p.evidence, at: new Date().toISOString() }, p.status);
      persist(ctx.cwd, s);
      refreshUi(ctx, s);
      return text(`AC ${p.acId} → ${p.status} (${p.evidence.kind}:${p.evidence.ref}).`);
    },
  });

  pi.registerTool({
    name: "gs_advance",
    label: "GS: advance",
    description:
      "Request a stage transition. The machine evaluates the current stage's gate on RECORDED evidence and either advances (routing the next stage's coordinator model) or returns the exact failed gates. Never forces a pass.",
    parameters: z.object({ expectedRevision: z.number().optional() }),
    async execute(_id, p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      const cfg = loadGsConfig(s.projectRoot);
      let res;
      try {
        res = advance(s, cfg, p.expectedRevision);
      } catch (e) {
        return text(`gs_advance rejected: ${(e as Error).message}`);
      }
      persist(ctx.cwd, s);
      if (!res.ok) {
        refreshUi(ctx, s);
        return text(`Gate ${s.stage} not met:\n${res.gate?.findings.map((f) => `- ${f.code}: ${f.message}`).join("\n")}`);
      }
      // Advanced: route the new stage's coordinator model + checkpoint.
      if ("waitForIdle" in ctx) await routeCoordinator(ctx as ExtensionCommandContext, s);
      persist(ctx.cwd, s);
      saveCheckpoint(ctx.cwd, s);
      refreshUi(ctx, s);
      if (s.stage === "done") {
        await restoreCoordinator(ctx);
        persist(ctx.cwd, s);
        if (ctx.hasUI) ctx.ui.notify("GS run complete.", "info");
        return text("GS DONE — all gates passed. Commit locally if the goal asked; the coordinator model is restored.");
      }
      return text(`Advanced to ${s.stage}.\n\n${stageInstruction(s, s.pendingAction)}`);
    },
  });

  pi.registerTool({
    name: "gs_worktree",
    label: "GS: git worktree",
    description: "Isolate a risky slice: open a worktree, squash-merge it back, or remove it.",
    parameters: z.object({ action: z.enum(["open", "merge", "remove"]), slug: z.string() }),
    async execute(_id, p, _sig, _u, ctx) {
      const s = state(ctx.cwd);
      if (!s) return text("No active GS run.");
      const slug = p.slug.replace(/[^a-zA-Z0-9._-]/g, "-");
      if (!slug || slug !== p.slug) return text("Invalid worktree slug; use only letters, digits, dot, underscore, and hyphen.");
      const wt = join(".cache/8sync/gs/wt", slug);
      const branch = `8sync-gs/${slug}`;
      const sh = (args: string[]) => runVerify([{ program: "git", args }], ctx.cwd, 120);
      const owned = s.worktrees?.find((entry) => entry.slug === slug);
      if (p.action === "open") {
        if (owned) return text(`Worktree ${wt} is already owned by this run.`);
        const r = sh(["worktree", "add", wt, "-b", branch]);
        if (!r.passed) return text(`worktree add failed:\n${r.failureOutput}`);
        s.worktrees = [...(s.worktrees ?? []), { slug, path: wt, branch, createdAt: new Date().toISOString() }];
        persist(ctx.cwd, s);
        return text(`Worktree ${wt} on ${branch}.`);
      }
      if (!owned || owned.path !== wt || owned.branch !== branch) {
        return text(`Refusing ${p.action}: ${slug} is not a worktree owned by this GS run.`);
      }
      const clean = sh(["-C", wt, "status", "--porcelain"]);
      if (!clean.passed) return text(`worktree status failed:\n${clean.failureOutput}`);
      if (clean.evidence.some((evidence) => evidence.output.trim().length > 0)) {
        return text(`Refusing ${p.action}: worktree ${wt} has uncommitted changes.`);
      }
      if (p.action === "merge") {
        const m = sh(["merge", "--squash", branch]);
        if (!m.passed) return text(`squash-merge failed:\n${m.failureOutput}`);
      }
      const removed = sh(["worktree", "remove", wt]);
      if (!removed.passed) return text(`worktree remove failed; branch retained:\n${removed.failureOutput}`);
      const branchRemoved = sh(["branch", "-d", branch]);
      s.worktrees = (s.worktrees ?? []).filter((entry) => entry.slug !== slug);
      persist(ctx.cwd, s);
      if (!branchRemoved.passed) {
        return text(
          `${p.action === "merge" ? `Squash-merged ${branch}; ` : ""}removed worktree ${wt}; ` +
            `branch ${branch} retained because safe deletion was refused.`,
        );
      }
      return text(p.action === "merge" ? `Squash-merged ${branch}; commit the staged result.` : `Removed worktree ${wt} and safely deleted ${branch}.`);
    },
  });

  // =======================================================================
  // Hooks
  // =======================================================================
  pi.on("tool_call", (event: ToolCallEvent, ctx: ExtensionContext) => {
    const s = state(ctx.cwd);
    if (!s || s.status === "done" || s.status === "aborted") return;
    const cfg = loadGsConfig(s.projectRoot);

    // 1. No edits before an approved plan.
    if ((event.toolName === "edit" || event.toolName === "write") && !planApproved(s)) {
      if (s.stage === "clarify" || s.stage === "research" || s.stage === "plan" || s.stage === "plan_review") {
        return { block: true, reason: `GS: no file edits before the plan is approved (stage ${s.stage}). Finish planning + review first.` };
      }
    }

    // 2. Bash safety: commits require all verification/review/UAT gates, and
    // destructive/outward actions require exact one-shot user consent.
    if (event.toolName === "bash") {
      const cmd = typeof event.input.command === "string" ? event.input.command : "";
      if (isGitCommit(cmd) && !commitReady(s)) {
        return {
          block: true,
          reason: "GS: git commit is blocked until task verification, verifier, review/security, objective AC evidence, and user UAT all pass.",
        };
      }
      const cls = classifyBashCommand(cmd);
      if (cls.destructive || cls.outward) {
        const needUser =
          (cls.destructive && cfg.safety.requireUserForDestructive) ||
          (cls.outward && cfg.safety.requireUserForExternalEffects);
        if (needUser) {
          const hash = bashActionHash(cmd, ctx.cwd);
          if (!actionApproved(s, hash)) {
            return {
              block: true,
              reason:
                `GS: "${cls.reason}" needs exact one-shot user consent. ` +
                `Run /gs approve action ${hash}, then retry this unchanged command.`,
            };
          }
          consumeActionApproval(s, hash);
          persist(ctx.cwd, s);
        }
      }
    }

    // 3. Agent-required stages: bind one exact task batch to the pending lease.
    if (event.toolName === "task") {
      const allowed = agentsForStage(s.stage, s.risk);
      if (allowed.length === 0) return;
      const pending = s.pendingAction;
      if (
        !pending ||
        pending.kind !== "agent" ||
        pending.stage !== s.stage ||
        pending.leaseRevision !== s.revision ||
        !pending.agent ||
        !pending.assignmentHash
      ) {
        return { block: true, reason: `GS: stage ${s.stage} requires an exact leased assignment. Call gs_next first.` };
      }
      if (pending.toolCallId && pending.toolCallId !== event.toolCallId) {
        return { block: true, reason: `GS: pending assignment is already bound to task call ${pending.toolCallId}.` };
      }
      const items = extractTaskAgents(event.input);
      const expectedTaskIds = pending.taskIds ?? [];
      const expectedCount = expectedTaskIds.length > 0 ? expectedTaskIds.length : 1;
      if (items.length !== expectedCount) {
        return { block: true, reason: `GS: lease requires exactly ${expectedCount} task item(s), received ${items.length}.` };
      }
      const seenTaskIds = new Set<string>();
      for (const item of items) {
        if (item.agent !== pending.agent || !allowed.includes(item.agent as never)) {
          return { block: true, reason: `GS: lease requires agent ${pending.agent}; received ${item.agent ?? "none"}.` };
        }
        if (!modelMatchesLease(item.model, pending, buildModelIndex(ctx))) {
          return {
            block: true,
            reason: `GS: lease requires model ${pending.modelSelector ?? pending.modelId} in family ${pending.modelFamily}; received ${item.model ?? "none"}.`,
          };
        }
        if (!promptHasExactLine(item.task, `Assignment hash: ${pending.assignmentHash}`)) {
          return { block: true, reason: `GS: task prompt must contain exact assignment hash ${pending.assignmentHash}.` };
        }
        if (expectedTaskIds.length > 0) {
          const matched = expectedTaskIds.filter((taskId) => promptHasExactLine(item.task, `Task ID: ${taskId}`));
          if (matched.length !== 1 || seenTaskIds.has(matched[0])) {
            return { block: true, reason: "GS: every leased worker task ID must appear exactly once as `Task ID: <id>`." };
          }
          const taskId = matched[0];
          const task = findTask(s, taskId);
          if (!task || !task.acceptance.every((id) => item.task.includes(id)) || !task.ownership.every((path) => item.task.includes(path))) {
            return { block: true, reason: `GS: worker ${taskId} prompt is missing an exact AC ID or allowed ownership path.` };
          }
          seenTaskIds.add(taskId);
        }
      }
      setPending(s, { ...pending, toolCallId: event.toolCallId, boundRevision: s.revision + 1 });
      persist(ctx.cwd, s);
    }
    return;
  });

  pi.on("tool_result", (event: ToolResultEvent, ctx: ExtensionContext) => {
    if (event.toolName !== "task" || event.isError) return;
    const s = state(ctx.cwd);
    if (!s) return;
    const pending = s.pendingAction;
    if (
      !pending ||
      pending.kind !== "agent" ||
      pending.stage !== s.stage ||
      pending.toolCallId !== event.toolCallId ||
      pending.boundRevision !== s.revision
    ) {
      return;
    }
    const idx = buildModelIndex(ctx);
    const evs = evidenceFromTaskDetails(event.toolCallId, normalizeTaskDetails(event.details), {
      assignmentHash: pending.assignmentHash ?? "",
      requestedSelector: pending.modelSelector ?? pending.modelId ?? "",
      familyOf: (model) => idx.familyOf(model),
    });
    const expectedTaskIds = pending.taskIds ?? [];
    const valid =
      evs.length === (expectedTaskIds.length > 0 ? expectedTaskIds.length : 1) &&
      evs.every(
        (evidence) =>
          evidence.batchToolCallId === pending.toolCallId &&
          evidence.agent === pending.agent &&
          evidence.assignmentHash === pending.assignmentHash &&
          evidence.resolvedModel === pending.modelId &&
          evidence.resolvedModelFamily === pending.modelFamily &&
          evidence.exitCode === 0,
      ) &&
      (expectedTaskIds.length === 0 ||
        new Set(evs.map((evidence) => evidence.taskId)).size === expectedTaskIds.length) &&
      expectedTaskIds.every((taskId) => evs.some((evidence) => evidence.taskId === taskId));
    if (!valid) return;
    let applied = 0;
    for (const evidence of evs) {
      const result = recordAgentRun(s, pending.stage, evidence);
      if (result.applied) applied += 1;
    }
    if (applied === evs.length) {
      clearPending(s);
      persist(ctx.cwd, s);
      refreshUi(ctx, s);
    }
    return;
  });

  pi.on("session_before_compact", (_event, ctx: ExtensionContext) => {
    const s = state(ctx.cwd);
    if (s) saveCheckpoint(ctx.cwd, s);
  });
  pi.on("session_shutdown", async (_event, ctx: ExtensionContext) => {
    const s = state(ctx.cwd);
    if (s) {
      saveCheckpoint(ctx.cwd, s);
      if (s.status === "done" || s.status === "aborted") await restoreCoordinator(ctx);
    }
    clearUi(ctx);
  });

  // =======================================================================
  // /gs command
  // =======================================================================
  pi.registerCommand("gs", {
    description: "Native /gs agent-team engine — decompose a goal into a gated, model-routed, evidence-backed run.",
    handler: async (args: string, ctx: ExtensionCommandContext) => {
      const root = ctx.cwd;
      const raw = args.trim();
      const tokens = raw.length ? raw.split(/\s+/) : [];
      const sub = tokens[0]?.toLowerCase();

      if (sub === "status") {
        const s = state(root);
        if (!s) return void ctx.ui.notify("No active GS run. Start with /gs <goal>.", "info");
        refreshUi(ctx, s);
        return void ctx.ui.notify(statusLine(s), s.status === "blocked" ? "warning" : "info");
      }

      if (sub === "approve") {
        const what = tokens[1]?.toLowerCase();
        const s = state(root);
        if (!s) return void ctx.ui.notify("No active GS run.", "warning");
        if (what === "action") {
          const hash = tokens[2]?.toLowerCase();
          if (!hash || !/^[0-9a-f]{1,8}$/.test(hash)) {
            return void ctx.ui.notify("Usage: /gs approve action <hash>", "warning");
          }
          recordApproval(s, { what: "action", by: "user", note: hash });
          persist(root, s);
          saveCheckpoint(root, s);
          return void ctx.ui.notify(`Approved exact action ${hash} for one use.`, "info");
        }
        if (what !== "requirements" && what !== "plan" && what !== "uat") {
          return void ctx.ui.notify("Usage: /gs approve requirements|plan|uat|action <hash>", "warning");
        }
        const expectedStage = what === "requirements" ? "clarify" : what === "plan" ? "plan_review" : "uat";
        if (s.stage !== expectedStage) {
          return void ctx.ui.notify(`Cannot approve ${what} at ${s.stage}; approval is only valid at ${expectedStage}.`, "warning");
        }
        const gate = evaluateGate(s, loadGsConfig(s.projectRoot));
        const approvalCode = what === "requirements" ? "REQUIREMENTS_UNAPPROVED" : what === "plan" ? "PLAN_UNAPPROVED" : "UAT_UNAPPROVED";
        const blockers = gate.findings.filter((finding) => finding.code !== approvalCode);
        if (blockers.length > 0) {
          return void ctx.ui.notify(`Cannot approve ${what}: ${blockers.map((finding) => finding.message).join("; ")}`, "warning");
        }
        recordApproval(s, { what, by: "user" });
        persist(root, s);
        saveCheckpoint(root, s);
        refreshUi(ctx, s);
        ctx.ui.notify(`Approved ${what}. Driving the run.`, "info");
        pi.sendUserMessage(`/gs approve recorded: ${what}. Re-evaluate the gate with gs_advance and continue.`, { deliverAs: "followUp" });
        return;
      }

      if (sub === "reject") {
        const target = tokens[1]?.toLowerCase() as Stage | undefined;
        const reason = tokens.slice(2).join(" ") || "user rejected";
        const s = state(root);
        if (!s || !target) return void ctx.ui.notify("Usage: /gs reject <stage> <reason>", "warning");
        reopen(s, target, reason);
        invalidateApprovalsForReopen(s, target);
        persist(root, s);
        refreshUi(ctx, s);
        pi.sendUserMessage(`GS reopened ${target}: ${reason}. ${stageInstruction(s, undefined)}`, { deliverAs: "followUp" });
        return;
      }

      if (sub === "pause") {
        const s = state(root);
        if (s) {
          clearPending(s);
          persist(root, s);
          saveCheckpoint(root, s);
          ctx.ui.notify("GS paused — checkpoint written. Resume with /gs continue.", "info");
        }
        return;
      }

      if (sub === "abort") {
        const s = state(root);
        if (s) {
          block(s, "user abort");
          s.status = "aborted";
          s.stage = "aborted";
          await restoreCoordinator(ctx);
          persist(root, s);
          saveCheckpoint(root, s);
          clearUi(ctx);
          ctx.ui.notify("GS aborted. Artifacts kept; coordinator model restored.", "info");
        }
        return;
      }

      if (sub === "reset") {
        if (tokens[1] !== "--yes") return void ctx.ui.notify("Destructive: /gs reset --yes to archive the active run.", "warning");
        clearActive(root);
        clearUi(ctx);
        return void ctx.ui.notify("GS run cleared.", "info");
      }

      if (sub === "benchmark") {
        if (tokens.includes("--live")) {
          ctx.ui.notify("GS live benchmark is unsupported in this build.", "warning");
          pi.sendUserMessage("GS benchmark --live is explicitly unsupported: no credentialed isolated A/B runner is installed.", {
            deliverAs: "followUp",
          });
          return;
        }
        const report = runContractBenchmark();
        ctx.ui.notify(`GS benchmark: ${report.summary}`, report.pass ? "info" : "warning");
        pi.sendUserMessage(`GS contract benchmark:\n${report.detail}`, { deliverAs: "followUp" });
        return;
      }

      if (sub === "continue" || sub === "resume") {
        const r = resume(root);
        if (!r.state) return void ctx.ui.notify(`No resumable GS run (${r.detail}).`, "warning");
        saveRuntime(root, r.state);
        await routeCoordinator(ctx, r.state);
        persist(root, r.state);
        refreshUi(ctx, r.state);
        ctx.ui.notify(`Resumed GS ${r.state.slug} (${r.source}) at ${r.state.stage}.`, "info");
        pi.sendUserMessage(stageInstruction(r.state, r.state.pendingAction), { deliverAs: "followUp" });
        return;
      }

      // Bare `/gs` with no goal → continue active run or show help.
      if (tokens.length === 0) {
        const existing = state(root);
        if (existing) {
          refreshUi(ctx, existing);
          pi.sendUserMessage(stageInstruction(existing, existing.pendingAction), { deliverAs: "followUp" });
          return;
        }
        return void ctx.ui.notify(`No active GS run.\n${HELP}`, "info");
      }

      // Start a new run. `--auto` anywhere → autonomous mode.
      const active = state(root);
      if (active) {
        return void ctx.ui.notify(
          `GS ${active.slug} is already active at ${active.stage}. Continue it or run /gs reset --yes before starting another goal.`,
          "warning",
        );
      }
      const auto = tokens.includes("--auto");
      const goal = tokens.filter((t) => t !== "--auto").join(" ").trim();
      if (!goal) return void ctx.ui.notify("Usage: /gs [--auto] <goal>", "warning");
      const mode: Mode = auto ? "auto" : "assisted";
      const slug = slugify(goal);
      const s = createRun({ runId: `gs-${Date.now().toString(16)}`, slug, goal, projectRoot: root, mode });
      await routeCoordinator(ctx, s);
      persist(root, s);
      saveCheckpoint(root, s);
      refreshUi(ctx, s);
      ctx.ui.notify(`GS started: ${slug} (${mode}).`, "info");
      pi.sendUserMessage(stageInstruction(s, undefined), { deliverAs: "followUp" });
    },
  });
}

// ===========================================================================
// Pure adapter helpers (module scope — no omp needed)
// ===========================================================================

function riskSummary(r: RiskAssessment): string {
  const on = Object.entries({
    trivial: r.trivial,
    external: r.externalUnknown,
    newArch: r.newArchitecture,
    security: r.security,
    destructive: r.destructive,
    outward: r.outwardEffects,
  })
    .filter(([, v]) => v)
    .map(([k]) => k);
  return on.length ? on.join("+") : "none";
}

function lastAudit(s: GsState): string {
  return s.audit[s.audit.length - 1]?.detail ?? "";
}

function allTasksVerified(s: GsState): boolean {
  const tasks = s.plan?.slices.flatMap((slice) => slice.tasks) ?? [];
  return tasks.length > 0 && tasks.every((task) => task.status === "passed");
}

function commitReady(s: GsState): boolean {
  const verifier = latest(s, "verify", "gs-verifier");
  const reviewer = latest(s, "review", "gs-reviewer");
  const security = latest(s, "review", "gs-security");
  const reviewsPass =
    verifier?.verdict === "pass" &&
    reviewer !== undefined &&
    reviewer.verdict !== "incorrect" &&
    (reviewer.blockingFindings ?? 0) === 0 &&
    (!s.risk.security ||
      (security !== undefined && security.verdict !== "incorrect" && (security.blockingFindings ?? 0) === 0));
  return (
    (s.stage === "closeout" || s.stage === "done") &&
    allTasksVerified(s) &&
    reviewsPass &&
    s.stages.verify.gate?.ok === true &&
    s.stages.review.gate?.ok === true &&
    s.stages.uat.gate?.ok === true &&
    s.acceptance.filter((criterion) => criterion.required).every((criterion) => criterion.status === "passed" && criterion.evidence.length > 0) &&
    s.approvals.some((approval) => approval.what === "uat" && approval.by === "user")
  );
}

interface TaskItemLike {
  agent?: string;
  model?: string;
  task: string;
}

/** Read task batch fields from a task tool call input, tolerantly. */
function extractTaskAgents(input: Record<string, unknown>): TaskItemLike[] {
  const tasks = input.tasks;
  if (!Array.isArray(tasks)) return [];
  const out: TaskItemLike[] = [];
  for (const value of tasks) {
    if (value && typeof value === "object") {
      const record = value as Record<string, unknown>;
      out.push({
        agent: typeof record.agent === "string" ? record.agent : undefined,
        model: typeof record.model === "string" ? record.model : undefined,
        task: typeof record.task === "string" ? record.task : "",
      });
    }
  }
  return out;
}

function modelMatchesLease(
  selector: string | undefined,
  pending: NonNullable<GsState["pendingAction"]>,
  index: { resolve: ModelResolver },
): boolean {
  if (!selector || !pending.modelId || !pending.modelFamily) return false;
  if (selector === pending.modelSelector || selector === pending.modelId) return true;
  const resolved = index.resolve(selector);
  return resolved?.id === pending.modelId && resolved.family === pending.modelFamily;
}

function promptHasExactLine(prompt: string, line: string): boolean {
  return prompt.split(/\r?\n/).some((candidate) => candidate.trim() === line);
}

function bashActionHash(command: string, root: string): string {
  return fnv1a(JSON.stringify({ kind: "bash", root: resolve(root), command }));
}

function verifyActionHash(command: VerifyCommand, root: string): string {
  return fnv1a(
    JSON.stringify({
      kind: "verify",
      program: command.program,
      args: command.args,
      cwd: resolve(root, command.cwd ?? "."),
    }),
  );
}


function consumeActionApproval(s: GsState, hash: string): void {
  const index = s.approvals.findIndex(
    (approval) => approval.what === "action" && approval.by === "user" && approval.note === hash,
  );
  if (index >= 0) s.approvals.splice(index, 1);
}

const SHELL_PROGRAMS: Record<string, true> = {
  bash: true,
  sh: true,
  zsh: true,
  fish: true,
  dash: true,
  cmd: true,
  "cmd.exe": true,
  powershell: true,
  pwsh: true,
};

function validateVerifyCommands(
  commands: VerifyCommand[],
  root: string,
): { ok: true; commands: VerifyCommand[] } | { ok: false; reason: string } {
  const projectRoot = resolve(root);
  const normalized: VerifyCommand[] = [];
  for (const command of commands) {
    if (SHELL_PROGRAMS[basename(command.program).toLowerCase()]) {
      return { ok: false, reason: `shell interpreter ${command.program} is not a direct-argv verification program` };
    }
    const cwd = resolve(projectRoot, command.cwd ?? ".");
    const rel = relative(projectRoot, cwd);
    if (rel === ".." || rel.startsWith("../") || isAbsolute(rel)) {
      return { ok: false, reason: `cwd ${command.cwd ?? cwd} escapes project root ${projectRoot}` };
    }
    normalized.push({ ...command, cwd });
  }
  return { ok: true, commands: normalized };
}

function classifyDirectCommand(command: VerifyCommand) {
  return classifyBashCommand([command.program, ...command.args].join(" "));
}

function verifyEvidenceRef(taskId: string, stdoutHash: string, stderrHash: string): string {
  return `verify:${taskId}:${stdoutHash}:${stderrHash}`;
}

function attachAutomaticVerifyEvidence(s: GsState, task: PlanTask, evidence: StageRecordVerifyEvidence[]): void {
  const allTasks = s.plan?.slices.flatMap((slice) => slice.tasks) ?? [];
  for (const acId of task.acceptance) {
    const ac = s.acceptance.find((criterion) => criterion.id === acId);
    if (!ac || ac.method !== "verify") continue;
    const complete = allTasks.filter((candidate) => candidate.acceptance.includes(acId)).every((candidate) => candidate.status === "passed");
    for (const commandEvidence of evidence) {
      const ref = verifyEvidenceRef(task.id, commandEvidence.stdoutHash, commandEvidence.stderrHash);
      if (!ac.evidence.some((item) => item.kind === "verify" && item.ref === ref)) {
        attachAcceptanceEvidence(
          s,
          acId,
          { kind: "verify", ref, detail: commandEvidence.output, at: commandEvidence.finishedAt },
          complete ? "passed" : "pending",
        );
      }
    }
  }
}

type StageRecordVerifyEvidence = GsState["stages"]["verify"]["verify"][number];

function evidenceCompatible(
  method: AcceptanceCriterion["method"],
  kind: AcceptanceCriterion["evidence"][number]["kind"],
): boolean {
  if (method === "verify") return kind === "verify";
  if (method === "review") return kind === "review";
  if (method === "uat") return kind === "browser" || kind === "smoke" || kind === "api";
  return kind === "browser" || kind === "smoke" || kind === "api" || kind === "note";
}

function objectiveEvidenceExists(
  s: GsState,
  kind: AcceptanceCriterion["evidence"][number]["kind"],
  ref: string,
  detail?: string,
): boolean {
  if (kind === "verify") {
    return s.acceptance.some((criterion) => criterion.evidence.some((item) => item.kind === "verify" && item.ref === ref));
  }
  if (kind === "review") {
    return s.stages.review.agentRuns.some(
      (run) =>
        run.exitCode === 0 &&
        run.agent === "gs-reviewer" &&
        run.verdict !== "incorrect" &&
        (run.blockingFindings ?? 0) === 0 &&
        (run.toolCallId === ref || run.batchToolCallId === ref || run.outputPath === ref),
    );
  }
  if (kind === "note") return false;
  return ref.trim().length > 0 && (detail?.trim().length ?? 0) > 0;
}

function invalidateApprovalsForReopen(s: GsState, target: Stage): void {
  const invalid = new Set<string>(["uat"]);
  if (target === "clarify" || target === "research" || target === "plan" || target === "plan_review") invalid.add("plan");
  if (target === "clarify") invalid.add("requirements");
  s.approvals = s.approvals.filter((approval) => !invalid.has(approval.what));
}
/** Coerce a tool_result `details` into the shape task-evidence expects. */
function normalizeTaskDetails(details: unknown): TaskDetailsLike {
  if (details && typeof details === "object" && "results" in details && Array.isArray(details.results)) {
    const results = details.results.filter(
      (value): value is SingleResultLike => typeof value === "object" && value !== null && !Array.isArray(value),
    );
    return { results };
  }
  return { results: [] };
}

interface NextResult {
  kind: "agent" | "verify" | "user" | "blocked";
  pending: GsState["pendingAction"] & object;
  instruction: string;
}

/** Compute the next required action + select the subagent model (pure-ish). */
function computeNext(s: GsState, cfg: GsConfig, idx: { resolve: ModelResolver; familyOf: (id: string) => string | undefined }): NextResult | undefined {
  const requirement = stageRequirement(s.stage);
  if (!requirement.agent) return undefined;

  let agent = requirement.agent as GsAgent;
  let role = requirement.modelRole as GsModelRole;
  if (s.stage === "review") {
    if (!latest(s, "review", "gs-reviewer")) {
      agent = "gs-reviewer";
      role = "review";
    } else if (s.risk.security && !latest(s, "review", "gs-security")) {
      agent = "gs-security";
      role = "security";
    } else {
      return undefined;
    }
  } else if (s.stage !== "implement" && latest(s, s.stage, agent)) {
    return undefined;
  }

  const excludeFamilies: string[] = [];
  if (s.stage === "plan_review") {
    const planner = latest(s, "plan", "gs-planner");
    if (planner?.resolvedModelFamily) excludeFamilies.push(planner.resolvedModelFamily);
  }
  if (s.stage === "review") {
    const implementation = latest(s, "implement", "gs-worker");
    if (implementation?.resolvedModelFamily) excludeFamilies.push(implementation.resolvedModelFamily);
  }

  const thinking = cfg.thinking[s.stage];
  const selection = selectStageModel(role, cfg, idx.resolve, { excludeFamilies, thinking });
  if (!selection.model) {
    return {
      kind: "blocked",
      pending: emptyPending(s),
      instruction: selection.finding?.message ?? `no model for role ${role}`,
    };
  }

  let taskIds: string[] | undefined;
  if (s.stage === "implement") {
    taskIds = currentWave(s).slice(0, cfg.limits.maxParallelWorkers);
    if (taskIds.length === 0) return undefined;
  }
  const taskSpecs = (taskIds ?? []).map((id) => {
    const task = findTask(s, id);
    return { id, acceptance: task?.acceptance ?? [], ownership: task?.ownership ?? [] };
  });
  const leaseRevision = s.revision + 1;
  const hash = assignmentHash({
    agent,
    taskIds,
    instruction: s.stage,
    modelId: selection.model.id,
    revision: leaseRevision,
    planHash: s.plan?.hash,
    taskSpecs,
  });
  const instruction =
    s.stage === "implement"
      ? [
          `Submit ONE task batch with agent ${agent} and model ${selection.model.requestedSelector} (${selection.model.id}).`,
          `Every task prompt must contain: Assignment hash: ${hash}`,
          ...taskSpecs.map((task) => {
            const planTask = findTask(s, task.id);
            return [
              `Task ID: ${task.id}`,
              `AC IDs: ${task.acceptance.join(", ")}`,
              `Allowed paths: ${task.ownership.join(", ")}`,
              `Skills: ${planTask?.skills.join(", ") || "none"}`,
            ].join("\n");
          }),
        ].join("\n")
      : [
          `Submit exactly one ${agent} task with model ${selection.model.requestedSelector} (${selection.model.id}, family ${selection.model.family}).`,
          `Its prompt must contain: Assignment hash: ${hash}`,
        ].join("\n");

  return {
    kind: "agent",
    pending: {
      id: `pa-${Date.now().toString(16)}`,
      stage: s.stage,
      kind: "agent",
      agent,
      modelRole: role,
      modelSelector: selection.model.requestedSelector,
      modelId: selection.model.id,
      modelFamily: selection.model.family,
      assignmentHash: hash,
      taskIds,
      leaseRevision,
      instruction,
      createdAt: new Date().toISOString(),
    },
    instruction,
  };
}

function emptyPending(s: GsState): NonNullable<GsState["pendingAction"]> {
  return { id: "none", stage: s.stage, kind: "user", instruction: "", createdAt: new Date().toISOString() };
}

function latest(s: GsState, stage: Stage, agent: string) {
  const runs = s.stages[stage].agentRuns.filter((r) => r.agent === agent && r.exitCode === 0);
  return runs.length ? runs[runs.length - 1] : undefined;
}

/** Tasks whose dependencies are all passed/skipped and are not yet done. */
function currentWave(s: GsState): string[] {
  const tasks = s.plan?.slices.flatMap((x) => x.tasks) ?? [];
  const doneIds = new Set(tasks.filter((t) => t.status === "passed" || t.status === "skipped").map((t) => t.id));
  const wave: string[] = [];
  for (const t of tasks) {
    if (t.status === "passed" || t.status === "skipped" || t.status === "blocked") continue;
    if (t.dependsOn.every((d) => doneIds.has(d))) wave.push(t.id);
  }
  return wave;
}

function slugify(goal: string): string {
  const base = goal
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
  return base || `run-${Date.now().toString(16)}`;
}
