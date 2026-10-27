import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const workflow: any = Bun.YAML.parse(readFileSync(new URL("../.github/workflows/fork-release.yml", import.meta.url), "utf8"));

describe("aira publishing boundary", () => {
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
printf '%s\\n' '[[{"tag_name":"aira-v1","draft":false},{"tag_name":"aira-v99","draft":true}],[{"tag_name":"aira-v2","draft":false}]]'
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
        const env = { ...process.env, PATH: `${dir}:${process.env.PATH}`, RUNNER_TEMP: dir, GITHUB_REPOSITORY: "three4c/herdr", AIRA_TEST_GH_FAIL: "" };
        const result = spawnSync("bash", ["-euo", "pipefail", "-c", command], { env, encoding: "utf8" });
        expect(result.stderr).toBe("");
        expect(result.status).toBe(0);
        expect(JSON.parse(readFileSync(join(dir, "published.json"), "utf8"))).toEqual(["aira-v1", "aira-v2"]);
        const failed = spawnSync("bash", ["-euo", "pipefail", "-c", command], { env: { ...env, AIRA_TEST_GH_FAIL: "1" }, encoding: "utf8" });
        expect(failed.status).toBe(9);
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
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

  test("only fork tags can publish, and both actors must be fork admins", () => {
    expect(workflow.on).toEqual({ push: { tags: ["aira-v*"] } });
    expect(workflow.permissions).toEqual({ contents: "read" });
    expect(workflow.concurrency).toEqual({ group: "aira-release", "cancel-in-progress": false });
    for (const [name, job] of Object.entries<any>(workflow.jobs)) {
      expect(job.if).toContain("github.repository == 'three4c/herdr'");
      if (name === "build") continue;
      expect(job.steps[0].run).toContain('test "$GITHUB_REPOSITORY" = "three4c/herdr"');
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
    expect(build.env.HERDR_FORK_RELEASE).toBe("${{ needs.preflight.outputs.number }}");
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
    expect(source.run).toContain('git cat-file -t "$GITHUB_REF_NAME"');
    expect(steps.find((step: any) => step.name === "Install Bun").with["bun-version"]).toBe("1.3.14");
    expect(steps.find((step: any) => step.name === "Run native checks without a Windows SDK").run.trim().split("\n")).toEqual(["just ci", "just docs-contract-test"]);
  });
});
