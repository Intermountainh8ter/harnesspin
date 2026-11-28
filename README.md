# harnesspin

**Pre-registered eval-harness run manifest.** Pin the harness commit,
task config, model, chat template, sampling, few-shot setup, and
metrics; emit a canonical SHA-256; compare a declared manifest to the
one the run actually emitted; apply a pre-reg comparability gate.
harnesspin does not run models or benchmarks — it is the manifest
layer between a harness and the rows the harness emits.

![status: initial scaffold](https://img.shields.io/badge/status-initial%20scaffold-lightgrey)
![language: Rust](https://img.shields.io/badge/language-Rust-blue)
![license: MIT](https://img.shields.io/badge/license-MIT-blue)

## Why

Every published leaderboard number depends on an eval harness, a task
config, a chat template, and a few-shot layout — but published numbers
rarely pin all of them, and silently non-comparable rows are routine.
lm-evaluation-harness changes its default chat template between
commits. HELM task configs resolve differently against different model
tokenisers. "k=5 few-shot MMLU" is a very different row depending on
which five exemplars the harness happened to pick that day. harnesspin
is the one-field hash and the one-verdict gate that lets a reviewer
tell a replicated row from a lookalike.

## What is in this scaffold

- `RunManifest { harness, task, model, chat_template_sha256_hex, sampling, few_shot, metrics }`
  — the seven pinned fields that make a row comparable to the paper's
  number (pre-reg §2).
- `canonical_sha256_hex(&manifest)` — SHA-256 over sorted-keys,
  whitespace-free JSON; the one-value row key (§6).
- `compare(declared, observed) -> Vec<Divergence>` — field-by-field
  diff in a fixed order; stable under input permutation.
- `verify(declared, observed) -> Verdict { Matched | DivergedPinned |
  Unverified }` — the pre-reg comparability gate (§4), with the §7
  degenerate-baseline guard applied first.
- `mutate` — model-free seeded single-field mutation study (`results/639-mutate.json`): across 1400 mutations (200 per pinned field) the pin flag rate is 1.0, the no-op control false-flag rate is 0.0, and the blanking unverified rate is 1.0.
- `pin-audit` — model-free seeded floating-reference audit (`results/755-pin-audit.json`): across 1000 identical-pin pairs (200 per class) carrying `main`, `latest`, `HEAD`, `unknown` or non-hex strings, the floating Matched rate is 1.0 and the hex-pin control Matched rate is 1.0, so Matched does not distinguish pinned from floating rows.
- `pass-k-baseline` — model-free seeded pass^k baseline (`results/759-pass-k-baseline.json`): with 200 tasks and 8 trials at seed 7, the coin-flip responder first falls below the 0.05 floor at k=4 (pass^4 0.049142857142857085), not the pre-registered k=5, though inside the 4..6 range; the always-fail responder is below the floor at k=1.
- `floor-sweep` — model-free seeded vacuous-task floor sweep (`results/767-floor-sweep.json`): with 400 tasks and 16 trials at seed 7, a do-nothing responder's pass^k equals the vacuous share at every k, and the agent's headline pass^k first comes within 0.05 of the 0.38-share floor at k=15; at share 0.06 it never comes within 0.05 by k=16.
- `scaffold-spread` — model-free seeded scaffold-version spread (`results/790-scaffold-spread.json`): with 200 tasks, 8 trials, 6 models and 10 scaffold versions at seed 7, the pairwise ranking-flip fraction is 0.10716666666666651 when each model is scored under a random scaffold version and 0.007166666666666667 under one pinned version.
- `shift-detect` — model-free seeded harness-version shift detection (`results/797-shift-detect.json`): with 400 replicates per fixture size at seed 7 and a paired one-trial two-sided z-test at 0.05, the smallest grid size reaching 0.8 power is 1600 tasks at shift 0.05 and 400 tasks at shift 0.1.
- `canary-power` — model-free seeded canary-release power (`results/810-canary-power.json`): with 8 trials per task, 400 replicates at seed 7 and a paired two-sided z-test at 0.05 on per-task pass^k estimates, the smallest per-trial drop reaching 0.8 power at 100 canary tasks is 0.06 at k=3 and 0.08 at k=5; the pre-registered expectation (at least 0.10 at k=3, k=5 no larger than k=3) was not met.
- CLI: `canon`, `compare`, `verify`, `mutate`, `pin-audit`, `pass-k-baseline`, `floor-sweep`, `scaffold-spread`, `shift-detect`, `canary-power`, `version`. Exit codes 0/1/2 for
  Matched/Diverged/Unverified so a CI job can gate on them.

## What is openly deferred (see §10)

- No live-harness adapter. A future `harnesspin observe` will spawn a
  harness and emit the observed manifest; the scaffold reads a JSON
  the caller produced.
- No cross-crate manifest import. A jailbreakreplay `ReplicationReport`
  carries most of the pinned fields; `from-replication` would lift
  those into a `RunManifest` without re-authoring.
- No manifest-authoring helper. A `harnesspin init` that scans a
  running harness is deferred.

## Related

Part of the [bench](https://github.com/Intermountainh8ter?tab=repositories)
suite of pre-registered audit crates: alongside contamcheck,
jailbreakreplay, toolguard, guardaudit, agentreplay, budgetguard,
shadowtrack, kernelwitness, provenancepack, attestchain.

## License

MIT.
