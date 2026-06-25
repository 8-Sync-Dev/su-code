# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
"""token_bench — prove codegraph (code-intel) token savings vs naive grep+read.

Run:  uv run scripts/token_bench.py <repo> [--terms a,b,c] [--k 8] [--ctx 40]
Needs: `codegraph` + `rg` on PATH, and the repo indexed (`codegraph index .`).

For each real symbol it compares the agent's two ways to answer
"where is X defined / what is it":
  OPTIMIZED = `codegraph query X` output  +  read one ~ctx-line slice at the hit
  NAIVE     = `rg -n X` output (all matches)  +  read the WHOLE definition file
Tokens are bytes/4 (same proxy both sides). Correctness = codegraph returns a
DEFINITION-kind node for the exact symbol whose location really contains it.
"""
import subprocess, os, re, sys, json, argparse

DEFK = {"function", "class", "constant", "interface", "method", "type_alias", "enum", "struct"}
CODE_EXT = r"(?:ts|tsx|js|jsx|py|go|rs|java|rb|c|cc|cpp|h|hpp)"
ANSI = re.compile(r"\x1b\[[0-9;]*m")  # codegraph emits color codes even when piped


def run(cmd, cwd=None, t=60):
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=t).stdout
    except Exception as e:
        return f"<err {e}>"


def rg(args, t=60):
    return run(["rg", "-nI", "-g", "!node_modules", "-g", "!dist", "-g", "!.next", "-g", "!.git", *args], t=t)


def auto_terms(repo, k):
    out = run(["rg", "-oNI", "-g", "!node_modules", "-g", "!*.test.*", "-g", "!dist", "-g", "!.next",
               r"export (?:async )?(?:function|class|const) (\w{6,})", repo], t=60)
    names = []
    for m in re.finditer(r"(?:function|class|const) (\w+)", out):
        if m.group(1) not in names:
            names.append(m.group(1))
        if len(names) >= k:
            break
    return names


def parse_def_hit(opt, term):
    """Return (relpath, line) of a DEFINITION-kind result for exactly `term`, else None.
    codegraph block format:  `KIND<ws>NAME (score%)\n  path:line\n  <snippet>`."""
    blocks = re.split(r"\n(?=\w+\s+\S+\s+\(\d+%\))", opt)
    for b in blocks:
        h = re.match(r"(\w+)\s+(\S+)\s+\(\d+%\)", b.strip())
        if not h:
            continue
        kind, name = h.group(1), h.group(2)
        loc = re.search(rf"([\w./-]+\.{CODE_EXT}):(\d+)", b)
        if kind in DEFK and name == term and loc:
            return loc.group(1), int(loc.group(2))
    return None


def slice_at(path, line, ctx):
    if not os.path.exists(path):
        return 0, ""
    L = open(path, errors="ignore").read().splitlines()
    s = "\n".join(L[max(0, line - 3):min(len(L), line + ctx)])
    return len(s), s


def bench(repo, terms, ctx):
    rows = []
    for term in terms:
        opt = ANSI.sub("", run(["codegraph", "query", term], cwd=repo, t=40))
        if "No results" in opt or not opt.strip():
            continue
        hit = parse_def_hit(opt, term)
        loc = hit or (lambda m: (m.group(1), int(m.group(2))) if m else None)(
            re.search(rf"([\w./-]+\.{CODE_EXT}):(\d+)", opt))
        if not loc:
            continue
        f, ln = loc
        sb, stxt = slice_at(os.path.join(repo, f), ln, ctx)
        opt_b = len(opt) + sb
        correct = bool(hit) and term in stxt          # def-kind hit AND location really contains it
        grep_b = len(rg([term, repo]))
        files = [x for x in run(["rg", "-lI", "-g", "!node_modules", "-g", "!dist", "-g", "!.next", term, repo]).splitlines()
                 if os.path.exists(x)]
        deff = next((x for x in files
                     if re.search(rf"(function|class|const|interface|type|def)\s+{re.escape(term)}\b",
                                  open(x, errors="ignore").read())), files[0] if files else None)
        naive_b = grep_b + (os.path.getsize(deff) if deff else 0)
        if naive_b and opt_b:
            rows.append({"term": term, "opt_b": opt_b, "naive_b": naive_b, "files": len(files), "correct": correct})
    return rows


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("repo")
    ap.add_argument("--terms", default="")
    ap.add_argument("--k", type=int, default=8)
    ap.add_argument("--ctx", type=int, default=40)
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()
    repo = os.path.abspath(os.path.expanduser(a.repo))
    terms = [t for t in a.terms.split(",") if t] or auto_terms(repo, a.k)
    rows = bench(repo, terms, a.ctx)
    if not rows:
        print("no measurable symbols — is the repo indexed? `codegraph index .`")
        sys.exit(1)
    to = sum(r["opt_b"] for r in rows)
    tn = sum(r["naive_b"] for r in rows)
    ok = sum(r["correct"] for r in rows)
    if a.json:
        print(json.dumps({"repo": repo, "rows": rows,
                          "reduction_pct": round(100 * (1 - to / tn), 1),
                          "correct": ok, "total": len(rows)}, indent=2))
        return
    print(f"token-bench · {repo}")
    for r in rows:
        red = 100 * (1 - r["opt_b"] / r["naive_b"])
        print(f"  {r['term']:<24} opt~{r['opt_b']//4:>5}tok  naive~{r['naive_b']//4:>7}tok  "
              f"({r['files']}f)  -{red:.1f}%  {'OK' if r['correct'] else 'check'}")
    print(f"  ── TOTAL  opt~{to//4}tok  naive~{tn//4}tok  →  -{100*(1-to/tn):.1f}% tokens · "
          f"correct-def {ok}/{len(rows)}")


if __name__ == "__main__":
    main()
