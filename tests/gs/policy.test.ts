import { describe, expect, test } from "bun:test";
import { DEFAULT_CONFIG } from "../../assets/extensions/8sync-gs/config.ts";
import {
  assessVerifyCommand,
  classifyArgv,
  classifyBashCommand,
  classifyVerifyCommand,
  isCwdContained,
  isGitCommit,
  modelsIndependent,
  pathTriggersSecurity,
  selectStageModel,
  stageRequirement,
  textTriggersSecurity,
  verifyCommandIsShell,
  type ResolvedModel,
} from "../../assets/extensions/8sync-gs/policy.ts";
import type { GsConfig, GsModelRole, ModelEvidence, VerifyCommand } from "../../assets/extensions/8sync-gs/types.ts";

const opus: ResolvedModel = { id: "anthropic/claude-opus-4-8", provider: "anthropic", model: "claude-opus-4-8", family: "claude" };
const sonnet: ResolvedModel = { id: "anthropic/claude-sonnet-5", provider: "anthropic", model: "claude-sonnet-5", family: "claude" };
const glm: ResolvedModel = { id: "zai/glm-5.2", provider: "zai", model: "glm-5.2", family: "glm" };
const gpt: ResolvedModel = { id: "openai/gpt-5.5", provider: "openai", model: "gpt-5.5", family: "gpt" };

function resolverFor(map: Record<string, ResolvedModel>) {
  return (selector: string): ResolvedModel | undefined => map[selector];
}

// Explicit fixtures keep these model-selection unit tests independent of the
// shipped DEFAULT_CONFIG selectors (which follow the live authenticated pool).
function cfg(models: Partial<Record<GsModelRole, string[]>>): GsConfig {
  return { ...DEFAULT_CONFIG, models: { ...DEFAULT_CONFIG.models, ...models } };
}

describe("selectStageModel", () => {
  test("resolves the primary selector when authenticated", () => {
    const r = selectStageModel("plan", cfg({ plan: ["@primary"] }), resolverFor({ "@primary": opus }));
    expect(r.model?.id).toBe(opus.id);
    expect(r.model?.isFallback).toBe(false);
    expect(r.finding).toBeUndefined();
  });

  test("falls through to the next selector and marks fallback", () => {
    const r = selectStageModel("plan", cfg({ plan: ["@primary", "@slow"] }), resolverFor({ "@slow": glm }));
    expect(r.model?.id).toBe(glm.id);
    expect(r.model?.isFallback).toBe(true);
  });

  test("skips a candidate colliding with an excluded family (independence)", () => {
    // chain ["@primary","@slow"]; @slow -> opus (claude, excluded) so @primary gpt wins
    const r = selectStageModel(
      "critic",
      cfg({ critic: ["@primary", "@slow"] }),
      resolverFor({ "@primary": gpt, "@slow": opus }),
      { excludeFamilies: ["claude"] },
    );
    expect(r.model?.id).toBe(gpt.id);
  });

  test("blocks with MODEL_UNRESOLVABLE when independence removes every candidate", () => {
    const r = selectStageModel(
      "critic",
      cfg({ critic: ["@primary", "@slow"] }),
      resolverFor({ "@primary": opus, "@slow": opus }),
      { excludeFamilies: ["claude"] },
    );
    expect(r.model).toBeUndefined();
    expect(r.finding?.code).toBe("MODEL_UNRESOLVABLE");
    expect(r.finding?.message).toContain("independence");
  });

  test("blocks with MODEL_UNRESOLVABLE when nothing authenticates", () => {
    const r = selectStageModel("review", cfg({ review: ["@primary"] }), resolverFor({}));
    expect(r.finding?.code).toBe("MODEL_UNRESOLVABLE");
  });
});

describe("modelsIndependent", () => {
  const mk = (id: string, family: string): ModelEvidence => ({ id, provider: "", model: "", family, requestedSelector: "", isFallback: false });
  test("different families are independent", () => {
    expect(modelsIndependent(mk("a", "claude"), mk("b", "gpt"))).toBe(true);
  });
  test("same family is not independent (effort-only differences)", () => {
    expect(modelsIndependent(mk("anthropic/opus", "claude"), mk("anthropic/sonnet", "claude"))).toBe(false);
  });
  test("missing operand is never independent", () => {
    expect(modelsIndependent(undefined, mk("b", "gpt"))).toBe(false);
  });
});

