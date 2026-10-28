# Aira distribution for three4c/herdr-aira

Aira is the personal fork of Herdr in `three4c/herdr-aira`. The installed command is
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
curl -fsSL https://github.com/three4c/herdr-aira/releases/latest/download/install.sh | sh && export PATH="$HOME/.local/bin:$PATH"
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
curl -fsSL https://github.com/three4c/herdr-aira/releases/latest/download/install.sh | HERDR_INSTALL_DIR="$HOME/bin" sh
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
https://github.com/three4c/herdr-aira/releases/latest/download/fork-latest.json
```

The manifest identifies `three4c/herdr-aira`, contains the base version, Aira
version, source commit, existing protocol/generation values, four pinned binary
URLs, and SHA-256 checksums. Both clients reject binaries from another repository
or another release. Application updates compare Aira major/minor/patch
numerically and ignore equal or older versions.
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

Aira uses its own major/minor/patch version, independently of the upstream
Herdr version. For a base `Cargo.toml` version of `0.9.5` and Aira `0.1.0`:

```text
Aira version: 0.1.0
Herdr base: 0.9.5
CLI: herdr 0.9.5-aira.0.1.0
Tag: aira-v0.1.0
```

`herdr-aira --version` retains the Herdr base and product prefix for provenance;
the installed command remains `herdr-aira`. The update decision uses only the
Aira version: `0.1.9` < `0.1.10` < `0.2.0` < `1.0.0`, regardless of the base.

Use patch releases for fixes, minor releases for new features, and major
releases when deliberately changing compatibility. Upstream updates do not
force Aira to adopt the upstream version number. For example, a release based
on Herdr `0.9.6` can be Aira `0.1.1`. Do not reset Aira's version on a base update.
Only stable three-part versions are supported; prerelease/build suffixes and
leading zeroes are rejected. Each component must fit an unsigned 32-bit integer.

`Cargo.toml` and `Cargo.lock` retain the upstream base version. Local builds
default to Aira `0.0.0`, which is never publishable. Set `HERDR_FORK_VERSION` to
build a specific Aira identity. Every source build uses the fork update source.
The retired integer-release experiment (`aira-v1` / `aira-v2`) is not supported;
there were no installed users to migrate. Published releases begin at
`aira-v0.1.0`.

## Publish a release

Release only when the owner requests it. Merging a PR into `master` does not
publish a release. The project-local `aira-release` skill reviews the diff since
the greatest published Aira tag, selects a patch/minor/major increment, and starts
the existing **Aira fork release** workflow. Cargo retains the upstream version.

After this workflow change is merged, start a release from the Actions page on
`master` with `version`, or use the following example (replace both values):

```sh
gh workflow run fork-release.yml --repo three4c/herdr-aira --ref master \
  -f version=0.1.1 -f source_sha="<reviewed-full-master-sha>"
```

The optional `source_sha` must equal master at dispatch time, so changes merged
after the AI's review cannot silently enter the release. An empty value on the
Actions page selects master at dispatch time. The selected SHA remains fixed
throughout the build. Both the original and rerun actors must be fork admins.
Only the tag-creation and publication jobs request write permission. The built-in
`GITHUB_TOKEN` is sufficient when fork tag rules permit it; no PAT, upstream
secret, or website deployment credential is needed.

Actions performs these steps:

1. Validate the independent Aira version and ensure it exceeds all published
   Aira releases. Wait up to 30 minutes for the selected commit's master CI.
   Failed, cancelled, missing, or skipped required CI cannot qualify. Nix and
   distribution checks must also pass when present for that commit. Waiting
   runs in Actions and consumes no agent monitoring tokens.
2. Create an annotated `aira-v<version>` tag at the fixed SHA. An existing
   unpublished annotated tag at the same SHA can resume; a moved or lightweight
   tag is rejected. Tags created by `GITHUB_TOKEN` do not trigger another push
   workflow, so the existing build jobs continue in this same workflow run.
3. Run native checks and release performance smoke in disposable sessions. Use
   the previous Aira release binary as the performance baseline when available;
   the first release compares with upstream stable. No ordinary session is used.
4. Build and execute version/license checks on native runners for Linux x86_64,
   Linux aarch64, Intel macOS, and Apple Silicon macOS. Package the same ten
   binaries/metadata/license/checksum assets, upload to a draft, and publish as
   Latest after every build succeeds.
5. Download the public assets, verify all checksums, source identity and platform
   headers, and exercise an anonymous install/reinstall in a temporary directory
   on Linux x86_64. Check an equal-version update and, when a previous Aira release
   exists, a real update from that binary. Preserve config and an upstream `herdr`
   sentinel; start no servers. Store a verification report in the workflow's
   artifacts and summary. Other platforms execute their build identity checks;
   the public install smoke currently runs on Linux x86_64.

The run URL is the completion report. Continuous agent monitoring is unnecessary.
A verification failure after publication leaves the release public; inspect the
failure rather than deleting published assets or moving its tag.

Manual pushes of owner-created annotated `aira-v*` tags remain supported and use
that same build/publication/verification pipeline. They bypass only the manual
request's prior-master-CI waiting stage; native release checks still run. Do not
use the upstream `just release` or `just preview` recipes for Aira.

Published versions are never overwritten. Publication is serialized and rechecks
the version before publishing. Use a newer version to ship a correction.

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
Rust tests cover fork semantic version ordering and disabled upstream update sources.

For a smoke test of real binaries, build two successive identities, copy each
binary into temporary staging paths, and run:

```sh
HERDR_FORK_VERSION=0.0.9 just build
# Copy target/release/herdr to a temporary path named old-herdr.
HERDR_FORK_VERSION=0.1.0 just build
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
