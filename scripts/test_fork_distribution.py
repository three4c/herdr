from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
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

    binary_name = "herdr-aira"

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
            "release": "0.1.10",
            "base_version": "0.9.5",
            "version": "0.9.5-aira.0.1.10",
            "assets": {target: f"https://github.com/three4c/herdr/releases/download/aira-v0.1.10/herdr-{target}" for target in fork_release.TARGETS},
            "sha256": {target: checksum for target in fork_release.TARGETS} if checksum else {},
            **self.manifest_changes,
        }
        path = self.root / "latest.json"
        path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
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
                self.assertEqual(marker.read_text(encoding="utf-8").splitlines(), [MANIFEST_URL, f"https://github.com/three4c/herdr/releases/download/aira-v0.1.10/herdr-{target}"])
                self.assertTrue(os.access(self.install_dir / self.binary_name, os.X_OK))

    def test_install_and_reinstall_preserve_upstream_executable(self):
        self.install_dir.mkdir()
        upstream_binary = self.install_dir / "herdr"
        upstream_binary.write_bytes(b"original-upstream-binary")
        for attempt in range(2):
            with self.subTest(attempt=attempt):
                (self.bin_dir / "sha256sum").unlink(missing_ok=True)
                result = self._run_installer(self.expected_sha256)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(upstream_binary.read_bytes(), b"original-upstream-binary")
                self.assertEqual((self.install_dir / self.binary_name).read_bytes(), self.payload.read_bytes())
                self.assertFalse((self.install_dir / self.binary_name).is_symlink())

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
        binary = self.install_dir / self.binary_name
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
            {"assets": {"linux-x86_64": "https://github.com/three4c/herdr/releases/download/aira-v0.1.9/herdr-linux-x86_64"}},
            {"version": "0.9.5"},
            {"release": 12},
            {"release": -12},
            {"release": "0.0.0"},
            {"release": "0.01.0"},
            {"release": "0.1.0-rc.1"},
            {"release": "4294967296.0.0"},
            {"base_version": "0.9.6"},
            {"version": "invalid-aira.0.1.10"},
        ):
            with self.subTest(changes=changes):
                (self.bin_dir / "sha256sum").unlink(missing_ok=True)
                (self.root / "curl-called").unlink(missing_ok=True)
                self.manifest_changes = changes
                result = self._run_installer(self.expected_sha256)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual((self.root / "curl-called").read_text(encoding="utf-8").splitlines(), [MANIFEST_URL])


class ForkReleaseTests(unittest.TestCase):
    def test_windows_installer_selects_fork_by_channel_without_version_parsing(self):
        powershell = shutil.which("pwsh") or shutil.which("powershell.exe")
        if not powershell:
            self.skipTest("PowerShell is required for the portable Windows channel contract")
        result = subprocess.run(
            [powershell, "-NoProfile", "-NonInteractive", "-File", str(ROOT / "scripts/test_fork_windows_channel.ps1")],
            capture_output=True, text=True, timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("PASS: Aira installer branch follows channel show", result.stdout)

    def test_protocol_source_is_read_as_utf8_on_every_host(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "wire.rs"
            source.write_bytes("// \u64cd\u4f5c\npub const PROTOCOL_VERSION: u32 = 22;\n".encode("utf-8"))
            # Model Windows' non-UTF-8 default even when this test runs on Unix.
            with mock.patch("io.text_encoding", side_effect=lambda encoding, stacklevel=2: encoding or "cp1252"):
                self.assertEqual(fork_release.source_number(source, "PROTOCOL_VERSION"), 22)

    def test_license_inventory_matches_sources_and_rejects_stale_inputs(self):
        fork_licenses.check(ROOT)
        with mock.patch.object(fork_licenses, "inputs", return_value={}), self.assertRaisesRegex(ValueError, "stale"):
            fork_licenses.check(ROOT)

    def test_release_version_is_canonical_and_semantically_monotonic(self):
        self.assertEqual(fork_release.require_newer("aira-v0.1.0", ["v99.0.0", "preview-anything"]), "0.1.0")
        for tag, published in (
            ("aira-v0.1.10", "aira-v0.1.9"),
            ("aira-v0.2.0", "aira-v0.1.99"),
            ("aira-v1.0.0", "aira-v0.99.99"),
        ):
            with self.subTest(tag=tag):
                self.assertEqual(fork_release.require_newer(tag, [published]), tag.removeprefix("aira-v"))
        for tag in ("v0.1.0", "aira-v1", "aira-v2", "aira-v0.0.0", "aira-v0.01.0", "aira-v0.1", "aira-v-1.0.0", "aira-v0.1.0-rc.1", "aira-v0.1.0+build", "aira-v4294967296.0.0"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                fork_release.release_version(tag)
        for tag in ("aira-v0.1.2", "aira-v0.1.10"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                fork_release.require_newer(tag, ["aira-v0.1.10"])
        # Retired integer releases must be withdrawn rather than silently ignored.
        with self.assertRaises(ValueError):
            fork_release.require_newer("aira-v0.1.0", ["aira-v2"])

    def test_packaging_produces_matching_checksums_and_fork_urls(self):
        with tempfile.TemporaryDirectory() as tmp:
            assets, output = Path(tmp) / "input", Path(tmp) / "output"
            assets.mkdir()
            for target in fork_release.TARGETS:
                (assets / f"herdr-{target}").write_bytes(f"binary:{target}".encode())
            manifest = fork_release.prepare("aira-v0.1.10", assets, output, ROOT)
            self.assertEqual(manifest["distribution"], "three4c/herdr")
            self.assertTrue(manifest["version"].endswith("-aira.0.1.10"))
            for target in fork_release.TARGETS:
                self.assertEqual(manifest["sha256"][target], hashlib.sha256((output / f"herdr-{target}").read_bytes()).hexdigest())
                self.assertEqual(manifest["assets"][target], f"https://github.com/three4c/herdr/releases/download/aira-v0.1.10/herdr-{target}")
            for line in (output / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
                digest, name = line.split("  ")
                self.assertEqual(digest, hashlib.sha256((output / name).read_bytes()).hexdigest())
            self.assertEqual((output / "install.sh").read_bytes(), INSTALLER.read_bytes())
            for source, name in (("LICENSE", "LICENSE"), ("distribution/fork/FORK-NOTICE.txt", "FORK-NOTICE.txt"), ("distribution/fork/THIRD-PARTY-NOTICES.txt", "THIRD-PARTY-NOTICES.txt")):
                self.assertEqual((output / name).read_bytes(), (ROOT / source).read_bytes())
            (assets / "herdr-linux-aarch64").unlink()
            with self.assertRaisesRegex(ValueError, "missing or empty"):
                fork_release.prepare("aira-v0.1.11", assets, output, ROOT)


if __name__ == "__main__":
    unittest.main()
