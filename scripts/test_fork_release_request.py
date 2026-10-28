from __future__ import annotations

import json
import os
import struct
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import fork_release, fork_release_request as request, verify_fork_release as verify

ROOT = Path(__file__).resolve().parents[1]
COMMIT = "a" * 40


class ReleaseRequestTests(unittest.TestCase):
    def setUp(self):
        self.env = mock.patch.dict(os.environ, {
            "GITHUB_REPOSITORY": "three4c/herdr-aira", "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_REF": "refs/heads/master", "GITHUB_SHA": COMMIT,
            "GITHUB_ACTOR": "three4c", "GITHUB_TRIGGERING_ACTOR": "three4c",
        })
        self.env.start()
        self.addCleanup(self.env.stop)
        self.tags = ["aira-v0.1.0"]
        self.writes = []
        self.run = {"id": 7, "head_sha": COMMIT, "head_branch": "master", "event": "push",
                    "status": "completed", "conclusion": "success"}
        self.jobs = [{"id": i, "name": name, "conclusion": "success"} for i, name in enumerate(sorted(request.CI_JOBS))]
        self.existing = None
        patch = mock.patch.object(request, "api", side_effect=self.api)
        patch.start()
        self.addCleanup(patch.stop)

    def api(self, path, *, data=None, pages=False, missing=False):
        if data is not None:
            self.writes.append((path, data))
            return {"sha": "b" * 40}
        if path.startswith("collaborators/"):
            return {"permission": "admin"}
        if path.startswith("releases?"):
            return [[{"tag_name": tag, "draft": False} for tag in self.tags]]
        if path.startswith("actions/workflows/"):
            return [{"workflow_runs": [self.run] if "/ci.yml/" in path else []}]
        if path.startswith("actions/runs/"):
            return [{"jobs": self.jobs}]
        if path.startswith("git/ref/"):
            return self.existing
        if path.startswith("git/tags/"):
            return {"tag": "aira-v0.1.1", "object": {"type": "commit", "sha": COMMIT}}
        raise AssertionError(path)

    def test_success_pins_the_reviewed_source_and_creates_an_annotated_tag(self):
        plan = request.request("0.1.1", COMMIT)
        self.assertEqual(plan["commit"], COMMIT)
        self.assertEqual(plan["baseline"], "aira-v0.1.0")
        self.assertEqual(self.writes[0][0], "git/tags")
        self.assertEqual(self.writes[0][1]["object"], COMMIT)
        self.assertEqual(self.writes[1], ("git/refs", {"ref": "refs/tags/aira-v0.1.1", "sha": "b" * 40}))

    def test_dry_run_performs_no_writes(self):
        request.request("0.1.1", COMMIT, dry_run=True)
        self.assertEqual(self.writes, [])

    def test_changed_source_wrong_repository_and_wrong_branch_are_rejected(self):
        for changes in ({"GITHUB_SHA": "c" * 40}, {"GITHUB_REPOSITORY": "herdrdev/herdr"},
                        {"GITHUB_REF": "refs/heads/topic"}, {"GITHUB_EVENT_NAME": "push"}):
            with self.subTest(changes=changes), mock.patch.dict(os.environ, changes), self.assertRaises(ValueError):
                request.request("0.1.1", COMMIT)
        self.assertEqual(self.writes, [])

    def test_non_admin_rerun_actor_is_rejected(self):
        original = self.api
        def api(path, **kwargs):
            return {"permission": "write"} if path == "collaborators/writer/permission" else original(path, **kwargs)
        with mock.patch.dict(os.environ, {"GITHUB_TRIGGERING_ACTOR": "writer"}), mock.patch.object(request, "api", side_effect=api), self.assertRaises(ValueError):
            request.request("0.1.1", COMMIT)
        self.assertEqual(self.writes, [])

    def test_invalid_equal_or_older_versions_cannot_create_tags(self):
        for version in ("0.1.0", "0.0.9", "0.01.1", "0.1.1;echo bad", "1"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                request.request(version, COMMIT)
        self.assertEqual(self.writes, [])

    def test_failed_cancelled_missing_and_skipped_ci_do_not_qualify(self):
        for conclusion in ("failure", "cancelled", "skipped"):
            self.run["conclusion"] = conclusion
            with self.subTest(conclusion=conclusion), self.assertRaises(ValueError):
                request.request("0.1.1", COMMIT)
        self.run["conclusion"] = "success"
        self.jobs[0]["conclusion"] = "skipped"
        with self.assertRaises(ValueError):
            request.request("0.1.1", COMMIT)
        self.jobs = []
        with self.assertRaises(ValueError):
            request.request("0.1.1", COMMIT)
        self.assertEqual(self.writes, [])

    def test_only_matching_master_pushes_count_and_wait_is_bounded(self):
        for field, value in (("head_sha", "d" * 40), ("event", "pull_request"), ("head_branch", "topic")):
            with self.subTest(field=field), mock.patch.dict(self.run, {field: value}), self.assertRaisesRegex(ValueError, "timed out"):
                request.wait_for_ci(COMMIT, timeout=0)

    def test_path_filtered_checks_must_succeed_when_they_run(self):
        original = self.api
        def api(path, **kwargs):
            if "/nix.yml/" in path:
                return [{"workflow_runs": [{**self.run, "id": 8, "conclusion": "failure"}]}]
            return original(path, **kwargs)
        with mock.patch.object(request, "api", side_effect=api), self.assertRaisesRegex(ValueError, "nix.yml"):
            request.request("0.1.1", COMMIT)
        self.assertEqual(self.writes, [])

    def test_waiting_and_successful_retry_use_the_latest_job_result(self):
        with mock.patch.object(request, "ci_state", side_effect=[(False, []), (True, ["receipt"])]), mock.patch.object(request.time, "sleep") as sleep:
            self.assertEqual(request.wait_for_ci(COMMIT), ["receipt"])
            sleep.assert_called_once()
        old = {**self.jobs[0], "id": -1, "conclusion": "failure"}
        self.jobs.insert(0, old)
        request.request("0.1.1", COMMIT)

    def test_newer_release_appearing_during_ci_prevents_tag_creation(self):
        def wait(commit):
            self.tags.append("aira-v0.1.2")
            return []
        with mock.patch.object(request, "wait_for_ci", side_effect=wait), self.assertRaises(ValueError):
            request.request("0.1.1", COMMIT)
        self.assertEqual(self.writes, [])

    def test_existing_annotated_tag_can_resume_but_lightweight_or_moved_cannot(self):
        self.existing = {"object": {"type": "tag", "sha": "b" * 40}}
        request.request("0.1.1", COMMIT)
        self.assertEqual(self.writes, [])
        with self.assertRaisesRegex(ValueError, "different source"):
            request.ensure_tag("aira-v0.1.1", "c" * 40)
        self.existing["object"]["type"] = "commit"
        with self.assertRaisesRegex(ValueError, "annotated"):
            request.request("0.1.1", COMMIT)


class ApiFailureTests(unittest.TestCase):
    def test_only_404_can_mean_an_absent_tag_and_pagination_uses_valid_flags(self):
        for status in (403, 500, 404):
            result = mock.Mock(returncode=1, stdout=json.dumps({"status": str(status)}), stderr="failure")
            with mock.patch.object(request.subprocess, "run", return_value=result):
                if status == 404:
                    self.assertIsNone(request.api("git/ref/tags/aira-v0.1.1", missing=True))
                else:
                    with self.assertRaises(RuntimeError):
                        request.api("git/ref/tags/aira-v0.1.1", missing=True)
        result = mock.Mock(returncode=0, stdout="[[]]")
        with mock.patch.object(request.subprocess, "run", return_value=result) as run:
            request.api("releases?per_page=100", pages=True)
            self.assertIn("--slurp", run.call_args.args[0])
            self.assertNotIn("--jq", run.call_args.args[0])


class PublicAssetTests(unittest.TestCase):
    def test_integrity_source_urls_and_platform_headers(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets, output = Path(tmp) / "assets", Path(tmp) / "release"
            assets.mkdir()
            for target in fork_release.TARGETS:
                data = bytearray(24)
                if target.startswith("linux"):
                    data[:6] = b"\x7fELF\x02\x01"
                    struct.pack_into("<H", data, 18, 62 if target.endswith("x86_64") else 183)
                else:
                    struct.pack_into("<II", data, 0, 0xfeedfacf, 0x1000007 if target.endswith("x86_64") else 0x100000c)
                (assets / ("herdr-" + target)).write_bytes(data)
            manifest = fork_release.prepare("aira-v0.1.1", assets, output, ROOT)
            verify.verify_assets(output, "aira-v0.1.1", manifest["commit"])
            with self.assertRaises(ValueError):
                verify.verify_assets(output, "aira-v0.1.1", "c" * 40)
            binary = output / "herdr-linux-x86_64"
            binary.write_bytes(b"corrupt")
            with self.assertRaises(ValueError):
                verify.verify_assets(output, "aira-v0.1.1", manifest["commit"])

    def test_a_wrong_platform_header_is_rejected_even_with_matching_checksums(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets, output = Path(tmp) / "assets", Path(tmp) / "release"
            assets.mkdir()
            for target in fork_release.TARGETS:
                (assets / ("herdr-" + target)).write_bytes(b"not-an-executable-header")
            manifest = fork_release.prepare("aira-v0.1.1", assets, output, ROOT)
            with self.assertRaises(ValueError):
                verify.verify_assets(output, "aira-v0.1.1", manifest["commit"])


if __name__ == "__main__":
    unittest.main()
