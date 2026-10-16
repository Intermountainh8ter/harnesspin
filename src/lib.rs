//! Pre-registered eval-harness run manifest.
//!
//! A `RunManifest` is the full record of what one evaluation run
//! actually was — which harness commit, which task config, which
//! model, which chat template, which sampling knobs, which metrics,
//! and which few-shot source. A `canonical_sha256_hex(&manifest)` is
//! the one-value row key: two manifests with the same hash are the
//! same run; manifests with different hashes are not comparable
//! numbers no matter how similar their scores read.
//!
//! The library's job is modest: pin the fields, hash them canonically
//! (sorted keys, UTF-8, whitespace-free), compare a declared manifest
//! to the manifest the run actually emitted, and apply the pre-reg
//! comparability gate. The harness itself still runs the model; this
//! crate certifies that the row is the row the paper claimed.
//!
//! See `docs/PREREGISTRATION.md` for the clauses this scaffold closes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A specific eval harness at a specific commit. The commit is the
/// pre-reg's `pinned` field; the name is informational.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Harness {
    pub name: String,
    pub commit_sha: String,
}

/// A task (benchmark + subset) at a specific config. The config SHA
/// pins task parameters that affect the number (max tokens, metric
/// variant, filter, …); two configs with the same name and different
/// SHAs are different tasks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaskConfig {
    pub name: String,
    pub config_sha256_hex: String,
}

/// The model being evaluated. HF repo id + revision is standard;
/// hosted targets use the endpoint id and the dated snapshot. Pre-reg
/// §2 point 2: a different revision is a different model, full stop.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Model {
    pub id: String,
    pub revision: String,
}

/// Pinned sampling knobs. Match the shape used across the bench
/// so a manifest can borrow a sampling block from a jailbreakreplay
/// row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Sampling {
    pub temperature: f64,
    pub top_p: f64,
    pub top_k: u32,
    pub max_tokens: u32,
    pub seed: u64,
}

/// Pinned few-shot setup. `n_shots` is the count; `examples_sha256_hex`
/// is the hash of the few-shot example list, so a reviewer can tell
/// apart "same k, different exemplars" (a frequent silent source of
/// score drift) from "same exemplars, same k".
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FewShot {
    pub n_shots: u32,
    pub examples_sha256_hex: String,
}

/// The full manifest. Serialised with sorted keys through
/// `canonical_json_bytes` so hash-equality is a syntactic property.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RunManifest {
    pub harness: Harness,
    pub task: TaskConfig,
    pub model: Model,
    /// Pre-reg §2 point 2: chat template SHA at run time. A mismatch
    /// is a template-drift divergence and the row is not comparable.
    pub chat_template_sha256_hex: String,
    pub sampling: Sampling,
    pub few_shot: FewShot,
    /// Which metrics were REQUESTED from the harness, in a fixed
    /// order. "Requested" because a harness may silently drop one it
    /// cannot compute, and the pre-reg wants the requested set pinned
    /// separately from what the row ended up carrying.
    pub metrics: Vec<String>,
}

/// Canonical JSON bytes: `serde_json::to_value` → walk → write a
/// sorted-keys, whitespace-free string → `.into_bytes()`. This matches
/// what the other bench crates do, so a reviewer can run the same hash
/// recipe against a manifest from any of them.
pub fn canonical_json_bytes<T: Serialize>(v: &T) -> Vec<u8> {
    let value = serde_json::to_value(v).expect("Serialize does not fail on our types");
    let mut out = String::new();
    write_canonical(&value, &mut out);
    out.into_bytes()
}

