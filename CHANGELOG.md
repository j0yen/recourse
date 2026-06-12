# Changelog

## v0.2.0 — 2026-06-12

### Added
- `contest` subcommand: downstream users can dispute a verdict with `recourse contest <receipt-id> --expected allow --reason "..."`. Contests are captured as reviewer-gated proposals in `~/.local/share/recourse/contests/pending.ndjson` (schema `recourse.contest.v1`). No auto-action; human review required to uphold.
- `contest ls` — list open contest proposals
- `contest review <id> --uphold/--dismiss` — reviewer disposition

### Prior
- v0.1.0: initial release with `receipt emit`, `receipt show`, `receipt ls`, `pulse`, `feedback` subcommands
