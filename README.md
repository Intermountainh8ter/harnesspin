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
- CLI: `canon`, `compare`, `verify`, `version`. Exit codes 0/1/2 for
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