fn write_canonical(v: &serde_json::Value, out: &mut String) {
    use serde_json::Value::*;
    match v {
        Null => out.push_str("null"),
        Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Number(n) => out.push_str(&n.to_string()),
        String(s) => {
            // Delegate string escaping to serde_json so unicode and
            // control characters follow the standard escapes rather
            // than this crate's reinterpretation.
            out.push_str(&serde_json::to_string(s).expect("string serialises"));
        }
        Array(xs) => {
            out.push('[');
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(x, out);
            }
            out.push(']');
        }
        Object(obj) => {
            let mut keys: Vec<&std::string::String> = obj.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).expect("key serialises"));
                out.push(':');
                write_canonical(obj.get(k.as_str()).expect("key present"), out);
            }
            out.push('}');
        }
    }
}

/// Canonical SHA-256 of a manifest, lowercase hex.
pub fn canonical_sha256_hex<T: Serialize>(v: &T) -> String {
    let bytes = canonical_json_bytes(v);
    let d = Sha256::digest(&bytes);
    let mut s = String::with_capacity(d.len() * 2);
    for b in d {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// One kind of mismatch between a declared and an observed manifest.
/// Each variant names exactly one pre-reg field, so a reviewer can
/// read the divergence list and map it to the clause being violated.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Divergence {
    Harness { declared: Harness, observed: Harness },
    Task { declared: TaskConfig, observed: TaskConfig },
    Model { declared: Model, observed: Model },
    ChatTemplate { declared: String, observed: String },
    Sampling { declared: Sampling, observed: Sampling },
    FewShot { declared: FewShot, observed: FewShot },
    Metrics { declared: Vec<String>, observed: Vec<String> },
}

/// Compare two manifests. Returns the list of mismatches in a fixed
/// order (never reordered on input permutation, so a diff of two
/// `compare` runs is itself stable).
pub fn compare(declared: &RunManifest, observed: &RunManifest) -> Vec<Divergence> {
    let mut v = Vec::new();
    if declared.harness != observed.harness {
        v.push(Divergence::Harness {
            declared: declared.harness.clone(),
            observed: observed.harness.clone(),
        });
    }
    if declared.task != observed.task {
        v.push(Divergence::Task {
            declared: declared.task.clone(),
            observed: observed.task.clone(),
        });
    }
    if declared.model != observed.model {
        v.push(Divergence::Model {
            declared: declared.model.clone(),
            observed: observed.model.clone(),
        });
    }
    if declared.chat_template_sha256_hex != observed.chat_template_sha256_hex {
        v.push(Divergence::ChatTemplate {
            declared: declared.chat_template_sha256_hex.clone(),
            observed: observed.chat_template_sha256_hex.clone(),
        });
    }
    if declared.sampling != observed.sampling {
        v.push(Divergence::Sampling {
            declared: declared.sampling.clone(),
            observed: observed.sampling.clone(),
        });
    }
    if declared.few_shot != observed.few_shot {
        v.push(Divergence::FewShot {
            declared: declared.few_shot.clone(),
            observed: observed.few_shot.clone(),
        });
    }
    if declared.metrics != observed.metrics {
        v.push(Divergence::Metrics {
            declared: declared.metrics.clone(),
            observed: observed.metrics.clone(),
        });
    }
    v
}

/// Pre-reg §4 verdict. A `Matched` row is comparable to the paper's
/// number; any `DivergedPinned` row is NOT, regardless of how close
/// the two scores read. `Unverified` captures a manifest that is
/// structurally invalid for the gate (missing hashes, empty metric
/// list) — the pre-reg §7 degenerate-baseline guard.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Verdict {
    Matched,
    DivergedPinned { divergences: Vec<Divergence> },
    Unverified { reason: String },
}

