# Aira distribution for three4c/herdr

Aira is the personal fork of Herdr in `three4c/herdr`. The installed command is
`herdr-aira`, alongside upstream `herdr`. The repository, Cargo package, build
output, and Release asset names retain `herdr`. Upstream release workflows,
manifests, package versions, settings, saved sessions, and public protocols are
unchanged by this distribution setup.

This setup makes installation and updates convenient across the owner's PCs.
The public GitHub Releases are a download host for that personal workflow;
anyone with the URL can also download the assets. No authentication is needed.

## Install after the first Aira release is published

**These URLs become usable only after the first successful Aira release.**
Before publication, GitHub returns HTTP 404. Do not substitute the upstream
installer: it installs upstream Herdr.

Run this single line in a POSIX-compatible shell (including bash or zsh):

```sh
curl -fsSL https://github.com/three4c/herdr/releases/latest/download/install.sh | sh && export PATH="$HOME/.local/bin:$PATH"
```

Then run:

```sh
herdr-aira --version
herdr-aira
```

The installer selects macOS or Linux and x86_64 or aarch64 automatically. Linux
binaries use musl. Android/Termux, Windows, and other architectures are rejected.
The destination is `~/.local/bin/herdr-aira`; Rust and build tools are unnecessary.
`curl`, `awk`, ordinary POSIX utilities, and one of `sha256sum`, `shasum`, or
`openssl` are required. macOS includes `shasum`.

Add `export PATH="$HOME/.local/bin:$PATH"` to your shell startup file if this
directory is not already in PATH. The command above activates it for the current
shell; the installer does not edit shell configuration. It warns when an existing
`herdr-aira` elsewhere in PATH would shadow the fork. To choose another destination:

```sh
curl -fsSL https://github.com/three4c/herdr/releases/latest/download/install.sh | HERDR_INSTALL_DIR="$HOME/bin" sh
```

Installation replaces only `herdr-aira` at the selected destination after
verifying SHA-256. It creates no `herdr` alias or symlink and leaves any upstream
`herdr` executable untouched. Staging and the final rename happen on the
destination filesystem. Settings and saved sessions use the existing Herdr
locations; both commands can attach to compatible shared sessions. Launching
Aira does not replace an already running upstream server.

Fork binaries built before Aira distribution was added still contain the
upstream updater. Migrate those installations once with this installer; only
then use `herdr-aira update` for subsequent Aira releases. The old `herdr`
executable is left in place; switch to the new command. Check that
`herdr-aira --version` contains `-aira.` before relying on fork self-update.

## Update

Run outside Herdr after detaching:

```sh
herdr-aira update
```

The installer and the application's foreground/background update checks both
fetch:

```text
https://github.com/three4c/herdr/releases/latest/download/fork-latest.json
```

The manifest identifies `three4c/herdr`, contains the base version, release
number, source commit, existing protocol/generation values, four pinned binary
URLs, and SHA-256 checksums. Both clients reject binaries from another repository
or another release. Application updates ignore equal or older release numbers.
There is no fallback to upstream metadata or binaries if a request fails.

`herdr-aira channel show` reports `aira`. All channel switching is disabled,
including `channel set stable` and `channel set preview`; it fails without
editing config.
An existing `update.channel = "preview"` setting is preserved and ignored for
binary update selection. Homebrew, mise, and Nix update paths are unsupported;
install Aira in a standalone directory first. Windows self-update is disabled.
Cross-platform automatic remote installation is disabled; install Aira on the
SSH host with the same installer or provide a matching `HERDR_REMOTE_BINARY`.
Same-platform remote binary copying installs the local fork executable as
`~/.local/bin/herdr-aira`, preserving upstream `herdr` on the SSH host too.
Connection discovery checks Aira and compatible existing `herdr` candidates,
so shared sessions and saved SSH machines remain reachable.

The existing update command retains its server restart/handoff prompts. It does
not automatically convert an already running server to the new binary. Review
those prompts when updating a machine with running sessions. Agent-detection
catalog updates remain independent from binary distribution.

## Version identity

For example, a base `Cargo.toml` version of `0.9.5` and release number `12` produce:

```text
CLI: herdr 0.9.5-aira.12
Tag: aira-v12
```

`herdr-aira --version` retains the `herdr` product prefix shown above; the
installed command remains `herdr-aira`.

Increment the release number globally, including when adopting a new upstream
base: `0.9.5-aira.12` → `0.9.6-aira.13`. Do not reset it on an upstream version
change. Updates compare this number, rather than generic SemVer ordering of the
suffix. `Cargo.toml` and `Cargo.lock` retain the upstream base version. Local
builds default to `-aira.0`, which is never publishable; every source build in
this fork uses the fork update source, even without CI environment variables.

## Publish a release

This procedure operates only on `three4c/herdr`. Do not run the upstream
`just release` or `just preview` recipes for Aira.

