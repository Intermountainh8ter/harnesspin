# harnesspin pre-registration

Locked before the first harness-manifest certification is committed.
Append-only.

## 1. Scope

The population under certification is **leaderboard-style evaluation
rows**: a model, a benchmark, and a scalar score reported alongside
either a paper or a public leaderboard entry. For each row we certify
whether the manifest under which the number was produced matches a
declared manifest — the paper's or the leaderboard's. harnesspin does
not run the model. It is the manifest layer between a harness and the
rows the harness emits.

## 2. The seven pinned fields

A `RunManifest` pins exactly these fields; a divergence on any one is
a non-comparable row:

1. **Harness** — `(name, commit_sha)`. A different commit is a
   different harness, regardless of version string.
2. **Task config** — `(name, config_sha256_hex)`. The config SHA is
   the hash of the harness's resolved task configuration (max tokens,
   metric variant, filter chain). Two configs with the same name and
   different SHAs are different tasks.
3. **Model** — `(id, revision)`. A different revision is a different
   model. Hosted targets use the dated endpoint snapshot in `revision`.
4. **Chat template** — `chat_template_sha256_hex`. The template SHA
   at run time. Template drift between paper and replication is the
   single most common cause of silently non-comparable rows.
5. **Sampling** — `(temperature, top_p, top_k, max_tokens, seed)`.
   Any difference is a divergence; defaults don't count.
6. **Few-shot** — `(n_shots, examples_sha256_hex)`. The exemplar hash
   is pinned separately from `n_shots` because "same k, different
   exemplars" is a silent source of score drift (§5).
7. **Metrics** — ordered `Vec<String>` of the metric names the row
   claims to carry. Metrics requested FROM the harness, not what the
   row ended up with (§3).

## 3. Requested vs reported metrics

The pre-reg pins the metrics the manifest **requested** from the
harness. A harness may silently drop a metric it cannot compute
(missing per-example reference, unsupported option on a target).
harnesspin certifies the request, not the outcome; a reviewer reading
a row with fewer metrics than its manifest claims knows the harness
dropped some, which is a different question from "the paper picked
different metrics."

## 4. Verdicts

Exactly three verdicts, read from a `Verdict`:

- **Matched** — declared and observed manifests agree on every pinned
  field. The row is comparable to the paper's number.
- **DivergedPinned** — one or more pinned fields differ. The row is
  NOT comparable to the paper's number, regardless of how close the
  two scores read. The verdict carries the full divergence list.
- **Unverified** — a manifest failed §7's degenerate-baseline check
  (any pinned hash blank, any identity blank, metric list empty).
  The row cannot be certified either way.

## 5. Comparability rule

A row with verdict `DivergedPinned` is NOT a replication of the
paper's number. A reviewer may still find it informative — "we ran
the same model under different sampling and got the same score" is a
real finding — but it is not reported as "we reproduced the paper."
The pre-reg forbids downstream aggregation that treats a
DivergedPinned row as a Matched one.

## 6. Canonical hash

`canonical_sha256_hex(manifest)` is defined as the SHA-256 of the
canonical-JSON bytes of the manifest, where canonical-JSON is: UTF-8;
no whitespace; object keys sorted lexicographically; numbers in
`serde_json`'s `Number::to_string` form; strings escaped the way
`serde_json::to_string` escapes them. This matches the other bench
crates so the same recipe hashes every bench manifest the same way.

Two manifests with the same hash are the same manifest. Two manifests
with different hashes are not comparable rows.

## 7. Degenerate-baseline guard

A manifest with any of these blank is **Unverified**, not a passing
manifest with "defaults":

- `harness.commit_sha`
- `task.config_sha256_hex`
- `model.id`, `model.revision`
- `chat_template_sha256_hex`
- `few_shot.examples_sha256_hex`
- `metrics` (empty list)

This closes the "I didn't pin it so it must have been the default"
loophole; defaults are a position, not an absence.

## 8. Fixture discipline

A manifest committed to the repo as a release artifact (the paper's
declared manifest, or a certified row's observed manifest) is JSON
with the shape above. Raw harness logs, task configs the manifest's
SHA points to, and few-shot exemplar lists are external fixtures;
their hashes travel in the manifest, their contents do not need to be
in-repo. A reviewer that needs the content follows the hash to the
upstream source.

## 9. What this crate does NOT do

- Run models or benchmarks. harnesspin is the manifest layer; the
  harness is still responsible for emitting its own observed manifest.
- Judge whether two scores are "close enough." A DivergedPinned row
  is non-comparable regardless of the score; a Matched row is
  comparable regardless of the score. Score agreement is a downstream
  question for `releaseflag`, when that exists.
- Decide which harness or template is the right one. The pre-reg pins
  the harness and template the paper claimed; the certification is
  that the replication used the same ones.

## 10. Openly tracked gaps

- No live-harness adapter. The scaffold reads two manifest JSONs the
  caller produced; it does not spawn a harness, parse its logs, or
  recover a manifest from a running job. A future `harnesspin
  observe --harness lm-eval-harness --config task.yaml` would fit.
- No manifest-authoring helper. A future `harnesspin init` that scans
  a running harness and emits a candidate manifest would reduce the
  cost of adopting the crate.
- No cross-crate manifest import. A jailbreakreplay `ReplicationReport`
  carries most of the pinned fields; a future `harnesspin from-replication`
  that lifts those into a `RunManifest` would make the bench suite
  more cohesive.
