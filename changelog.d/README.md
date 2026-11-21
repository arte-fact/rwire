# Changelog fragments

Each pull request adds **one file** here instead of editing `CHANGELOG.md`:
two open PRs never touch the same lines, so the changelog no longer causes
merge conflicts.

- Name: `<short-topic>.md` (for example `delegated-auth-hardening.md`).
- Content: the section exactly as it will appear in `CHANGELOG.md`, starting
  with a `### Added: …`, `### Changed: …` or `### Fixed: …` heading.
- At release time the fragments are appended under the release heading of
  `CHANGELOG.md` (in merge order) and deleted in the same commit.

`CHANGELOG.md` itself is only edited by the release commit.
