#!/usr/bin/env python3
"""Smoke-test two real fork binaries in a temporary directory, without a server."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from fork_release import TARGETS, source_number

MANIFEST_URL = "https://github.com/three4c/herdr/releases/latest/download/fork-latest.json"


def identity(binary: Path) -> tuple[str, str, int]:
    label = subprocess.check_output([str(binary), "--version"], text=True).strip()
    match = re.fullmatch(r"herdr ([0-9]+\.[0-9]+\.[0-9]+)-aira\.([0-9]+)", label)
    if not match:
        raise ValueError(f"unexpected fork identity: {label}")
    return label.removeprefix("herdr "), match[1], int(match[2])


def smoke(old: Path, new: Path) -> None:
    repo_root = Path(__file__).resolve().parents[1]
    old_identity, _, old_number = identity(old)
    new_identity, new_base, new_number = identity(new)
    licenses = subprocess.check_output([str(new), "--licenses"], text=True)
    assert licenses.startswith((repo_root / "LICENSE").read_text())
    assert (repo_root / "distribution/fork/FORK-NOTICE.txt").read_text() in licenses
    assert (repo_root / "distribution/fork/THIRD-PARTY-NOTICES.txt").read_text() in licenses
    assert 0 < old_number < new_number
    with tempfile.TemporaryDirectory(prefix="herdr-aira-update-") as tmp:
        root = Path(tmp)
        commands = root / "commands"
        commands.mkdir()
        curl = commands / "curl"
        curl.write_text(f"#!{sys.executable}\n" + '''import os, sys, shutil
from pathlib import Path
args = sys.argv[1:]
url = next(arg for arg in args if arg.startswith("https://"))
with open(os.environ["AIRA_SMOKE_REQUESTS"], "a") as log:
    log.write(url + "\\n")
if "-o" in args:
    if os.environ.get("AIRA_SMOKE_FAIL") == "1":
        sys.exit(22)
    if not url.startswith("https://github.com/three4c/herdr/releases/download/aira-v"):
        sys.exit(99)
    shutil.copyfile(os.environ["AIRA_SMOKE_PAYLOAD"], args[args.index("-o") + 1])
else:
    if os.environ.get("AIRA_SMOKE_FAIL") == "manifest":
        sys.exit(22)
    if url != "https://github.com/three4c/herdr/releases/latest/download/fork-latest.json":
        sys.exit(99)
    sys.stdout.write(Path(os.environ["AIRA_SMOKE_MANIFEST"]).read_text())
''')
        curl.chmod(0o755)
        binary = root / "herdr"
        shutil.copy2(old, binary)
        config = root / "config.toml"
        config.write_text('[update]\nchannel = "preview"\n')
        original_config = config.read_bytes()
        requests = root / "requests"
        manifest_path = root / "manifest.json"
        env = {key: value for key, value in os.environ.items() if not key.startswith("HERDR_")}
        env.update({
            "PATH": f"{commands}{os.pathsep}{os.environ['PATH']}",
            "HERDR_CONFIG_PATH": str(config),
            "XDG_CONFIG_HOME": str(root / "config"),
            "XDG_STATE_HOME": str(root / "state"),
            "HERDR_SOCKET_PATH": str(root / "absent.sock"),
            "HERDR_CLIENT_SOCKET_PATH": str(root / "absent-client.sock"),
            "AIRA_SMOKE_REQUESTS": str(requests),
            "AIRA_SMOKE_MANIFEST": str(manifest_path),
            "AIRA_SMOKE_PAYLOAD": str(new),
        })
        digest = hashlib.sha256(new.read_bytes()).hexdigest()
        manifest = {
            "distribution": "three4c/herdr", "release": new_number,
            "version": new_identity, "base_version": new_base,
            "commit": "a" * 40,
            "protocol": source_number(repo_root / "src/protocol/wire.rs", "PROTOCOL_VERSION"),
            "endpoint_generation": source_number(repo_root / "src/protocol/endpoint.rs", "ENDPOINT_PROTOCOL_GENERATION"),
            "notes": "Temporary update smoke test",
            "assets": {target: f"https://github.com/three4c/herdr/releases/download/aira-v{new_number}/herdr-{target}" for target in TARGETS},
            "sha256": {target: digest for target in TARGETS},
        }

        def run(*args: str, succeeds: bool = True):
            result = subprocess.run([str(binary), *args], env=env, input="", capture_output=True, text=True, timeout=30)
            assert (result.returncode == 0) == succeeds, result.stdout + result.stderr
            assert config.read_bytes() == original_config
            return result

        for channel in ("stable", "preview"):
            run("channel", "set", channel, succeeds=False)
        assert "aira" in run("channel", "show").stdout
        assert not requests.exists()

        manifest_path.write_text(json.dumps(manifest, indent=2))
        installer_env = {**env, "HERDR_INSTALL_DIR": str(root / "install")}
        installed = subprocess.run(["/bin/sh", str(repo_root / "distribution/fork/install.sh")], env=installer_env, capture_output=True, text=True, timeout=30)
        assert installed.returncode == 0, installed.stdout + installed.stderr
        assert identity(root / "install/herdr")[0] == new_identity

        bad = {**manifest, "sha256": {target: "0" * 64 for target in TARGETS}}
        manifest_path.write_text(json.dumps(bad))
        run("update", succeeds=False)
        assert identity(binary)[0] == old_identity
        manifest_path.write_text(json.dumps(manifest))
        for stage in ("manifest", "1"):
            env["AIRA_SMOKE_FAIL"] = stage
            run("update", succeeds=False)
            assert identity(binary)[0] == old_identity
        env.pop("AIRA_SMOKE_FAIL")

        bad = {**manifest, "assets": {target: "https://herdr.dev/latest.json" for target in TARGETS}}
        manifest_path.write_text(json.dumps(bad))
        requests.write_text("")
        run("update", succeeds=False)
        assert requests.read_text().splitlines() == [MANIFEST_URL]
        assert identity(binary)[0] == old_identity

        manifest_path.write_text(json.dumps(manifest))
        run("update")
        assert identity(binary)[0] == new_identity
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == digest
        requests.write_text("")
        run("update")
        assert requests.read_text().splitlines() == [MANIFEST_URL]
        manifest["release"] = old_number
        manifest["version"] = f"{new_base}-aira.{old_number}"
        manifest["assets"] = {target: f"https://github.com/three4c/herdr/releases/download/aira-v{old_number}/herdr-{target}" for target in TARGETS}
        manifest_path.write_text(json.dumps(manifest))
        requests.write_text("")
        run("update")
        assert requests.read_text().splitlines() == [MANIFEST_URL]
        assert identity(binary)[0] == new_identity
        assert not list(root.glob(".herdr-update-*.tmp"))
        print(f"PASS: installer and {old_identity} -> {new_identity}; repeated update and downgrade ignored; checksum, network, upstream URL and channel guards verified; no servers started")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--old", type=Path, required=True)
    parser.add_argument("--new", type=Path, required=True)
    args = parser.parse_args()
    smoke(args.old.resolve(), args.new.resolve())
