---
name: aira-release
description: Choose the next Aira fork version from released-to-master changes and start an explicitly requested three4c/herdr release. Use for Aira version selection or publication, not upstream Herdr releases.
---

# Aira release

This personal fork has an independent version, tags `aira-vX.Y.Z`, and the
installed command `herdr-aira`. Cargo retains the upstream Herdr base version.
Use only `three4c/herdr`; never run upstream `just release` or `just preview`.

## Decide the version

Verify `origin` and `gh auth status`, then fetch `origin/master` and tags without
changing the user's worktree. Record the full master SHA. Find the greatest
published Aira version through the fork's GitHub Releases API, excluding drafts.
Use the published tag as the comparison boundary; unfiltered `git describe`
can select an upstream tag. Inspect first-parent history, PR context, and the
actual diff through the recorded SHA, rather than deciding from commit subjects
alone. Include upstream changes that affect the fork's users.

- Fixes, security fixes, compatible dependency updates, and release tooling:
  increment patch.
- New user-visible features: increment minor and reset patch.
- Breaking changes at 1.x or later: increment major and reset minor/patch.
- During 0.x development, breaking changes increment minor. Publishing 1.0.0
  declares a stable compatibility policy and needs the owner's intent.
- Multiple changes use the largest applicable increment. Do not reset the Aira
  version when adopting a new upstream base. If there is no published release,
  start at 0.1.0. If there are no changes to ship, explain that instead of
  manufacturing a release; honor an explicit request to rebuild unchanged code.

State the selected version, reason, comparison tag, and exact source SHA.
Ask only when impact or intended scope is unclear. A request to recommend a
version does not authorize publication. An explicit release request authorizes
the following dispatch with the selected version; do not ask again merely
because Actions will create a tag and publish the approved fork.

## Start Actions

After an explicit release request, use the existing workflow:

```sh
gh workflow run fork-release.yml --repo three4c/herdr --ref master \
  -f version="<selected-version>" -f source_sha="<reviewed-full-master-sha>"
```

Actions verifies both actors are fork admins, rejects an unreviewed source
change, waits for matching master CI, creates or validates the annotated tag,
and runs the existing build/publication pipeline. It also runs performance and
public installation/update verification. No merge-triggered release occurs.
Do not bypass those checks by pushing a tag when dispatch rejects a request.

Fetch the new run URL once and report it. Do not continuously watch or poll
unless requested; CI waiting happens inside Actions. On a user completion
notification, check the exact run's final result, including public verification.
Report success only after verification succeeds. If publication succeeded but
verification failed, say that the release is already public.

For a failure, inspect the failed step and confirm the source before proposing
a fix. Do not repeatedly rerun tests until they pass. Never move/delete a
published tag or assets; a correction uses a newer version. An existing
unpublished tag at the same SHA can resume through the workflow; a different
SHA is rejected. Do not retry a version that has already been published merely
to repeat post-publication verification.
