# RouterOS response-field coverage

## Status

Implemented. The checked-in baseline covers 25 CHR catalog versions, 137 API
commands, 134 Rust models, and 3,016 version/command/model observations. The
full audit currently reports zero ignored response fields and zero typed decode
failures.

Run `just schema-audit` to verify the live responses against the baseline. Run
`just schema-audit-update` only after inspecting and modeling an intentional
RouterOS schema change.

## Goal

Make every supported RouterOS response field explicit in the versionless Rust
models, while allowing fields absent in particular RouterOS versions.

## Compatibility contract

- Supported RouterOS versions are the CHR catalog versions exercised by the
  `version-stress-test` scenario. The `config-stress` scenario supplies
  representative configured objects across the collection surface.
- A typed endpoint may omit a field in a particular RouterOS version. Such
  version-dependent fields use `Option<T>` (or an empty collection where that
  better represents the API).
- A typed endpoint must not return an unmodelled field in any supported
  version.
- Unsupported RouterOS commands remain endpoint-local failures, rather than
  schema failures.

## Implementation phases

### 1. Add a strict schema-audit path

Keep normal crawler deserialization permissive initially. Add an ignored
scenario test that runs the version-stress manifest and deserializes every raw
API row through `serde_ignored` immediately before it becomes its
`mikrotik-types` model.

Collect ignored attributes by RouterOS version, API command, and model type.
The test must fail with a deterministic, grouped report when any response
field was ignored. This establishes the actual field union without making
ordinary crawling brittle when MikroTik introduces an attribute.

### 2. Cover the collection surface

Refactor the endpoint collector only as much as necessary for the audit to
observe raw rows for every endpoint used to build `RouterOsSnapshot`.

Expand `config-stress` with representative objects so fields are actually
returned from firewall, routing, PPP, IPsec, interfaces, services, queues, and
the other collected sections. Commands absent from a RouterOS version are
reported as endpoint-local unsupported-command results, not missing model
fields.

### 3. Model reported fields

For each ignored field:

- Add it to the owning response struct using its RouterOS wire name.
- Use `Option<T>` if the field is absent in any supported version.
- Introduce a domain type for meaningful constrained syntax; otherwise select
  the smallest accurate primitive.
- Add a focused fixture or unit test and retain a version-stress regression
  assertion.

If a field is intentionally outside the snapshot scope, document that decision
and explicitly exclude the command from the audit. Do not silently tolerate
the omission.

### 4. Audit every supported CHR

Run the audit against every catalog image, including the v6 images separately
because their command tree differs. Fix scenario bootstrap compatibility and
add configuration objects where required to exercise meaningful response
fields.

Persist a machine-readable baseline of the observed field union so adding a
future RouterOS release produces an actionable schema diff.

### 5. Enforce strict API decoding

After the audit is clean, make live API decoding strict at `Client::print` by
rejecting any fields reported by the same `serde_ignored` instrumentation used
by the audit. This deliberately rejects a new, present field that lacks a
model, while version-specific *absent* fields remain optional.

Keep direct deserialization of the shared response structs permissive for
backward compatibility with historical snapshot files. Enforcing strictness at
the live API boundary avoids duplicating the response model solely to apply
`#[serde(deny_unknown_fields)]`.

Add a test proving that a synthetic unknown API field fails with command and
model context.

### 6. Make it ongoing release hygiene

Run the strict audit through the local CHR/version-stress workflow, rather
than ordinary PR CI, because QEMU execution and image downloads are costly.
Run it whenever the CHR catalog gains a release. Treat a new field as a
required model update and a decoding/type-shape failure as a compatibility
regression.

## Acceptance criteria

- Every supported-version run reports zero ignored response fields for every
  supported typed endpoint.
- Version-specific fields have appropriate optionality.
- Production API decoding rejects a synthetic unknown field.
- Persisted snapshots remain readable wherever compatibility is promised.
- A future RouterOS change reports the RouterOS version, command, field, and
  Rust model that need attention.
