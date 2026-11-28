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

/// Name of the pinned field a divergence reports.
pub fn divergence_field(d: &Divergence) -> &'static str {
    match d {
        Divergence::Harness { .. } => "harness",
        Divergence::Task { .. } => "task",
        Divergence::Model { .. } => "model",
        Divergence::ChatTemplate { .. } => "chat_template",
        Divergence::Sampling { .. } => "sampling",
        Divergence::FewShot { .. } => "few_shot",
        Divergence::Metrics { .. } => "metrics",
    }
}

/// The seven pinned fields, in `compare` order.
pub const PINNED_FIELDS: [&str; 7] = [
    "harness",
    "task",
    "model",
    "chat_template",
    "sampling",
    "few_shot",
    "metrics",
];

/// Seeded single-field mutation study (model-free).
pub mod pass_k;
pub mod floor_sweep;
pub mod scaffold_spread;

pub mod mutate {
    use super::*;
    use serde_json::{json, Value};

    /// splitmix64: small seeded generator, no new dependency.
    pub struct Rng(pub u64);
    impl Rng {
        pub fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        pub fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    fn hex(r: &mut Rng) -> String {
        (0..4).map(|_| format!("{:016x}", r.next())).collect()
    }

    /// Reference manifest, drawn from the seed.
    pub fn reference(r: &mut Rng) -> RunManifest {
        RunManifest {
            harness: Harness { name: "ref-harness".into(), commit_sha: hex(r)[..40].into() },
            task: TaskConfig { name: "ref-task".into(), config_sha256_hex: hex(r) },
            model: Model { id: "ref-org/ref-model".into(), revision: hex(r)[..12].into() },
            chat_template_sha256_hex: hex(r),
            sampling: Sampling { temperature: 0.0, top_p: 1.0, top_k: 0, max_tokens: 128, seed: r.below(1000) },
            few_shot: FewShot { n_shots: 5, examples_sha256_hex: hex(r) },
            metrics: vec!["acc".into(), "acc_norm".into()],
        }
    }

    /// Change one field to a different, non-blank value.
    pub fn mutate_field(m: &RunManifest, field: &str, r: &mut Rng) -> RunManifest {
        let mut x = m.clone();
        match field {
            "harness" => {
                if r.below(2) == 0 { x.harness.name = format!("h-{}", hex(r)[..8].to_string()); }
                else { x.harness.commit_sha = hex(r)[..40].into(); }
            }
            "task" => {
                if r.below(2) == 0 { x.task.name = format!("t-{}", &hex(r)[..8]); }
                else { x.task.config_sha256_hex = hex(r); }
            }
            "model" => {
                if r.below(2) == 0 { x.model.id = format!("o/m-{}", &hex(r)[..8]); }
                else { x.model.revision = hex(r)[..12].into(); }
            }
            "chat_template" => x.chat_template_sha256_hex = hex(r),
            "sampling" => match r.below(5) {
                0 => x.sampling.temperature += 0.25 * (1 + r.below(4)) as f64,
                1 => x.sampling.top_p -= 0.05 * (1 + r.below(4)) as f64,
                2 => x.sampling.top_k += 1 + r.below(100) as u32,
                3 => x.sampling.max_tokens += 1 + r.below(1000) as u32,
                _ => x.sampling.seed += 1 + r.below(1000),
            },
            "few_shot" => {
                if r.below(2) == 0 { x.few_shot.n_shots += 1 + r.below(10) as u32; }
                else { x.few_shot.examples_sha256_hex = hex(r); }
            }
            "metrics" => match r.below(3) {
                0 => x.metrics.push(format!("m-{}", &hex(r)[..6])),
                1 => x.metrics.reverse(),
                _ => x.metrics[0] = format!("m-{}", &hex(r)[..6]),
            },
            _ => unreachable!("unknown field"),
        }
        x
    }

    /// Blank one blank-checked sub-field of `field`.
    pub fn blank_field(m: &RunManifest, field: &str, r: &mut Rng) -> RunManifest {
        let mut x = m.clone();
        match field {
            "harness" => x.harness.commit_sha = if r.below(2) == 0 { "".into() } else { "  ".into() },
            "task" => x.task.config_sha256_hex = String::new(),
            "model" => if r.below(2) == 0 { x.model.id.clear() } else { x.model.revision.clear() },
            "chat_template" => x.chat_template_sha256_hex = String::new(),
            "few_shot" => x.few_shot.examples_sha256_hex = String::new(),
            "metrics" => x.metrics.clear(),
            _ => unreachable!("sampling is not blank-checked"),
        }
        x
    }