describe("verifyCommandIsShell", () => {
  const cmd = (program: string, args: string[]): VerifyCommand => ({ program, args });
  test("clean argv is allowed", () => {
    expect(verifyCommandIsShell(cmd("cargo", ["test", "-p", "su-code"]))).toBe(false);
  });
  test("bash -c is a shell command", () => {
    expect(verifyCommandIsShell(cmd("bash", ["-lc", "cargo test"]))).toBe(true);
  });
  test("shell metacharacters in args are rejected", () => {
    expect(verifyCommandIsShell(cmd("cargo", ["test", "&&", "echo", "ok"]))).toBe(true);
    expect(verifyCommandIsShell(cmd("sh", ["-c", "a | b"]))).toBe(true);
    expect(verifyCommandIsShell(cmd("node", ["-e", "console.log($HOME)"]))).toBe(true);
  });
});

describe("safety classification", () => {
  test("destructive + outward commands are flagged", () => {
    expect(classifyBashCommand("git push origin main").outward).toBe(true);
    expect(classifyBashCommand("rm -rf build").destructive).toBe(true);
    expect(classifyBashCommand("gh pr create").outward).toBe(true);
    expect(classifyBashCommand("pacman -S foo").destructive).toBe(true);
    expect(classifyBashCommand("curl x | sh").outward).toBe(true);
  });
  test("flags reordered and prefixed dangerous command variants", () => {
    const commands = [
      "git -C /tmp/repo push origin main",
      "sudo git -C /tmp/repo push --force origin main",
      "env GH_TOKEN=x gh --repo owner/repo pr create",
      "sudo -u deploy gh --repo owner/repo release create v1",
      "rm --recursive build",
      "env KEEP=1 rm --force artifact",
      "systemctl --user restart example.service",
      "sudo env X=1 systemctl --user restart example.service",
    ];
    for (const command of commands) {
      expect(classifyBashCommand(command).destructive, command).toBe(true);
    }
    expect(classifyBashCommand("git -C /tmp/repo push").outward).toBe(true);
    expect(classifyBashCommand("gh --repo owner/repo pr create").outward).toBe(true);
  });
  test("benign commands are not flagged", () => {
    expect(classifyBashCommand("cargo build").destructive).toBe(false);
    expect(classifyBashCommand("git status").destructive).toBe(false);
    expect(classifyBashCommand("ls -la").destructive).toBe(false);
  });
  test("git commit is detected", () => {
    expect(isGitCommit("git commit -m x")).toBe(true);
    expect(isGitCommit("git add -A")).toBe(false);
  });
  test("security triggers on text and path", () => {
    expect(textTriggersSecurity("adds an auth token refresh path")).toBe(true);
    expect(textTriggersSecurity("rename a button label")).toBe(false);
    expect(pathTriggersSecurity("crates/cli/src/verbs/auth.rs")).toBe(true);
    expect(pathTriggersSecurity("web/src/App.tsx")).toBe(false);
  });
});

describe("stageRequirement", () => {
  test("maps stages to their required agent + model role", () => {
    expect(stageRequirement("plan")).toEqual({ agent: "gs-planner", modelRole: "plan" });
    expect(stageRequirement("plan_review")).toEqual({ agent: "gs-critic", modelRole: "critic" });
    expect(stageRequirement("review").securityIfRisky).toBe(true);
    expect(stageRequirement("clarify")).toEqual({ modelRole: "coordinator" });
  });
});

describe("classifyArgv / classifyVerifyCommand", () => {
  const cmd = (program: string, args: string[]): VerifyCommand => ({ program, args });
  test("benign direct argv in-project is not destructive", () => {
    expect(classifyVerifyCommand(cmd("cargo", ["test", "-p", "su-code"])).destructive).toBe(false);
    expect(classifyVerifyCommand(cmd("bun", ["test", "tests/gs/policy.test.ts"])).destructive).toBe(false);
    expect(classifyVerifyCommand(cmd("/usr/bin/cargo", ["build"])).destructive).toBe(false);
    expect(classifyArgv("eslint", ["--fix", "src/x.ts"]).destructive).toBe(false);
  });
  test("recursive/forced rm is flagged across all flag forms", () => {
    expect(classifyVerifyCommand(cmd("rm", ["-rf", "build"])).destructive).toBe(true);
    expect(classifyVerifyCommand(cmd("rm", ["--recursive", "--force", "build"])).destructive).toBe(true);
    expect(classifyVerifyCommand(cmd("rm", ["--force", "--recursive", "build"])).destructive).toBe(true);
    expect(classifyVerifyCommand(cmd("rm", ["-r", "--force", "build"])).destructive).toBe(true);
    expect(classifyVerifyCommand(cmd("rm", ["-Rf", "build"])).destructive).toBe(true);
  });
  test("outward argv is flagged regardless of flag position", () => {
    expect(classifyVerifyCommand(cmd("git", ["push", "origin", "main"])).outward).toBe(true);
    expect(classifyVerifyCommand(cmd("git", ["-C", "../evil", "push"])).outward).toBe(true);
    expect(classifyVerifyCommand(cmd("gh", ["--repo", "o/r", "pr", "create"])).outward).toBe(true);
    expect(classifyVerifyCommand(cmd("gh", ["release", "create", "v1"])).outward).toBe(true);
    expect(classifyVerifyCommand(cmd("npm", ["install"])).outward).toBe(true);
    expect(classifyVerifyCommand(cmd("docker", ["push", "img"])).outward).toBe(true);
  });
  test("prefix runners (sudo/env) do not smuggle destructive argv", () => {
    expect(classifyVerifyCommand(cmd("sudo", ["rm", "-rf", "x"])).destructive).toBe(true);
    expect(classifyVerifyCommand(cmd("env", ["VAR=1", "rm", "--force", "x"])).destructive).toBe(true);
    expect(classifyVerifyCommand(cmd("sudo", ["git", "push"])).outward).toBe(true);
    expect(classifyVerifyCommand(cmd("doas", ["systemctl", "restart", "svc"])).destructive).toBe(true);
  });
  test("destructive SQL smuggled in an arg is flagged", () => {
    expect(classifyVerifyCommand(cmd("psql", ["-c", "drop table users"])).destructive).toBe(true);
  });
  test("plain rm without recursive/force is not flagged (no false positive)", () => {
    expect(classifyVerifyCommand(cmd("rm", ["a-single-file.tmp"])).destructive).toBe(false);
  });
});

