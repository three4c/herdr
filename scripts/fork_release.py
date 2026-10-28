#!/usr/bin/env python3
"""Generate release assets without changing upstream release metadata."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tomllib
from pathlib import Path

REPOSITORY = "three4c/herdr"
TARGETS = ("linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-aarch64")


def version_parts(version: str) -> tuple[int, int, int]:
    if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version):
        raise ValueError("fork version must be canonical major.minor.patch")
    parts = tuple(int(part) for part in version.split("."))
    if any(part > 2**32 - 1 for part in parts):
        raise ValueError("fork version components must fit u32")
    return parts


def release_version(tag: str) -> str:
    if not tag.startswith("aira-v"):
        raise ValueError("tag must be aira-v<major.minor.patch>")
    version = tag.removeprefix("aira-v")
    if version_parts(version) == (0, 0, 0):
        raise ValueError("0.0.0 is reserved for local builds")
    return version


def require_newer(tag: str, published_tags: list[str]) -> str:
    version = release_version(tag)
    versions = [version_parts(release_version(other)) for other in published_tags if other.startswith("aira-v")]
    if versions and version_parts(version) <= max(versions):
        raise ValueError("fork version must exceed every published fork release")
    return version


def source_number(path: Path, constant: str) -> int:
    match = re.search(rf"pub const {constant}: u32 = ([0-9]+);", path.read_text(encoding="utf-8"))
    if not match:
        raise ValueError(f"cannot read {constant} from {path}")
    return int(match[1])


def prepare(tag: str, assets: Path, output: Path, root: Path = Path(".")) -> dict:
    version = release_version(tag)
    base = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if version_parts(base) == (0, 0, 0):
        raise ValueError("Cargo.toml must contain a stable base version")
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    checksums = {}
    output.mkdir(parents=True, exist_ok=True)
    for target in TARGETS:
        name = f"herdr-{target}"
        source = assets / name
        if not source.is_file() or source.stat().st_size == 0:
            raise ValueError(f"missing or empty release binary: {source}")
        checksums[target] = hashlib.sha256(source.read_bytes()).hexdigest()
        shutil.copyfile(source, output / name)
    manifest = {
        "distribution": REPOSITORY,
        "release": version,
        "version": f"{base}-aira.{version}",
        "base_version": base,
        "commit": commit,
        "protocol": source_number(root / "src/protocol/wire.rs", "PROTOCOL_VERSION"),
        "endpoint_generation": source_number(root / "src/protocol/endpoint.rs", "ENDPOINT_PROTOCOL_GENERATION"),
        "notes": f"Aira {version}, based on Herdr {base} ({commit[:12]}). https://github.com/{REPOSITORY}/releases/tag/{tag}",
        "assets": {target: f"https://github.com/{REPOSITORY}/releases/download/{tag}/herdr-{target}" for target in TARGETS},
        "sha256": checksums,
    }
    (output / "fork-latest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")
    shutil.copyfile(root / "distribution/fork/install.sh", output / "install.sh")
    names = [f"herdr-{target}" for target in TARGETS] + ["fork-latest.json", "install.sh"]
    for source, name in (("LICENSE", "LICENSE"), ("distribution/fork/FORK-NOTICE.txt", "FORK-NOTICE.txt"), ("distribution/fork/THIRD-PARTY-NOTICES.txt", "THIRD-PARTY-NOTICES.txt")):
        shutil.copyfile(root / source, output / name)
        names.append(name)
    (output / "SHA256SUMS").write_text("".join(
        f"{hashlib.sha256((output / name).read_bytes()).hexdigest()}  {name}\n" for name in names
    ), encoding="utf-8", newline="\n")
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check")
    check.add_argument("--tag", required=True)
    check.add_argument("--published", type=Path, required=True, help="JSON array of published release tags")
    pack = commands.add_parser("prepare")
    pack.add_argument("--tag", required=True)
    pack.add_argument("--assets", type=Path, required=True)
    pack.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "check":
        print(require_newer(args.tag, json.loads(args.published.read_text(encoding="utf-8"))))
    else:
        prepare(args.tag, args.assets, args.output)


if __name__ == "__main__":
    main()
