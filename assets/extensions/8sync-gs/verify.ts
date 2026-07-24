// 8sync-gs — deterministic command execution for gs_verify. Runs {program,args}
// DIRECTLY (no shell), so a plan-approved argv cannot smuggle shell tricks and
// the model cannot self-report a pass. Produces hashed evidence.

import { spawnSync } from "node:child_process";
import { fnv1a } from "./machine.ts";
import type { VerifyCommand, VerifyEvidence } from "./types.ts";

const MAX_OUTPUT = 2000;

export interface RunResult {
  ok: boolean;
  evidence: VerifyEvidence;
  output: string;
}

/** Execute one verify command directly (no shell). */
export function runVerifyCommand(cmd: VerifyCommand, cwd: string, timeoutSeconds: number): RunResult {
  const started = Date.now();
  const r = spawnSync(cmd.program, cmd.args, {
    cwd: cmd.cwd ? cmd.cwd : cwd,
    encoding: "utf8",
    timeout: (cmd.timeoutSeconds ?? timeoutSeconds) * 1000,
    shell: false,
  });
  const durationMs = Date.now() - started;
  const stdout = r.stdout ?? "";
  const stderr = r.stderr ?? "";
  const combined = `${stdout}${stderr}`.trim();
  const output = combined.length > MAX_OUTPUT ? `${combined.slice(0, MAX_OUTPUT)}\n…[truncated]` : combined;
  const exitCode = r.status ?? (r.error ? 127 : 0);
  const finishedAt = new Date().toISOString();
  return {
    ok: exitCode === 0 && !r.error,
    output,
    evidence: {
      command: cmd,
      exitCode,
      stdoutHash: fnv1a(stdout),
      stderrHash: fnv1a(stderr),
      durationMs,
      finishedAt,
      output,
    },
  };
}

export interface VerifyRunOutcome {
  passed: boolean;
  evidence: VerifyEvidence[];
  failureOutput: string;
  summary: string;
}

/** Run a task's verify commands; all must exit 0. */
export function runVerify(commands: VerifyCommand[], cwd: string, timeoutSeconds: number): VerifyRunOutcome {
  const evidence: VerifyEvidence[] = [];
  const failures: string[] = [];
  for (const cmd of commands) {
    const r = runVerifyCommand(cmd, cwd, timeoutSeconds);
    evidence.push(r.evidence);
    if (!r.ok) failures.push(`$ ${cmd.program} ${cmd.args.join(" ")}\n${r.output}`);
  }
  const passed = failures.length === 0;
  return {
    passed,
    evidence,
    failureOutput: failures.join("\n\n"),
    summary: passed
      ? `all ${commands.length} checks passed`
      : `${failures.length}/${commands.length} checks failed`,
  };
}