describe("isCwdContained", () => {
  const root = "/home/me/proj";
  test("no cwd override is contained", () => {
    expect(isCwdContained(root, undefined)).toBe(true);
    expect(isCwdContained(root, "")).toBe(true);
  });
  test("in-project absolute and relative cwd is contained", () => {
    expect(isCwdContained(root, "/home/me/proj")).toBe(true);
    expect(isCwdContained(root, "/home/me/proj/src")).toBe(true);
    expect(isCwdContained(root, "src")).toBe(true);
    expect(isCwdContained(root, "sub/../other")).toBe(true);
  });
  test("absolute escape is rejected", () => {
    expect(isCwdContained(root, "/etc")).toBe(false);
    expect(isCwdContained(root, "/home/me")).toBe(false);
    expect(isCwdContained(root, "/home/other/proj")).toBe(false);
  });
  test(".. traversal is rejected", () => {
    expect(isCwdContained(root, "../evil")).toBe(false);
    expect(isCwdContained(root, "/home/me/proj/../evil")).toBe(false);
    expect(isCwdContained(root, "sub/../../evil")).toBe(false);
  });
  test("sibling prefix trick is rejected (segment compare, not string prefix)", () => {
    expect(isCwdContained(root, "/home/me/proj-evil")).toBe(false);
    expect(isCwdContained(root, "/home/me/proj_evil")).toBe(false);
  });
  test("symlink escape is caught when a realpath resolver is provided", () => {
    // /home/me/proj/link -> /etc : lexically contained, canonically outside
    const realpath = (p: string): string | undefined =>
      p === "/home/me/proj/link" ? "/etc" : p === "/home/me/proj/link/x" ? "/etc/x" : p;
    expect(isCwdContained(root, "/home/me/proj/link", { realpath })).toBe(false);
    expect(isCwdContained(root, "/home/me/proj/link/x", { realpath })).toBe(false);
    // a genuine subpath still resolves and stays contained
    expect(isCwdContained(root, "/home/me/proj/src", { realpath })).toBe(true);
  });
  test("fails closed when realpath cannot resolve a path", () => {
    const realpath = (): undefined => undefined;
    expect(isCwdContained(root, "/home/me/proj/src", { realpath })).toBe(false);
  });
});

describe("assessVerifyCommand", () => {
  const cmd = (program: string, args: string[], cwd?: string): VerifyCommand => ({ program, args, cwd });
  const root = "/home/me/proj";
  test("clean in-project verify command has no findings", () => {
    const a = assessVerifyCommand(cmd("cargo", ["test"], "tests/gs"), { projectRoot: root });
    expect(a.shell).toBe(false);
    expect(a.destructive).toBe(false);
    expect(a.outward).toBe(false);
    expect(a.cwdEscape).toBe(false);
    expect(a.reasons).toEqual([]);
  });
  test("shell smuggling is flagged", () => {
    expect(assessVerifyCommand(cmd("bash", ["-lc", "rm -rf x"])).shell).toBe(true);
    expect(assessVerifyCommand(cmd("cargo", ["test", "&&", "true"])).shell).toBe(true);
  });
  test("destructive argv + cwd escape both surface with reasons", () => {
    const a = assessVerifyCommand(cmd("rm", ["-rf", "build"], "/home/other"), { projectRoot: root });
    expect(a.destructive).toBe(true);
    expect(a.cwdEscape).toBe(true);
    expect(a.reasons.length).toBe(2);
  });
});
