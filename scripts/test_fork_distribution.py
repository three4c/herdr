from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts import fork_release
from scripts import fork_licenses
from scripts import test_unix_installer as upstream

ROOT = Path(__file__).resolve().parents[1]
INSTALLER = ROOT / "distribution/fork/install.sh"
MANIFEST_URL = "https://github.com/three4c/herdr/releases/latest/download/fork-latest.json"


@unittest.skipUnless(os.name == "posix", "Unix installer requires POSIX")
class ForkInstallerTests(upstream.UnixInstallerTests):
    """Exercise the fork installer using temporary files and a network substitute."""

    def setUp(self):
        super().setUp()
        self.manifest_changes = {}
        self._write_executable("uname", """#!/bin/sh
case "$1" in
  -s) echo "${FAKE_SYSTEM:-Linux}" ;;
  -m) echo "${FAKE_ARCH:-x86_64}" ;;
  -o)
    [ "${FAKE_UNAME_OS_UNAVAILABLE:-}" != 1 ] || exit 1
    echo "${FAKE_UNAME_OS:-GNU/Linux}" ;;
  *) exit 1 ;;
esac
""")
        self._write_executable("curl", """#!/bin/sh
out=''
previous=''
url=''
for argument in "$@"; do
  if [ "$previous" = -o ]; then out="$argument"; fi
  case "$argument" in https://*) url="$argument" ;; esac
  previous="$argument"
done
printf '%s\n' "$url" >> "$FAKE_CURL_MARKER"
case "$url" in
  https://github.com/three4c/herdr/releases/*) ;;
  *) exit 99 ;;
esac
if [ -n "$out" ]; then
  [ "${FAKE_FAIL:-}" != asset ] || exit 22
  cp "$FAKE_PAYLOAD" "$out"
else
  [ "${FAKE_FAIL:-}" != manifest ] || exit 22
  cat "$FAKE_MANIFEST"
fi
""")

    def _write_manifest(self, checksum):
        manifest = {
            "distribution": "three4c/herdr",
            "release": 12,
            "version": "0.9.5-aira.12",
            "assets": {target: f"https://github.com/three4c/herdr/releases/download/aira-v12/herdr-{target}" for target in fork_release.TARGETS},
            "sha256": {target: checksum for target in fork_release.TARGETS} if checksum else {},
            **self.manifest_changes,
        }
        path = self.root / "latest.json"
        path.write_text(json.dumps(manifest, indent=2) + "\n")
        return path

    def _run_installer(self, *args, **kwargs):
        with mock.patch.object(upstream, "INSTALLER", INSTALLER):
            return super()._run_installer(*args, **kwargs)

    def test_platform_matrix_and_fork_only_urls(self):
        for system, arch, target in (
            ("Darwin", "arm64", "macos-aarch64"),
            ("Darwin", "x86_64", "macos-x86_64"),
            ("Linux", "aarch64", "linux-aarch64"),
            ("Linux", "amd64", "linux-x86_64"),
        ):
            with self.subTest(target=target):
                (self.bin_dir / "sha256sum").unlink(missing_ok=True)
                marker = self.root / "curl-called"
                marker.unlink(missing_ok=True)
                result = self._run_installer(self.expected_sha256, extra_env={"FAKE_SYSTEM": system, "FAKE_ARCH": arch})
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(marker.read_text().splitlines(), [MANIFEST_URL, f"https://github.com/three4c/herdr/releases/download/aira-v12/herdr-{target}"])
                self.assertTrue(os.access(self.install_dir / "herdr", os.X_OK))

    def test_unsupported_platform_never_downloads(self):
        for extra in ({"FAKE_SYSTEM": "FreeBSD"}, {"FAKE_SYSTEM": "Windows_NT"}, {"FAKE_ARCH": "i686"}):
            with self.subTest(extra=extra):
                (self.bin_dir / "sha256sum").unlink(missing_ok=True)
                result = self._run_installer(self.expected_sha256, extra_env=extra)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("unsupported", result.stderr)
                self.assertFalse((self.root / "curl-called").exists())

    def test_network_errors_preserve_existing_binary(self):
        self.install_dir.mkdir()
        binary = self.install_dir / "herdr"
        binary.write_bytes(b"old-aira")
        for stage in ("manifest", "asset"):
            with self.subTest(stage=stage):
                (self.bin_dir / "sha256sum").unlink(missing_ok=True)
                result = self._run_installer(self.expected_sha256, extra_env={"FAKE_FAIL": stage})
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(binary.read_bytes(), b"old-aira")
                self.assertFalse(list(self.install_dir.glob(".herdr-install.*")))

    def test_upstream_or_wrong_release_assets_are_rejected(self):
        for changes in (
            {"distribution": "herdrdev/herdr"},
            {"assets": {"linux-x86_64": "https://github.com/herdrdev/herdr/releases/download/v0.9.5/herdr-linux-x86_64"}},
            {"assets": {"linux-x86_64": "https://github.com/three4c/herdr/releases/download/aira-v11/herdr-linux-x86_64"}},
            {"version": "0.9.5"},
            {"release": "12"},
            {"release": -12},
            {"version": "invalid-aira.12"},
        ):
            with self.subTest(changes=changes):
                (self.bin_dir / "sha256sum").unlink(missing_ok=True)
                (self.root / "curl-called").unlink(missing_ok=True)
                self.manifest_changes = changes
                result = self._run_installer(self.expected_sha256)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual((self.root / "curl-called").read_text().splitlines(), [MANIFEST_URL])


