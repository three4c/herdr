import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const workflow: any = Bun.YAML.parse(readFileSync(new URL("../.github/workflows/fork-release.yml", import.meta.url), "utf8"));

describe("aira publishing boundary", () => {
  test.skipIf(process.platform === "win32")("publication preserves tag notes and resumes only unpublished releases", () => {
    const dir = mkdtempSync(join(tmpdir(), "aira-release-publish-"));
    try {
      mkdirSync(join(dir, "scripts"));
      mkdirSync(join(dir, "release-assets"));
      writeFileSync(join(dir, "scripts/fork_release.py"), readFileSync(new URL("fork_release.py", import.meta.url)));
      for (const name of ["herdr-linux-x86_64", "herdr-linux-aarch64", "herdr-macos-x86_64", "herdr-macos-aarch64", "install.sh", "fork-latest.json", "SHA256SUMS", "LICENSE", "FORK-NOTICE.txt", "THIRD-PARTY-NOTICES.txt"]) {
        writeFileSync(join(dir, "release-assets", name), "fixture\n");
      }
      const notes = "release: aira v0.1.0\n\nLiteral `backticks`, $(commands), and non-ASCII: 私用.\n";
      writeFileSync(join(dir, "tag-notes.txt"), notes);
      const git = (args: string[]) => {
        const result = spawnSync("git", args, { cwd: dir, encoding: "utf8" });
        expect(result.status, result.stderr).toBe(0);
      };
      git(["init", "--quiet"]);
      git(["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "--allow-empty", "-m", "fixture"]);
      git(["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "tag", "--annotate", "--no-sign", "aira-v0.1.0", "--file", "tag-notes.txt"]);
      git(["update-ref", "refs/remotes/origin/master", "HEAD"]);
      const expectedNotes = spawnSync("git", ["for-each-ref", "--format=%(contents)", "refs/tags/aira-v0.1.0"], { cwd: dir, encoding: "utf8" }).stdout;
      expect(expectedNotes.trimEnd()).toBe(notes.trimEnd());
      writeFileSync(join(dir, "gh"), `#!/usr/bin/env python3
import json, os, pathlib, sys
root = pathlib.Path(os.environ["RUNNER_TEMP"])
scenario = os.environ["AIRA_TEST_SCENARIO"]
args = sys.argv[1:]
with (root / "calls.jsonl").open("a") as log:
    log.write(json.dumps(args) + "\\n")
if args[0] == "api":
    if scenario == "api-failure": sys.exit(9)
    print(json.dumps([[{"tag_name": "aira-v0.2.0", "draft": False}] if scenario == "newer" else []]))
    sys.exit(0)
assert args[:1] == ["release"]
assert args[args.index("--repo") + 1] == "three4c/herdr-aira"
assert args[2] == "aira-v0.1.0"
action = args[1]
if action == "view":
    if scenario in ("draft", "published"):
        print(json.dumps({"isDraft": scenario == "draft"}))
        sys.exit(0)
    sys.exit(1)
if action == "create":
    # GitHub CLI rejects this combination before contacting its API.
    if "--notes-from-tag" in args:
        sys.exit("using --notes-from-tag with --repo is not supported")
    assert "--draft" in args and "--verify-tag" in args
    source = pathlib.Path(args[args.index("--notes-file") + 1])
    (root / "created-notes.txt").write_bytes(source.read_bytes())
elif action == "upload":
    if scenario == "upload-failure": sys.exit(8)
    assets = [pathlib.Path(arg) for arg in args[3:] if arg.startswith("release-assets/")]
    assert len(assets) == 10 and all(asset.is_file() for asset in assets)
elif action == "edit":
    assert "--draft=false" in args and "--prerelease=false" in args and "--latest" in args
else:
    raise AssertionError(args)
`);
      chmodSync(join(dir, "gh"), 0o700);
      const preflight = spawnSync("bash", ["-euo", "pipefail", "-c", workflow.jobs.preflight.steps.find((step: any) => step.id === "release").run], {
        cwd: dir,
        env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, RUNNER_TEMP: dir, GITHUB_REPOSITORY: "three4c/herdr-aira", RELEASE_TAG: "aira-v0.1.0", GITHUB_OUTPUT: join(dir, "github-output"), AIRA_TEST_SCENARIO: "new" },
        encoding: "utf8",
      });
      expect(preflight.status, preflight.stderr).toBe(0);
      expect(readFileSync(join(dir, "github-output"), "utf8")).toContain("version=0.1.0\ntag=aira-v0.1.0\ncommit=");
      const script = workflow.jobs.publish.steps.find((step: any) => step.name === "Publish complete fork release").run;
      const cases: Array<[string, number, string[]]> = [
        ["new", 0, ["view", "create", "upload", "edit"]],
        ["draft", 0, ["view", "upload", "edit"]],
        ["published", 1, ["view"]],
        ["newer", 1, []],
        ["api-failure", 9, []],
        ["upload-failure", 8, ["view", "create", "upload"]],
      ];
      for (const [scenario, status, actions] of cases) {
        writeFileSync(join(dir, "calls.jsonl"), "");
        rmSync(join(dir, "created-notes.txt"), { force: true });
        const result = spawnSync("bash", ["-euo", "pipefail", "-c", script], {
          cwd: dir,
          env: { ...process.env, PATH: `${dir}:${process.env.PATH}`, RUNNER_TEMP: dir, GITHUB_REPOSITORY: "three4c/herdr-aira", RELEASE_TAG: "aira-v0.1.0", AIRA_TEST_SCENARIO: scenario },
          encoding: "utf8",
        });
        expect(result.status, `${scenario}: ${result.stderr}`).toBe(status);
        const calls = readFileSync(join(dir, "calls.jsonl"), "utf8").trim().split("\n").map((line) => JSON.parse(line));
        expect(calls.filter((args) => args[0] === "release").map((args) => args[1])).toEqual([...actions]);
        if (actions.includes("create")) {
          expect(readFileSync(join(dir, "created-notes.txt"), "utf8")).toBe(expectedNotes);
        }
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  // Both publishing jobs execute these commands on POSIX runners.
  test.skipIf(process.platform === "win32")("published release discovery handles pagination, drafts, and API failure", () => {
    const dir = mkdtempSync(join(tmpdir(), "aira-release-discovery-"));
    try {
      writeFileSync(join(dir, "gh"), `#!/bin/sh
[ "\${AIRA_TEST_GH_FAIL:-}" != 1 ] || exit 9
slurp=no
filter=no
for argument do
  case "$argument" in
    --slurp) slurp=yes ;;
    --jq|--template) filter=yes ;;
  esac
done
[ "$slurp:$filter" != yes:yes ] || { echo '--slurp cannot be combined with --jq or --template' >&2; exit 2; }
printf '%s\\n' '[[{"tag_name":"aira-v0.0.9","draft":false},{"tag_name":"aira-v99.0.0","draft":true}],[{"tag_name":"aira-v0.1.0","draft":false}]]'
`);
      chmodSync(join(dir, "gh"), 0o700);
      const scripts = [
        workflow.jobs.preflight.steps.find((step: any) => step.id === "release").run,
        workflow.jobs.publish.steps.find((step: any) => step.name === "Publish complete fork release").run,
      ];
      for (const script of scripts) {
        const lines = script.split("\n");
        const start = lines.findIndex((line: string) => line.trimStart().startsWith("gh api ") && line.includes("/releases?per_page="));
        expect(start).toBeGreaterThanOrEqual(0);
        let end = start;
        while (end < lines.length && !lines[end].includes('> "$RUNNER_TEMP/published.json"')) end++;
        expect(end).toBeLessThan(lines.length);
        const command = lines.slice(start, end + 1).join("\n");
        const env = { ...process.env, PATH: `${dir}:${process.env.PATH}`, RUNNER_TEMP: dir, GITHUB_REPOSITORY: "three4c/herdr-aira", AIRA_TEST_GH_FAIL: "" };
        const result = spawnSync("bash", ["-euo", "pipefail", "-c", command], { env, encoding: "utf8" });
        expect(result.stderr).toBe("");
        expect(result.status).toBe(0);
        expect(JSON.parse(readFileSync(join(dir, "published.json"), "utf8"))).toEqual(["aira-v0.0.9", "aira-v0.1.0"]);
        const failed = spawnSync("bash", ["-euo", "pipefail", "-c", command], { env: { ...env, AIRA_TEST_GH_FAIL: "1" }, encoding: "utf8" });
        expect(failed.status).toBe(9);
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("manual requests preserve source pinning and tag pushes survive the skipped request job", () => {
    expect(workflow.jobs.request.if).toContain("github.ref == 'refs/heads/master'");
    const request = workflow.jobs.request.steps.find((step: any) => step.id === "request");
    expect(request.env.REQUEST_VERSION).toBe("${{ inputs.version }}");
    expect(request.env.REQUEST_COMMIT).toBe("${{ inputs.source_sha }}");
    expect(request.run).not.toContain("${{");
    expect(workflow.jobs.preflight.if).toContain("always()");
    for (const name of ["build", "publish", "verify"]) {
      const job = workflow.jobs[name];
      expect(job.if).toContain("!cancelled()");
      expect(job.if).toContain("needs.preflight.result == 'success'");
      const checkout = job.steps.find((step: any) => step.uses?.startsWith("actions/checkout@"));
      expect(checkout.with.ref).toBe("${{ needs.preflight.outputs.commit }}");
    }
    expect(workflow.jobs.publish.if).toContain("needs.build.result == 'success'");
    expect(workflow.jobs.verify.if).toContain("needs.publish.result == 'success'");
  });

  test("private website deployments stay in the canonical repository", () => {
    const website: any = Bun.YAML.parse(readFileSync(new URL("../.github/workflows/website-deploy.yml", import.meta.url), "utf8"));
    // A job-level gate covers both master pushes and manual dispatches.
    for (const job of Object.values<any>(website.jobs)) {
      expect(job.if).toBe("github.repository == 'herdrdev/herdr'");
    }
    expect(website.jobs.trigger.steps[0].run).toContain("HERDR_WEBSITE_DEPLOY_HOOK is not configured");
    expect(website.jobs.trigger.steps[0].run).toContain("exit 1");
  });

  test("only explicit fork release requests can publish, and both actors must be fork admins", () => {
    expect(workflow.on.push).toEqual({ tags: ["aira-v*"] });
    expect(workflow.on.workflow_dispatch.inputs.version.required).toBe(true);
    expect(Object.keys(workflow.on).sort()).toEqual(["push", "workflow_dispatch"]);
    expect(workflow.permissions).toEqual({ contents: "read" });
    expect(workflow.concurrency).toEqual({ group: "aira-release", "cancel-in-progress": false });
    for (const [name, job] of Object.entries<any>(workflow.jobs)) {
      expect(job.if).toContain("github.repository == 'three4c/herdr-aira'");
      if (name === "build") continue;
      expect(job.steps[0].run).toContain('test "$GITHUB_REPOSITORY" = "three4c/herdr-aira"');
      expect(job.steps[0].run).toContain('"$GITHUB_ACTOR" "$GITHUB_TRIGGERING_ACTOR"');
      expect(job.steps[0].run).toContain('test "$permission" = admin');
    }
    expect(workflow.jobs.publish.permissions).toEqual({ contents: "write" });
  });

  test("all four binaries pass checks before a complete release becomes latest", () => {
    const build = workflow.jobs.build;
    expect(build.needs).toBe("preflight");
    expect(build.strategy.matrix.include.map((entry: any) => entry.asset).sort()).toEqual([
      "herdr-linux-aarch64", "herdr-linux-x86_64", "herdr-macos-aarch64", "herdr-macos-x86_64",
    ]);
    expect(build.env.HERDR_FORK_VERSION).toBe("${{ needs.preflight.outputs.version }}");
    expect(workflow.jobs.publish.needs).toEqual(["preflight", "build"]);
    const runs = workflow.jobs.publish.steps.map((step: any) => step.run ?? "").join("\n");
    expect(runs).toContain("scripts/fork_release.py check");
    expect(runs).toContain("--draft --title");
    expect(runs).toContain("gh release upload");
    expect(runs).toContain("--draft=false --prerelease=false --latest");
    expect(runs.indexOf("gh release upload")).toBeLessThan(runs.indexOf("--draft=false"));
    expect(runs).not.toContain("herdrdev/");
    expect(runs).not.toContain("distribution/latest.json");
    expect(runs).toContain("scripts/fork_licenses.py check");
  });

  test("publication requires master ancestry, an annotated tag, and the CI Bun version", () => {
    const steps = workflow.jobs.preflight.steps;
    const source = steps.find((step: any) => step.id === "release");
    expect(source.run).toContain("git merge-base --is-ancestor HEAD origin/master");
    expect(source.run).toContain('git cat-file -t "$RELEASE_TAG"');
    expect(steps.find((step: any) => step.name === "Install Bun").with["bun-version"]).toBe("1.3.14");
    expect(steps.find((step: any) => step.name === "Run native checks without a Windows SDK").run.trim().split("\n")).toEqual(["just ci", "just docs-contract-test"]);
  });
});
