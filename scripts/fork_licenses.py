#!/usr/bin/env python3
"""Preserve dependency license texts for Aira's standalone binaries."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = Path("distribution/fork")
TEXT = DIRECTORY / "THIRD-PARTY-NOTICES.txt"
INVENTORY = DIRECTORY / "license-inventory.json"
TARGETS = ("x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl", "x86_64-apple-darwin", "aarch64-apple-darwin")
NOTICE_NAME = re.compile(r"^(licen[cs]e|notice|copying|copyright|authors)([._-]|$)", re.I)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def notice_text(text: str) -> str:
    """Retain license wording while normalizing trailing whitespace."""
    return "\n".join(line.rstrip() for line in text.splitlines()).strip()


def inputs(root: Path) -> dict[str, str]:
    paths = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates/ghostty-vt/Cargo.toml",
             "LICENSE", "vendor/libghostty-vt.vendor.json", "vendor/libghostty-vt/LICENSE",
             "vendor/libghostty-vt/build.zig.zon", "vendor/portable-pty/Cargo.toml",
             "vendor/portable-pty/LICENSE.md", "scripts/fork_licenses.py"]
    paths.append(".github/workflows/fork-release.yml")
    paths += [path.relative_to(root).as_posix() for path in (root / DIRECTORY / "licenses").rglob("*") if path.is_file()]
    paths += [path.relative_to(root).as_posix() for path in (root / "vendor/libghostty-vt/pkg").rglob("build.zig.zon")]
    paths += ["vendor/libghostty-vt/pkg/simdutf/vendor/simdutf.h", "vendor/libghostty-vt/pkg/simdutf/vendor/simdutf.cpp"]
    return {name: sha256(root / name) for name in sorted(paths)}


def package_texts(package: dict, root: Path) -> list[tuple[str, str]]:
    directory = Path(package["manifest_path"]).parent
    if package["name"] in ("herdr", "ghostty-vt") and package["source"] is None:
        return [("LICENSE", (root / "LICENSE").read_text(encoding="utf-8"))]
    files = [path for path in directory.rglob("*") if path.is_file() and (
        NOTICE_NAME.match(path.name) or any(part.lower() in ("licenses", "licences") for part in path.relative_to(directory).parts[:-1])
    )]
    texts = [(path.relative_to(directory).as_posix(), path.read_text(encoding="utf-8")) for path in sorted(files)]
    if not texts:
        vcs = json.loads((directory / ".cargo_vcs_info.json").read_text(encoding="utf-8"))
        repository = package["repository"].rstrip("/").split("/")[-1]
        name = f"{repository}-{vcs['git']['sha1'][:6]}.txt"
        fallback = root / DIRECTORY / "licenses" / name
        if not fallback.is_file():
            raise ValueError(f"missing preserved upstream license for {package['name']} {package['version']}: {name}")
        texts = [(name, fallback.read_text(encoding="utf-8"))]
    return texts


def generate(root: Path = ROOT) -> None:
    packages = {}
    for target in TARGETS:
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--offline", "--format-version", "1", "--filter-platform", target], cwd=root, text=True
        ))
        all_packages = {package["id"]: package for package in metadata["packages"]}
        nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
        pending, visited = [metadata["resolve"]["root"]], set()
        while pending:
            key = pending.pop()
            if key in visited:
                continue
            visited.add(key)
            packages[key] = all_packages[key]
            for dependency in nodes[key]["deps"]:
                if any(kind["kind"] != "dev" for kind in dependency["dep_kinds"]):
                    pending.append(dependency["pkg"])

    sections: dict[str, list[str]] = {}
    components = []
    for package in sorted(packages.values(), key=lambda item: (item["name"], item["version"])):
        if not package["license"]:
            raise ValueError(f"review missing license expression for {package['name']}")
        texts = package_texts(package, root)
        label = f"{package['name']} {package['version']} ({package['license']})"
        components.append({"name": package["name"], "version": package["version"], "license": package["license"]})
        for filename, text in texts:
            sections.setdefault(notice_text(text), []).append(f"{label}: {filename}")

    for filename in ("vendor/libghostty-vt/LICENSE", "vendor/portable-pty/LICENSE.md"):
        sections.setdefault(notice_text((root / filename).read_text(encoding="utf-8")), []).append(filename)
    preserved = json.loads((root / DIRECTORY / "licenses/sources.json").read_text(encoding="utf-8"))
    for entry in preserved:
        path = root / DIRECTORY / "licenses" / entry["file"]
        if sha256(path) != entry["sha256"]:
            raise ValueError(f"license source digest mismatch: {path}")
        sections.setdefault(notice_text(path.read_text(encoding="utf-8")), []).append(entry["source"])

    header = (
        "Aira / Herdr third-party notices\n\n"
        "Generated by scripts/fork_licenses.py from Cargo.lock and preserved upstream texts.\n"
        "The inventory conservatively includes normal and build dependencies across all four\n"
        "supported targets. Some components are not linked into every target binary.\n"
        "License alternatives and accompanying notices are reproduced without modification.\n"
        "For components offering alternatives, MIT is selected where offered, otherwise\n"
        "Apache-2.0 or the supplied permissive license; required Unicode notices are retained.\n\n"
    )
    content = header + "\n\n".join("=" * 72 + "\n" + "\n".join(labels) + "\n\n" + text for text, labels in sections.items()) + "\n"
    (root / TEXT).write_text(content, encoding="utf-8", newline="\n")
    inventory = {"inputs": inputs(root), "notice_sha256": sha256(root / TEXT), "components": components}
    (root / INVENTORY).write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Preserved notices for {len(components)} Cargo packages and native dependencies")


def check(root: Path = ROOT) -> None:
    inventory = json.loads((root / INVENTORY).read_text(encoding="utf-8"))
    if inventory["inputs"] != inputs(root) or inventory["notice_sha256"] != sha256(root / TEXT):
        raise ValueError("Aira license inventory is stale; review dependency licenses and run `just fork-licenses-generate`")
    if not inventory["components"]:
        raise ValueError("Aira license inventory has no components")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("generate", "check"))
    args = parser.parse_args()
    generate() if args.command == "generate" else check()