class ForkReleaseTests(unittest.TestCase):
    def test_license_inventory_matches_sources_and_rejects_stale_inputs(self):
        fork_licenses.check(ROOT)
        with mock.patch.object(fork_licenses, "inputs", return_value={}), self.assertRaisesRegex(ValueError, "stale"):
            fork_licenses.check(ROOT)

    def test_release_number_is_canonical_and_monotonic(self):
        self.assertEqual(fork_release.require_newer("aira-v12", ["v99.0.0", "preview-anything", "aira-v11"]), 12)
        for tag in ("v1", "aira-v0", "aira-v01", "aira-v1.2", "aira-v-1", f"aira-v{2**64}"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                fork_release.release_number(tag)
        for tag in ("aira-v9", "aira-v10"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                fork_release.require_newer(tag, ["aira-v10"])

    def test_packaging_produces_matching_checksums_and_fork_urls(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets, output = Path(tmp) / "input", Path(tmp) / "output"
            assets.mkdir()
            for target in fork_release.TARGETS:
                (assets / f"herdr-{target}").write_bytes(f"binary:{target}".encode())
            manifest = fork_release.prepare("aira-v12", assets, output, ROOT)
            self.assertEqual(manifest["distribution"], "three4c/herdr")
            self.assertTrue(manifest["version"].endswith("-aira.12"))
            for target in fork_release.TARGETS:
                self.assertEqual(manifest["sha256"][target], hashlib.sha256((output / f"herdr-{target}").read_bytes()).hexdigest())
                self.assertEqual(manifest["assets"][target], f"https://github.com/three4c/herdr/releases/download/aira-v12/herdr-{target}")
            for line in (output / "SHA256SUMS").read_text().splitlines():
                digest, name = line.split("  ")
                self.assertEqual(digest, hashlib.sha256((output / name).read_bytes()).hexdigest())
            self.assertEqual((output / "install.sh").read_bytes(), INSTALLER.read_bytes())
            for source, name in (("LICENSE", "LICENSE"), ("distribution/fork/FORK-NOTICE.txt", "FORK-NOTICE.txt"), ("distribution/fork/THIRD-PARTY-NOTICES.txt", "THIRD-PARTY-NOTICES.txt")):
                self.assertEqual((output / name).read_bytes(), (ROOT / source).read_bytes())
            (assets / "herdr-linux-aarch64").unlink()
            with self.assertRaisesRegex(ValueError, "missing or empty"):
                fork_release.prepare("aira-v13", assets, output, ROOT)


if __name__ == "__main__":
    unittest.main()
