# Changelog

## v0.3.0 — 2026-06-12

recourse feedback: closes the improvement cycle. `feedback propose [--since]` gathers upheld+amended contests since the last version into a reviewer-gated changeset.toml + CHANGELOG-<version>.md (publishes nothing). `feedback ship <version> --confirm` runs `tribunal gate` as a pre-flight (gate failure blocks publish, non-zero exit) then invokes `herald-market` exactly once on pass; without --confirm it is a dry preview. Zero new contests exits cleanly ("nothing to ship") so no empty version is cut. `--upstream` records upstream intent without mutating the local marketplace; PR mechanics deferred. SIGPIPE-safe, rustc 1.85. tribunal + herald are recorded stubs in tests. 8 new feedback tests, full suite green.

## v0.2.0 — 2026-06-12

### Added
- `contest` subcommand: downstream users can dispute a verdict with `recourse contest <receipt-id> --expected allow --reason "..."`. Contests are captured as reviewer-gated proposals in `~/.local/share/recourse/contests/pending.ndjson` (schema `recourse.contest.v1`). No auto-action; human review required to uphold.
- `contest ls` — list open contest proposals
- `contest review <id> --uphold/--dismiss` — reviewer disposition

### Prior
- v0.1.0: initial release with `receipt emit`, `receipt show`, `receipt ls`, `pulse`, `feedback` subcommands
