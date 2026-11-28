# Pre-registration additions

## 2026-11-27T06:41:48Z mutate proposal:5ce815d650e15a6c37dd3e8f63ca6c7c8f0a52eb63b01b732a65fe26bd341df5

Hypothesis: over seeded single-field mutations (200 per pinned field, each to a different non-blank value), verify returns DivergedPinned with exactly one divergence naming the mutated field and the canonical SHA-256 differs from the reference in every case; across 200 no-op controls (key reorder, whitespace) verify returns Matched with an identical hash in every case; across 200 blanking mutations of the blank-checked fields verify returns Unverified in every case.

Metric: /mutation/overall/pin_flag_rate. Expected: equals 1.0, control false-flag rate equals 0.0.

Falsifier: any non-blank single-field mutation yields Matched, an unchanged hash, or a divergence naming a different field; or any no-op control yields a non-Matched verdict or a different hash.

## 2026-11-28T00:35:51Z pin-audit proposal:3af34e2109e276a83366d19f726c3b28a37fe7870bbb8cffc3c50df893af594b

Hypothesis: verify only rejects blank pins. Across 200 seeded manifest pairs per floating-reference class (harness.commit_sha, model.revision, chat_template_sha256_hex, task.config_sha256_hex, few_shot.examples_sha256_hex), where declared and observed carry the same non-blank floating value (main, latest, HEAD, unknown, non-hex strings), verify returns Matched in every case. The same pairs with a real 40-hex or 64-hex pin also return Matched at 1.0, so Matched cannot distinguish pinned from floating rows.

Metric: /pin_audit/overall/floating_matched_rate. Expected: equals 1.0; hex-format control matched rate equals 1.0.

Falsifier: any identical-floating-reference pair yields Unverified or DivergedPinned, i.e. floating_matched_rate below 1.0.

## 2026-11-28T00:53:01Z pass-k-baseline proposal:21d3bff7f8a5693d76f0ba79032248af3b51e6c43932f6f6a5d36fcc93bd7657

Hypothesis: with 200 synthetic tasks and n=8 trials per task at seed 7, a coin-flip responder (per-trial success 0.5) has estimated pass^k (C(c,k)/C(n,k), mean over tasks) below the 0.05 floor first at k=5; the always-fail responder is below the floor at k=1.

Metric: /pass_k_baseline/coinflip/k_below_floor. Expected: equals 5 (range 4..6).

Falsifier: coinflip k_below_floor outside 4..6, or the estimated pass^k curve not decreasing monotonically in k.

## 2026-11-28T01:24:59Z floor-sweep proposal:812610e44c614c8a92f7d675ac796cba41ddaa6c1e9a11d3baf7c1da5aa72ef1

Hypothesis: with 400 synthetic tasks, 16 trials per task, seed 7, vacuous-task share 0.38 and non-vacuous per-task agent success drawn uniformly from 0.2 to 1.0, the do-nothing responder's pass^k equals the vacuous share at every k, and the agent's headline pass^k first comes within 0.05 of that floor at k=15 (range 12..16). At vacuous share 0.06 the agent stays more than 0.05 above the floor until a larger k than at 0.38, or never within 16.

Metric: /floor_sweep/0/k_at_floor (arm 0 is the 0.38 share). Expected: equals 15 (range 12..16).

Falsifier: k_at_floor for the 0.38 arm outside 12..16, or the do-nothing pass^k differing from the vacuous share at any k, or the 0.06 arm reaching the margin at a smaller k than the 0.38 arm.

## 2026-11-28T02:55:02Z scaffold-spread proposal:4ad2e61ef8a8d9f5abee405ebde85d943a54c9f7426ef9fa920d512cb28161df

Hypothesis: on a seeded synthetic task set (200 tasks, 8 trials, 6 models with a 0.03 skill step, 10 scaffold versions, each version adding a global offset drawn from U(-0.05, 0.05) and a per-task perturbation drawn from U(-0.15, 0.15) to success probability), the pairwise ranking-flip fraction when each model is scored under a randomly drawn scaffold version (unpinned) exceeds 0.10, while under one pinned version for all models it stays below 0.05. Flip fraction is averaged over 200 seeded replicates; a tied score pair counts as half a flip.

Metric: /scaffold_spread/ranking_flip_fraction_unpinned. Expected: at least 0.10 (range 0.10..0.5), above the pinned fraction.

Falsifier: ranking_flip_fraction_unpinned below 0.05, or not larger than ranking_flip_fraction_pinned.
