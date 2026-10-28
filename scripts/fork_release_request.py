#!/usr/bin/env python3
"""Validate a manual Aira request, wait for master CI, then create its tag."""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import time
from pathlib import Path

try:
    from .fork_release import REPOSITORY, require_newer, version_parts
except ImportError:
    from fork_release import REPOSITORY, require_newer, version_parts

CI_JOBS = {"conventional-commits", "check (ubuntu-latest)", "check (macos-latest)",
           "check (windows-latest)", "Windows ConPTY package"}


def api(path: str, *, data: dict | None = None, pages: bool = False, missing: bool = False):
    command = ["gh", "api", f"repos/{REPOSITORY}/{path}"]
    if pages:
        command += ["--paginate", "--slurp"]
    if data is not None:
        command += ["--method", "POST", "--input", "-"]
    result = subprocess.run(command, input=json.dumps(data) if data is not None else None,
                            capture_output=True, text=True, check=False)
    if result.returncode:
        try:
            error = json.loads(result.stdout)
        except ValueError:
            error = {}
        if missing and str(error.get("status")) == "404":
            return None
        raise RuntimeError(f"GitHub API failed for {path}: {result.stderr.strip()}")
    return json.loads(result.stdout)


def published_tags() -> list[str]:
    return [release["tag_name"] for page in api("releases?per_page=100", pages=True)
            for release in page if not release["draft"]]


def ci_state(commit: str) -> tuple[bool, list[str]]:
    ready, urls = True, []
    for workflow in ("ci.yml", "distribution.yml", "nix.yml"):
        pages = api(f"actions/workflows/{workflow}/runs?head_sha={commit}&event=push&branch=master&per_page=100", pages=True)
        runs = [run for page in pages for run in page["workflow_runs"]
                if run["head_sha"] == commit and run["head_branch"] == "master" and run["event"] == "push"]
        if not runs:
            # Distribution and Nix use path filters; they need not run on every commit.
            ready = ready and workflow != "ci.yml"
            continue
        run = max(runs, key=lambda item: item["id"])
        urls.append(f"https://github.com/{REPOSITORY}/actions/runs/{run['id']}")
        if run["status"] != "completed":
            ready = False
            continue
        if run["conclusion"] != "success":
            raise ValueError(f"{workflow} did not succeed for {commit}: {run['conclusion']}")
        if workflow == "ci.yml":
            jobs = [job for page in api(f"actions/runs/{run['id']}/jobs?filter=all&per_page=100", pages=True)
                    for job in page["jobs"]]
            latest = {}
            for job in sorted(jobs, key=lambda item: item["id"]):
                latest[job["name"]] = job
            if not CI_JOBS.issubset(latest) or any(latest[name]["conclusion"] != "success" for name in CI_JOBS):
                raise ValueError("required master CI jobs must all succeed; skipped jobs do not qualify")
    return ready, urls


def wait_for_ci(commit: str, timeout: float = 1800) -> list[str]:
    deadline = time.monotonic() + timeout
    while True:
        ready, urls = ci_state(commit)
        if ready:
            return urls
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise ValueError(f"timed out waiting for master CI at {commit}")
        print(f"Waiting for master CI at {commit}", flush=True)
        time.sleep(min(15, remaining))


def ensure_tag(tag: str, commit: str) -> None:
    existing = api(f"git/ref/tags/{tag}", missing=True)
    if existing is not None:
        if existing["object"]["type"] != "tag":
            raise ValueError("existing release tag must be annotated")
        annotated = api(f"git/tags/{existing['object']['sha']}")
        if (annotated["tag"] != tag or annotated["object"]["type"] != "commit"
                or annotated["object"]["sha"] != commit):
            raise ValueError("existing release tag points at different source; never move it")
        return
    annotated = api("git/tags", data={"tag": tag, "message": f"release: aira v{tag.removeprefix('aira-v')}\n\nSource: {commit}\n",
                                      "object": commit, "type": "commit"})
    api("git/refs", data={"ref": f"refs/tags/{tag}", "sha": annotated["sha"]})


def request(version: str, expected_commit: str = "", *, dry_run: bool = False) -> dict:
    if (os.environ.get("GITHUB_REPOSITORY") != REPOSITORY
            or os.environ.get("GITHUB_EVENT_NAME") != "workflow_dispatch"
            or os.environ.get("GITHUB_REF") != "refs/heads/master"):
        raise ValueError("manual Aira releases must run from three4c/herdr-aira master")
    commit = os.environ.get("GITHUB_SHA", "")
    if not re.fullmatch(r"[0-9a-f]{40}", commit) or (expected_commit and expected_commit != commit):
        raise ValueError("master changed since version review; review the new source before releasing")
    for name in ("GITHUB_ACTOR", "GITHUB_TRIGGERING_ACTOR"):
        actor = os.environ.get(name, "")
        if not re.fullmatch(r"[A-Za-z0-9-]+", actor) or api(f"collaborators/{actor}/permission")["permission"] != "admin":
            raise ValueError("both release actors must be fork admins")
    tag = f"aira-v{version}"
    tags = published_tags()
    require_newer(tag, tags)
    urls = wait_for_ci(commit)
    # A request can wait for CI; recheck publication immediately before creating a tag.
    require_newer(tag, published_tags())
    if not dry_run:
        ensure_tag(tag, commit)
    baseline = max((other for other in tags if other.startswith("aira-v")),
                   key=lambda other: version_parts(other.removeprefix("aira-v")), default="")
    return {"version": version, "tag": tag, "commit": commit, "baseline": baseline, "ci": urls}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", default=os.environ.get("REQUEST_VERSION", ""))
    parser.add_argument("--commit", default=os.environ.get("REQUEST_COMMIT", ""))
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    plan = request(args.version, args.commit, dry_run=args.dry_run)
    if not args.dry_run:
        with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
            for key in ("version", "tag", "commit", "baseline"):
                output.write(f"{key}={plan[key]}\n")
    print(json.dumps(plan, indent=2))


if __name__ == "__main__":
    main()
