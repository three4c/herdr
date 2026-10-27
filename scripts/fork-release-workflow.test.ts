import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const workflow: any = Bun.YAML.parse(readFileSync(new URL("../.github/workflows/fork-release.yml", import.meta.url), "utf8"));

describe("aira publishing boundary", () => {
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