1. Review and merge the distribution implementation into the fork's `master`.
   Confirm GitHub Actions is enabled in the fork. The workflow uses the built-in
   `GITHUB_TOKEN`; only its publish job requests `contents: write`. No upstream
   secrets or website deployment credentials are needed. GitHub tag rules and
   organization policies must allow the owner to push `aira-v*` tags and the
   workflow to create Releases.
2. Fetch the fork and select the reviewed source on `origin/master`. Confirm
   `git remote get-url origin` points to `three4c/herdr`, the worktree is clean,
   and the intended changes are included. Run `just ci`,
   `just docs-contract-test`, and `just fork-distribution-test` with Bun 1.3.14
   and Python 3.11 or newer. No Windows SDK is required for these checks.
3. Choose a number greater than every published `aira-v<N>` release. For the
   first release use `1`. Create an annotated tag and push that specific tag:

   ```sh
   git fetch origin master --tags
   git tag -a aira-v1 origin/master -m "release: aira v1"
   git push origin refs/tags/aira-v1
   ```

   Subsequent releases use `aira-v2`, `aira-v3`, and so on. Tagging publishes
   the selected commit; no version-bump commit is needed. Inspect the selected
   source and get commit-message alignment before committing any source changes.
4. Monitor **Aira fork release** in the fork's Actions tab. It checks the original
   and rerun actors have fork admin permission, validates master ancestry, the
   annotated tag and increasing release number, and runs native CI. It builds
   and executes `--version` on native runners for all four targets. Python 3.13,
   Bun 1.3.14, Rust 1.96.1, and Zig 0.16.0 are pinned in the workflow.
5. After every build succeeds, it packages four binaries, `install.sh`,
   `fork-latest.json`, `LICENSE`, `FORK-NOTICE.txt`, `THIRD-PARTY-NOTICES.txt`,
   and `SHA256SUMS`. It uploads to a draft before publishing
   the complete release as Latest. Check the ten assets and run the installer
   in a temporary destination before announcing the release.

   ```sh
   stage="$(mktemp -d)"
   curl -fsSL https://github.com/three4c/herdr/releases/latest/download/install.sh | HERDR_INSTALL_DIR="$stage" sh
   "$stage/herdr-aira" --version
   ```

An upload failure may leave a draft; rerunning the failed workflow can replace
draft assets and complete publication. Already published release numbers are
rejected and never overwritten. Publication is serialized and rechecks the
sequence just before publishing, so an older queued build cannot become Latest.
Use a new number to ship a correction. Keep all published Aira releases and tags.

## Licenses and fork notices

Herdr and these fork modifications retain Apache License 2.0. The original
license is unchanged. `FORK-NOTICE.txt` describes the changes and identifies
the original project and this personal fork. Aira change comments mark modified
Rust source files, including the earlier tab-focused changes.

Each executable contains the complete Herdr license, fork notice, and preserved
third-party license texts. Display them offline with `herdr-aira --licenses`.
Installing or updating the raw binary preserves these texts as part of the
executable; separate license downloads are not required. The same texts are
also published as Release assets for direct access.

`license-inventory.json` records the covered Cargo package versions and hashes
of dependency inputs and notice sources. The inventory covers normal and build
dependencies for all four targets conservatively, plus vendored/native libraries
and runtime notices. Some covered components are not linked into every binary.
Missing package license texts are preserved from their exact upstream source
commit under `distribution/fork/licenses/`, with source URLs and SHA-256 values.

Dependency, native-source, toolchain, or build-workflow changes invalidate the
inventory and stop checks/publication. Review new license terms and attribution
texts, preserve any missing upstream texts with provenance in `licenses/`, and
run `just fork-licenses-generate` with dependencies fetched into Cargo's cache.
Generation uses locked offline Cargo metadata; it does not fetch license texts
or silently substitute generic license boilerplate. Keep applicable upstream
notices when changing the dependency set.

## Local verification before publishing

```sh
just fork-distribution-test
just ci
just docs-contract-test
actionlint .github/workflows/fork-release.yml
shellcheck distribution/fork/install.sh
```

The installer tests use temporary destinations and substitute network commands.
They cover all four platform combinations, checksum tools, checksum mismatch,
missing checksums, network failures, unsupported platforms, and foreign assets.
Rust tests cover fork sequence ordering and disabled upstream update sources.

For a smoke test of real binaries, build two successive identities, copy each
binary into temporary staging paths, and run:

```sh
HERDR_FORK_RELEASE=1 just build
# Copy target/release/herdr to a temporary path named old-herdr.
HERDR_FORK_RELEASE=2 just build
# Copy target/release/herdr to another temporary path named new-herdr.
just fork-update-smoke /temporary/path/old-herdr /temporary/path/new-herdr
```

The smoke test installs and updates only temporary copies, substitutes all HTTP
requests, isolates config/state/socket paths, and starts no servers. It verifies
a real binary update, repeated updates, downgrade rejection, integrity and
network failures, fork URL validation, preservation of upstream executables,
legacy preview settings, and disabled channel switching. Native execution of
the other three target binaries is
validated by GitHub Actions at publication time.