    fn emit_shuffled(v: &Value, r: &mut Rng, out: &mut String) {
        let ws = |r: &mut Rng, out: &mut String| {
            for _ in 0..r.below(3) { out.push(if r.below(2) == 0 { ' ' } else { '\n' }); }
        };
        match v {
            Value::Object(o) => {
                let mut keys: Vec<&String> = o.keys().collect();
                for i in (1..keys.len()).rev() { keys.swap(i, r.below(i as u64 + 1) as usize); }
                out.push('{');
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 { out.push(','); }
                    ws(r, out);
                    out.push_str(&serde_json::to_string(k).unwrap());
                    ws(r, out);
                    out.push(':');
                    ws(r, out);
                    emit_shuffled(&o[k.as_str()], r, out);
                }
                ws(r, out);
                out.push('}');
            }
            Value::Array(xs) => {
                out.push('[');
                for (i, x) in xs.iter().enumerate() {
                    if i > 0 { out.push(','); ws(r, out); }
                    emit_shuffled(x, r, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }

    /// No-op control: same manifest re-serialised with shuffled key order
    /// and random whitespace, parsed back.
    pub fn noop_control(m: &RunManifest, r: &mut Rng) -> RunManifest {
        let mut s = String::new();
        emit_shuffled(&serde_json::to_value(m).unwrap(), r, &mut s);
        serde_json::from_str(&s).expect("shuffled text parses")
    }

    /// Run the whole study; returns the `mutation` results object.
    pub fn run(seed: u64, per_field: usize, controls: usize) -> Value {
        let mut r = Rng(seed);
        let base = reference(&mut r);
        let base_hash = canonical_sha256_hex(&base);
        let mut per = serde_json::Map::new();
        let (mut tot, mut flagged) = (0usize, 0usize);
        for f in PINNED_FIELDS {
            let (mut pin, mut hash_changed, mut named_only) = (0, 0, 0);
            for _ in 0..per_field {
                let x = mutate_field(&base, f, &mut r);
                if let Verdict::DivergedPinned { divergences } = verify(&base, &x) {
                    pin += 1;
                    if divergences.len() == 1 && divergence_field(&divergences[0]) == f { named_only += 1; }
                }
                if canonical_sha256_hex(&x) != base_hash { hash_changed += 1; }
            }
            tot += per_field;
            flagged += pin;
            let n = per_field as f64;
            per.insert(f.into(), json!({
                "n": per_field, "pin_flag_rate": pin as f64 / n,
                "exact_single_field_rate": named_only as f64 / n,
                "hash_change_rate": hash_changed as f64 / n,
            }));
        }
        let (mut c_match, mut c_hash) = (0, 0);
        for _ in 0..controls {
            let x = noop_control(&base, &mut r);
            if verify(&base, &x) == Verdict::Matched { c_match += 1; }
            if canonical_sha256_hex(&x) == base_hash { c_hash += 1; }
        }
        let blankable: Vec<&str> = PINNED_FIELDS.iter().copied().filter(|f| *f != "sampling").collect();
        let mut unv = 0;
        for i in 0..controls {
            let f = blankable[i % blankable.len()];
            let x = blank_field(&base, f, &mut r);
            if matches!(verify(&base, &x), Verdict::Unverified { .. }) { unv += 1; }
        }
        let c = controls.max(1) as f64;
        json!({
            "reference_sha256": base_hash,
            "per_field": per,
            "overall": {
                "n": tot,
                "pin_flag_rate": flagged as f64 / tot.max(1) as f64,
            },
            "controls": {
                "n": controls,
                "matched_rate": c_match as f64 / c,
                "hash_identical_rate": c_hash as f64 / c,
                "false_flag_rate": 1.0 - c_match as f64 / c,
            },
            "blanking": { "n": controls, "unverified_rate": unv as f64 / c },
        })
    }
}

/// Floating-reference audit (model-free): does `verify` accept identical
/// non-hex pins as `Matched`?
pub mod pin_audit {
    use super::mutate::{reference, Rng};
    use super::*;
    use serde_json::{json, Value};

    pub const CLASSES: [&str; 5] = [
        "harness.commit_sha",
        "model.revision",
        "chat_template_sha256_hex",
        "task.config_sha256_hex",
        "few_shot.examples_sha256_hex",
    ];
    const FLOATING: [&str; 4] = ["main", "latest", "HEAD", "unknown"];

    fn floating_value(r: &mut Rng) -> String {
        let i = r.below(5) as usize;
        if i < FLOATING.len() {
            FLOATING[i].to_string()
        } else {
            format!("zz-{:08x}-not-hex", r.next() as u32)
        }
    }

    fn set(m: &mut RunManifest, class: &str, v: String) {
        match class {
            "harness.commit_sha" => m.harness.commit_sha = v,
            "model.revision" => m.model.revision = v,
            "chat_template_sha256_hex" => m.chat_template_sha256_hex = v,
            "task.config_sha256_hex" => m.task.config_sha256_hex = v,
            "few_shot.examples_sha256_hex" => m.few_shot.examples_sha256_hex = v,
            _ => unreachable!("unknown class"),
        }
    }

    fn is_hex_of_len(s: &str, n: usize) -> bool {
        s.len() == n && s.bytes().all(|b| b.is_ascii_hexdigit())
    }

    pub fn run(seed: u64, per_class: usize) -> Value {
        let mut r = Rng(seed);
        let mut per = serde_json::Map::new();
        let (mut tot, mut fm, mut hm) = (0usize, 0usize, 0usize);
        for class in CLASSES {
            let (mut f_match, mut h_match, mut h_valid) = (0usize, 0usize, 0usize);
            for _ in 0..per_class {
                let base = reference(&mut r);
                let mut a = base.clone();
                let v = floating_value(&mut r);
                set(&mut a, class, v);
                if verify(&a, &a.clone()) == Verdict::Matched {
                    f_match += 1;
                }
                // control: the reference carries a real hex pin
                let n = if class == "harness.commit_sha" { 40 } else if class == "model.revision" { 12 } else { 64 };
                let pin = match class {
                    "harness.commit_sha" => &base.harness.commit_sha,
                    "model.revision" => &base.model.revision,
                    "chat_template_sha256_hex" => &base.chat_template_sha256_hex,
                    "task.config_sha256_hex" => &base.task.config_sha256_hex,
                    _ => &base.few_shot.examples_sha256_hex,
                };
                if is_hex_of_len(pin, n) {
                    h_valid += 1;
                }
                if verify(&base, &base.clone()) == Verdict::Matched {
                    h_match += 1;
                }
            }
            tot += per_class;
            fm += f_match;
            hm += h_match;
            let n = per_class.max(1) as f64;
            per.insert(class.into(), json!({
                "n": per_class,
                "floating_matched_rate": f_match as f64 / n,
                "hex_control_matched_rate": h_match as f64 / n,
                "hex_control_valid_format_rate": h_valid as f64 / n,
            }));
        }
        let t = tot.max(1) as f64;
        json!({
            "per_class": per,
            "overall": {
                "n": tot,
                "floating_matched_rate": fm as f64 / t,
                "hex_control_matched_rate": hm as f64 / t,
            },
        })
    }
}

#[cfg(test)]
mod pin_audit_tests {
    use super::*;

    #[test]
    fn audit_is_deterministic_and_floating_pins_match() {
        let a = pin_audit::run(7, 10);
        assert_eq!(a, pin_audit::run(7, 10));
        assert_eq!(a["overall"]["n"], 50);
        assert_eq!(a["overall"]["floating_matched_rate"], 1.0);
        assert_eq!(a["overall"]["hex_control_matched_rate"], 1.0);
    }

    #[test]
    fn floating_reference_is_not_degenerate() {
        let mut r = pin_audit_rng();
        let mut m = super::mutate::reference(&mut r);
        m.harness.commit_sha = "main".into();
        assert!(is_degenerate(&m).is_none());
        assert_eq!(verify(&m, &m.clone()), Verdict::Matched);
    }

    fn pin_audit_rng() -> super::mutate::Rng {
        super::mutate::Rng(11)
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

#[cfg(test)]
mod mutate_tests {
    use super::mutate::*;
    use super::*;

    #[test]
    fn study_is_deterministic_and_clean() {
        let a = run(7, 20, 14);
        assert_eq!(a, run(7, 20, 14));
        assert_eq!(a["overall"]["pin_flag_rate"], 1.0);
        assert_eq!(a["controls"]["false_flag_rate"], 0.0);
        assert_eq!(a["blanking"]["unverified_rate"], 1.0);
    }

    #[test]
    fn noop_control_keeps_hash() {
        let mut r = Rng(3);
        let m = reference(&mut r);
        let x = noop_control(&m, &mut r);
        assert_eq!(canonical_sha256_hex(&m), canonical_sha256_hex(&x));
        assert_eq!(verify(&m, &x), Verdict::Matched);
    }

    #[test]
    fn mutation_changes_named_field() {
        let mut r = Rng(5);
        let m = reference(&mut r);
        for f in PINNED_FIELDS {
            let x = mutate_field(&m, f, &mut r);
            match verify(&m, &x) {
                Verdict::DivergedPinned { divergences } => {
                    assert_eq!(divergences.len(), 1);
                    assert_eq!(divergence_field(&divergences[0]), f);
                }
                v => panic!("{f}: {v:?}"),
            }
        }
    }
}

