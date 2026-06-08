# recourse

Durability and field-improvement loop for ousia-guard verdicts.

Every verdict emitted by `ousia-guard` leaves a durable, PII-free receipt.
The receipt stores a blake3 hash of the action (never the raw action), the
verdict, the fired rules, and provenance metadata.

## Receipt schema (`recourse.receipt.v1`)

```json
{
  "schema": "recourse.receipt.v1",
  "receipt_id": "01JXYZ...",
  "ts": "2026-06-08T09:30:00Z",
  "action_digest": "blake3:a1b2c3...",
  "verdict": "deny",
  "fired_rule": "dignity-floor",
  "tenet": "primacy_of_sentient_dignity",
  "axiom_chain": ["dignity-is-unconditional"],
  "ontology_version": "1.0.0",
  "guard_version": "0.1.0",
  "installation_id": "opaque-no-pii"
}
```

## Usage

```sh
# Emit a receipt from a verdict JSON
ousia-guard check --format json --explain < action.json | recourse receipt emit

# Emit with optional raw action storage (local only, never exported)
... | recourse receipt emit --store-raw

# Show a receipt by ID
recourse receipt show 01JXYZ...

# List receipts
recourse receipt ls
recourse receipt ls --since 30d --verdict deny --format json
```

## Storage

Receipts are appended to `$XDG_DATA_HOME/recourse/receipts/YYYY-MM.ndjson`
(or `~/.local/share/recourse/receipts/YYYY-MM.ndjson`).

## Install

```sh
cargo install --path recourse
```

## License

MIT — Joe Yen
