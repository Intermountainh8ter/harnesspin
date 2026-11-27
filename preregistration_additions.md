# Pre-registration additions

## 2026-11-27T06:41:48Z mutate proposal:5ce815d650e15a6c37dd3e8f63ca6c7c8f0a52eb63b01b732a65fe26bd341df5

Hypothesis: over seeded single-field mutations (200 per pinned field, each to a different non-blank value), verify returns DivergedPinned with exactly one divergence naming the mutated field and the canonical SHA-256 differs from the reference in every case; across 200 no-op controls (key reorder, whitespace) verify returns Matched with an identical hash in every case; across 200 blanking mutations of the blank-checked fields verify returns Unverified in every case.

Metric: /mutation/overall/pin_flag_rate. Expected: equals 1.0, control false-flag rate equals 0.0.

Falsifier: any non-blank single-field mutation yields Matched, an unchanged hash, or a divergence naming a different field; or any no-op control yields a non-Matched verdict or a different hash.
