# harnesspin — committed audit results

Each `*.json` here is one `verify()` output: `Matched`,
`DivergedPinned`, or `Unverified`. Pre-reg §8 fixture discipline —
manifests used in release gates are committed, raw harness logs are
not.

## demonstration-template-drift.json

The pre-reg's `DivergedPinned` row shape, computed from two manifests
in `../fixtures/`. The declared manifest advertises
`chat_template_sha256_hex = "0202…"`; the observed manifest ran
under `9999…`. Everything else is identical. Verdict:
`DivergedPinned { divergences: [ChatTemplate { declared, observed }] }`,
exit 1.

This is a *demonstration* using worked-example SHAs, not an audit of
a published paper. Real paper manifests go here when authors
disclose the harness-commit field — which almost no public paper
does today (see `docs/PREREGISTRATION.md` §5).