/// Pre-reg §7 degenerate-baseline check: a manifest that forgot to
/// pin something is not a passing manifest with "defaults", it is
/// Unverified. Pre-reg does not accept blank fields on pinned
/// hashes, nor an empty metric list.
pub fn is_degenerate(m: &RunManifest) -> Option<String> {
    let mut reasons = Vec::new();
    if m.harness.commit_sha.trim().is_empty() {
        reasons.push("harness.commit_sha is empty");
    }
    if m.task.config_sha256_hex.trim().is_empty() {
        reasons.push("task.config_sha256_hex is empty");
    }
    if m.model.id.trim().is_empty() {
        reasons.push("model.id is empty");
    }
    if m.model.revision.trim().is_empty() {
        reasons.push("model.revision is empty");
    }
    if m.chat_template_sha256_hex.trim().is_empty() {
        reasons.push("chat_template_sha256_hex is empty");
    }
    if m.few_shot.examples_sha256_hex.trim().is_empty() {
        reasons.push("few_shot.examples_sha256_hex is empty");
    }
    if m.metrics.is_empty() {
        reasons.push("metrics is empty");
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join("; "))
    }
}

/// Apply the pre-reg gate: degenerate-baseline check first, then a
/// field-by-field compare. `Matched` only when the manifest is
/// structurally valid AND every pinned field agrees.
pub fn verify(declared: &RunManifest, observed: &RunManifest) -> Verdict {
    if let Some(r) = is_degenerate(declared) {
        return Verdict::Unverified {
            reason: format!("declared manifest: {r}"),
        };
    }
    if let Some(r) = is_degenerate(observed) {
        return Verdict::Unverified {
            reason: format!("observed manifest: {r}"),
        };
    }
    let divs = compare(declared, observed);
    if divs.is_empty() {
        Verdict::Matched
    } else {
        Verdict::DivergedPinned { divergences: divs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> Harness {
        Harness {
            name: "lm-evaluation-harness".into(),
            commit_sha: "deadbeef1234".into(),
        }
    }

    fn t() -> TaskConfig {
        TaskConfig {
            name: "mmlu".into(),
            config_sha256_hex: "a".repeat(64),
        }
    }

    fn mdl() -> Model {
        Model {
            id: "org/model".into(),
            revision: "r1".into(),
        }
    }

    fn s() -> Sampling {
        Sampling {
            temperature: 0.0,
            top_p: 1.0,
            top_k: 0,
            max_tokens: 128,
            seed: 1,
        }
    }

    fn fs() -> FewShot {
        FewShot {
            n_shots: 5,
            examples_sha256_hex: "b".repeat(64),
        }
    }

    fn manifest() -> RunManifest {
        RunManifest {
            harness: h(),
            task: t(),
            model: mdl(),
            chat_template_sha256_hex: "c".repeat(64),
            sampling: s(),
            few_shot: fs(),
            metrics: vec!["acc".into(), "acc_norm".into()],
        }
    }

    #[test]
    fn canonical_bytes_sort_object_keys() {
        // Build the same object two different ways and confirm the
        // canonical bytes are identical.
        let a = serde_json::json!({"b": 1, "a": 2});
        let b = serde_json::json!({"a": 2, "b": 1});
        assert_eq!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn canonical_hash_is_lowercase_hex_sha256() {
        let hash = canonical_sha256_hex(&manifest());
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn canonical_hash_changes_when_any_pinned_field_changes() {
        let base = manifest();
        let base_hash = canonical_sha256_hex(&base);
        let mut mutated = base.clone();
        mutated.harness.commit_sha = "cafef00d1234".into();
        assert_ne!(canonical_sha256_hex(&mutated), base_hash);
    }

    #[test]
    fn canonical_hash_is_stable_across_metric_renaming_only_when_identical() {
        let a = manifest();
        let mut b = manifest();
        b.metrics = vec!["acc_norm".into(), "acc".into()];
        // metrics is a Vec, not a Set: order matters. If the paper
        // listed metrics in a different order, that is a divergence.
        assert_ne!(canonical_sha256_hex(&a), canonical_sha256_hex(&b));
    }

    #[test]
    fn verify_matched_when_identical() {
        let m = manifest();
        assert_eq!(verify(&m, &m), Verdict::Matched);
    }

    #[test]
    fn verify_lists_every_mismatched_field() {
        let a = manifest();
        let mut b = manifest();
        b.harness.commit_sha = "x".repeat(12);
        b.model.revision = "r2".into();
        b.sampling.temperature = 0.7;
        let v = verify(&a, &b);
        let divs = match v {
            Verdict::DivergedPinned { divergences } => divergences,
            other => panic!("want DivergedPinned, got {other:?}"),
        };
        assert_eq!(divs.len(), 3);
        assert!(matches!(divs[0], Divergence::Harness { .. }));
        assert!(matches!(divs[1], Divergence::Model { .. }));
        assert!(matches!(divs[2], Divergence::Sampling { .. }));
    }

    #[test]
    fn verify_blocks_on_degenerate_declared_manifest() {
        let mut a = manifest();
        a.chat_template_sha256_hex = "".into();
        let b = manifest();
        match verify(&a, &b) {
            Verdict::Unverified { reason } => {
                assert!(reason.contains("declared manifest"));
                assert!(reason.contains("chat_template_sha256_hex"));
            }
            other => panic!("want Unverified, got {other:?}"),
        }
    }

    #[test]
    fn verify_blocks_on_degenerate_observed_manifest() {
        let a = manifest();
        let mut b = manifest();
        b.metrics.clear();
        match verify(&a, &b) {
            Verdict::Unverified { reason } => {
                assert!(reason.contains("observed manifest"));
                assert!(reason.contains("metrics"));
            }
            other => panic!("want Unverified, got {other:?}"),
        }
    }

    #[test]
    fn is_degenerate_names_every_missing_field_in_one_pass() {
        let empty = RunManifest {
            harness: Harness {
                name: "".into(),
                commit_sha: "".into(),
            },
            task: TaskConfig {
                name: "".into(),
                config_sha256_hex: "".into(),
            },
            model: Model {
                id: "".into(),
                revision: "".into(),
            },
            chat_template_sha256_hex: "".into(),
            sampling: s(),
            few_shot: FewShot {
                n_shots: 0,
                examples_sha256_hex: "".into(),
            },
            metrics: vec![],
        };
        let reason = is_degenerate(&empty).expect("empty is degenerate");
        for frag in [
            "harness.commit_sha",
            "task.config_sha256_hex",
            "model.id",
            "model.revision",
            "chat_template_sha256_hex",
            "few_shot.examples_sha256_hex",
            "metrics",
        ] {
            assert!(reason.contains(frag), "missing fragment {frag} in {reason}");
        }
    }

    #[test]
    fn manifest_round_trips_json() {
        let m = manifest();
        let s = serde_json::to_string(&m).unwrap();
        let m2: RunManifest = serde_json::from_str(&s).unwrap();
        assert_eq!(m, m2);
    }

    #[test]
    fn compare_detects_template_drift_specifically() {
        let a = manifest();
        let mut b = manifest();
        b.chat_template_sha256_hex = "d".repeat(64);
        let divs = compare(&a, &b);
        assert_eq!(divs.len(), 1);
        assert!(matches!(divs[0], Divergence::ChatTemplate { .. }));
    }

    #[test]
    fn compare_detects_fewshot_drift_at_identical_count() {
        let a = manifest();
        let mut b = manifest();
        // Same n_shots, different examples: this is the silent one.
        b.few_shot.examples_sha256_hex = "e".repeat(64);
        let divs = compare(&a, &b);
        assert_eq!(divs.len(), 1);
        assert!(matches!(divs[0], Divergence::FewShot { .. }));
    }

    #[test]
    fn verdict_round_trips_json_with_divergences() {
        let a = manifest();
        let mut b = manifest();
        b.sampling.seed = 2;
        let v = verify(&a, &b);
        let s = serde_json::to_string(&v).unwrap();
        let v2: Verdict = serde_json::from_str(&s).unwrap();
        assert_eq!(v, v2);
    }
}
