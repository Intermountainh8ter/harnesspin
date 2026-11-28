# Pre-registration additions

## 2026-11-27T06:41:48Z mutate proposal:5ce815d650e15a6c37dd3e8f63ca6c7c8f0a52eb63b01b732a65fe26bd341df5

Hypothesis: over seeded single-field mutations (200 per pinned field, each to a different non-blank value), verify returns DivergedPinned with exactly one divergence naming the mutated field and the canonical SHA-256 differs from the reference in every case; across 200 no-op controls (key reorder, whitespace) verify returns Matched with an identical hash in every case; across 200 blanking mutations of the blank-checked fields verify returns Unverified in every case.

Metric: /mutation/overall/pin_flag_rate. Expected: equals 1.0, control false-flag rate equals 0.0.

Falsifier: any non-blank single-field mutation yields Matched, an unchanged hash, or a divergence naming a different field; or any no-op control yields a non-Matched verdict or a different hash.

## 2026-11-28T00:35:51Z pin-audit proposal:3af34e2109e276a83366d19f726c3b28a37fe7870bbb8cffc3c50df893af594b

Hypothesis: verify only rejects blank pins. Across 200 seeded manifest pairs per floating-reference class (harness.commit_sha, model.revision, chat_template_sha256_hex, task.config_sha256_hex, few_shot.examples_sha256_hex), where declared and observed carry the same non-blank floating value (main, latest, HEAD, unknown, non-hex strings), verify returns Matched in every case. The same pairs with a real 40-hex or 64-hex pin also return Matched at 1.0, so Matched cannot distinguish pinned from floating rows.

Metric: /pin_audit/overall/floating_matched_rate. Expected: equals 1.0; hex-format control matched rate equals 1.0.

Falsifier: any identical-floating-reference pair yields Unverified or DivergedPinned, i.e. floating_matched_rate below 1.0.
