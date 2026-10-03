//! t-129 / WRAV-001..WRAV-006: authorized review transitions (WRA-001).
//!
//! Every fixture is its own temporary Git repository, every signing key is
//! generated with the host's real `ssh-keygen` (detached `-Y sign` under the
//! contract namespaces), and the only route to a positive flow is the
//! library trust injection `ratmac::review::test::install` over the
//! fixture's own Engine root. Each check that has a positive control
//! establishes it first, then judges every refusal twin by the exact
//! contract bytes and by before/after byte snapshots of the Engine root.

use ratmac::cli;
use ratmac::review::test::{install, TestTrust, TrustGuard};
use ratmac_qa::support::{self, CaptureOptions, TempTree};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

/// The host verifier this suite pins, per the accepted design.
const KEYGEN: &str = "C:/WINDOWS/System32/OpenSSH/ssh-keygen.exe";
const REVIEW_NAMESPACE: &str = "ratmac-review-v1";
const APPROVE_NAMESPACE: &str = "ratmac-approve-v1";

// --- shared plumbing ---------------------------------------------------------

/// One in-process Engine invocation: its exit code, its stdout bytes, and
/// the refusal message it returned (empty on success).
struct Ran {
    code: i32,
    stdout: String,
    message: String,
}

/// Run one `rtm` command line in `root` through the library entry, so the
/// installed trust reader participates.
fn rtm(root: &Path, args: &[&str]) -> Ran {
    let mut full = vec!["rtm".to_owned()];
    full.extend(args.iter().map(|arg| (*arg).to_owned()));
    let mut stdout = Vec::new();
    match cli::run_from(full, root, &mut stdout) {
        Ok(code) => Ran {
            code,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            message: String::new(),
        },
        Err(error) => Ran {
            code: error.exit_code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            message: error.to_string(),
        },
    }
}

/// The project-scope spelling: the platform's canonicalized primary
/// checkout with forward slashes, `//?/` prefix included on Windows,
/// compared byte-for-byte.
fn project_scope(root: &Path) -> String {
    let canonical = fs::canonicalize(resolved_primary(root))
        .unwrap_or_else(|error| panic!("canonicalize the fixture primary checkout: {error}"));
    shown(&canonical)
}

/// The run id a successful `rtm start` reported.
fn started_run_id(ran: &Ran) -> String {
    ran.stdout
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("rtm: started run "))
        .and_then(|rest| rest.split(" at ").next())
        .unwrap_or_default()
        .to_owned()
}

/// The one spelling paths carry in this suite's expectations.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Length-frame one field: unsigned 64-bit big-endian length, then bytes.
fn frame(body: &mut Vec<u8>, field: &[u8]) {
    body.extend_from_slice(&(field.len() as u64).to_be_bytes());
    body.extend_from_slice(field);
}

/// The declared policy digest exactly as the contract frames it:
/// `ratmac-policy-v1`, one NUL byte, then the Approver label, the minimum in
/// canonical decimal, the role combination, and each reviewer sorted by
/// label as framed label then key.
fn policy_digest(
    approver: &str,
    minimum: u32,
    may_review: bool,
    reviewers: &[(String, String)],
) -> String {
    let mut body = b"ratmac-policy-v1".to_vec();
    body.push(0);
    frame(&mut body, approver.as_bytes());
    frame(&mut body, minimum.to_string().as_bytes());
    frame(
        &mut body,
        if may_review { "true" } else { "false" }.as_bytes(),
    );
    let mut sorted = reviewers.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));
    for (label, key) in sorted {
        frame(&mut body, label.as_bytes());
        frame(&mut body, key.as_bytes());
    }
    sha256_hex(&body)
}

/// The operating-system identity the Engine renders as the builder actor.
fn builder_actor() -> String {
    if cfg!(windows) {
        format!(
            "{}\\{}",
            std::env::var("USERDOMAIN").unwrap_or_default(),
            std::env::var("USERNAME").unwrap_or_default()
        )
    } else {
        std::env::var("USER").unwrap_or_else(|_| "builder".to_owned())
    }
}

fn ssh(dir: &Path, args: &[&str]) -> Output {
    assert!(
        Path::new(KEYGEN).is_file(),
        "the host verifier {KEYGEN} must exist"
    );
    support::command(KEYGEN, dir)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run the host ssh-keygen {args:?}: {error}"))
}

/// Fixture identities generated at test time with the host's real tool.
struct KeySet {
    dir: TempTree,
    publics: BTreeMap<String, String>,
}

impl KeySet {
    fn new(label: &str, names: &[&str]) -> Self {
        let dir = TempTree::new(&format!("{label}-keys")).expect("create fixture key tree");
        let mut publics = BTreeMap::new();
        for name in names {
            let path = dir.join(name);
            let generated = ssh(
                dir.path(),
                &[
                    "-t",
                    "ed25519",
                    "-f",
                    &path.to_string_lossy(),
                    "-N",
                    "",
                    "-C",
                    name,
                ],
            );
            assert!(
                generated.status.success(),
                "generate fixture key {name}: {}",
                support::text(&generated)
            );
            let public = fs::read_to_string(dir.join(format!("{name}.pub")))
                .unwrap_or_else(|error| panic!("read {name}.pub: {error}"));
            publics.insert((*name).to_owned(), public.trim().to_owned());
        }
        Self { dir, publics }
    }

    fn public(&self, name: &str) -> &str {
        self.publics
            .get(name)
            .unwrap_or_else(|| panic!("fixture key {name} exists"))
    }

    /// One detached signature over `payload`'s bytes under `namespace`. The
    /// host ssh-keygen never overwrites an existing `.sig` (it prompts, meets
    /// end of input, and leaves the first signature), so every signer signs
    /// a fresh per-signer copy of the same bytes and the copy's signature is
    /// returned - over the identical canonical intent.
    fn sign(&self, name: &str, namespace: &str, payload: &Path) -> PathBuf {
        let stem = payload
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let copy = payload.with_file_name(format!("{stem}.{name}.{namespace}"));
        let signature = PathBuf::from(format!("{}.sig", shown(&copy)));
        assert!(
            !copy.is_file() && !signature.is_file(),
            "each signer needs a fresh payload copy, both for its own              signature and because the host ssh-keygen never overwrites one:              {}",
            shown(&copy)
        );
        let bytes = fs::read(payload)
            .unwrap_or_else(|error| panic!("read the payload to copy for {name}: {error}"));
        fs::write(&copy, bytes)
            .unwrap_or_else(|error| panic!("write the per-signer copy {}: {error}", shown(&copy)));
        let key = self.dir.join(name);
        let signed = ssh(
            self.dir.path(),
            &[
                "-Y",
                "sign",
                "-f",
                &key.to_string_lossy(),
                "-n",
                namespace,
                &copy.to_string_lossy(),
            ],
        );
        assert!(
            signed.status.success(),
            "sign with {name} under {namespace}: {}",
            support::text(&signed)
        );
        assert!(
            signature.is_file(),
            "the detached signature {} exists",
            shown(&signature)
        );
        signature
    }
}

/// The primary checkout Git itself reports for `root`: the same source the
/// t-119 one-root rule resolves.
fn resolved_primary(root: &Path) -> PathBuf {
    let output = support::git(root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .expect("list the fixture worktrees");
    assert!(
        output.status.success(),
        "the fixture repository lists its worktrees"
    );
    let text = String::from_utf8_lossy(&output.stdout);
    let primary = text
        .split("\n\n")
        .find(|record| !record.lines().any(|line| line == "bare"))
        .and_then(|record| {
            record
                .lines()
                .find_map(|line| line.strip_prefix("worktree "))
        })
        .unwrap_or_else(|| panic!("the fixture repository reports a primary worktree: {text}"));
    PathBuf::from(primary)
}

// --- one governed fixture ----------------------------------------------------

/// The ordinary fixture runbook body: intake -> build -> done with a blocked
/// route back from build, and a `work` root the review policy digests.
const ORDINARY_BODY: &str = "\
[roots]
issue = \"work\"

[states.intake]
prompt = \"Intake the request.\"

[states.build]
prompt = \"Build the change.\"

[states.done]
prompt = \"Ship the change.\"

[[transitions]]
from = \"intake\"
to = \"build\"

[[transitions]]
from = \"build\"
to = \"done\"

[[transitions]]
from = \"build\"
to = \"intake\"
blocked-route = true
";

/// One declared review policy plus the enrollment facts that should or
/// should not agree with it.
struct Declaration {
    reviewers: Vec<(String, String)>,
    approver: String,
    minimum_reviews: u32,
    approver_may_review: bool,
    builder_keys: Vec<String>,
    /// The digest the manifest records, when it is not the true one.
    enrolled_policy_digest: Option<String>,
    /// The digest the runbook declares, when it is not the recomputed one.
    declared_policy_digest: Option<String>,
    /// The verifier digest the runbook declares, when substituted.
    declared_verifier_digest: Option<String>,
    /// The signers digest the runbook declares, when substituted.
    declared_signers_digest: Option<String>,
    /// The signers digest the enrollment binds, when substituted.
    manifest_signers_digest: Option<String>,
    /// The verifier digest the trust reader binds, when substituted.
    verifier_sha256: Option<String>,
    /// Install the trust reader at all; `false` leaves the production one.
    trust: bool,
    /// Write the manifest file at its addressed path.
    manifest_on_disk: bool,
}

impl Declaration {
    /// The independent default for a fixture with the full a/b/lead bench:
    /// reviewers a, b, and lead, approver lead, minimum two, the Approver
    /// never reviewing.
    fn independent(keys: &KeySet) -> Self {
        Self {
            reviewers: ["a", "b", "lead"]
                .iter()
                .map(|label| ((*label).to_owned(), keys.public(label).to_owned()))
                .collect(),
            approver: "lead".to_owned(),
            minimum_reviews: 2,
            approver_may_review: false,
            builder_keys: Vec::new(),
            enrolled_policy_digest: None,
            declared_policy_digest: None,
            declared_verifier_digest: None,
            declared_signers_digest: None,
            manifest_signers_digest: None,
            verifier_sha256: None,
            trust: true,
            manifest_on_disk: true,
        }
    }

    /// The same independent defaults with the reviewer rows given
    /// explicitly, for a fixture whose bench is not the full a/b/lead set.
    fn with_reviewers(reviewers: Vec<(String, String)>) -> Self {
        Self {
            reviewers,
            approver: "lead".to_owned(),
            minimum_reviews: 2,
            approver_may_review: false,
            builder_keys: Vec::new(),
            enrolled_policy_digest: None,
            declared_policy_digest: None,
            declared_verifier_digest: None,
            declared_signers_digest: None,
            manifest_signers_digest: None,
            verifier_sha256: None,
            trust: true,
            manifest_on_disk: true,
        }
    }
}

/// A temporary Git repository carrying a governed (or plain) runbook, its
/// fixture identities, and the installed trust reader for its Engine root.
struct Fixture {
    repo: TempRepo,
    keys: KeySet,
    declaration: Declaration,
    runbook_bytes: Vec<u8>,
    manifest_text: String,
    _trust: Option<TrustGuard>,
}

impl Fixture {
    /// Build one fixture whose runbook is `body` plus the declared review
    /// table bound to a freshly generated enrollment.
    fn governed(label: &str, body: &str, keys: KeySet, declaration: Declaration) -> Self {
        let repo = TempRepo::new(label);
        repo.write("work/content.txt", "Contributor bytes under review.\n");
        repo.write(
            "work/blocker.txt",
            "Opaque blocker; its content is not a gate.\n",
        );
        let verifier_digest = sha256_hex(&fs::read(KEYGEN).expect("read the pinned verifier"));
        let mut signers = String::new();
        for (label, public) in &declaration.reviewers {
            let parts: Vec<&str> = public.split_whitespace().collect();
            assert!(
                parts.len() >= 2 && parts[0] == "ssh-ed25519",
                "fixture public key for {label} is one OpenSSH line"
            );
            signers.push_str(&format!("{label} {} {}\n", parts[0], parts[1]));
        }
        keys.dir.write("allowed_signers", &signers);
        let signers_path = keys.dir.join("allowed_signers");
        let signers_digest = sha256_hex(signers.as_bytes());
        let true_digest = policy_digest(
            &declaration.approver,
            declaration.minimum_reviews,
            declaration.approver_may_review,
            &declaration.reviewers,
        );
        let scope = project_scope(repo.root());
        let mut manifest = String::new();
        manifest.push_str(&format!("project-scope = \"{scope}\"\n"));
        manifest.push_str(&format!(
            "policy-digest = \"{}\"\n",
            declaration
                .enrolled_policy_digest
                .clone()
                .unwrap_or_else(|| true_digest.clone())
        ));
        manifest.push_str(&format!("approver = \"{}\"\n", declaration.approver));
        manifest.push_str(&format!(
            "minimum-reviews = {}\n",
            declaration.minimum_reviews
        ));
        manifest.push_str(&format!(
            "approver-may-review = {}\n",
            declaration.approver_may_review
        ));
        manifest.push_str(&format!("verifier-path = \"{KEYGEN}\"\n"));
        manifest.push_str(&format!("verifier-digest = \"{verifier_digest}\"\n"));
        manifest.push_str(&format!("signers-path = \"{}\"\n", shown(&signers_path)));
        manifest.push_str(&format!(
            "signers-digest = \"{}\"\n",
            declaration
                .manifest_signers_digest
                .clone()
                .unwrap_or_else(|| signers_digest.clone())
        ));
        manifest.push_str("[builder]\n");
        manifest.push_str(&format!(
            "actor = \"{}\"\n",
            builder_actor().replace('\\', "\\\\")
        ));
        let enrolled: Vec<String> = declaration
            .builder_keys
            .iter()
            .map(|key| format!("\"{key}\""))
            .collect();
        manifest.push_str(&format!("keys = [{}]\n", enrolled.join(", ")));
        for (label, public) in &declaration.reviewers {
            manifest.push_str("\n[[reviewers]]\n");
            manifest.push_str(&format!("label = \"{label}\"\n"));
            manifest.push_str(&format!("key = \"{public}\"\n"));
        }
        let manifest_bytes = manifest.clone().into_bytes();
        let manifest_path = keys.dir.join("enrollment-manifest.toml");
        if declaration.manifest_on_disk {
            keys.dir.write("enrollment-manifest.toml", &manifest_bytes);
        }
        let mut runbook = String::from(body);
        runbook.push_str("\n[review]\n");
        runbook.push_str(&format!("manifest = \"{}\"\n", shown(&manifest_path)));
        runbook.push_str(&format!(
            "policy-digest = \"{}\"\n",
            declaration
                .declared_policy_digest
                .clone()
                .unwrap_or_else(|| true_digest.clone())
        ));
        runbook.push_str(&format!(
            "verifier-digest = \"{}\"\n",
            declaration
                .declared_verifier_digest
                .clone()
                .unwrap_or_else(|| verifier_digest.clone())
        ));
        runbook.push_str(&format!(
            "signers-digest = \"{}\"\n",
            declaration
                .declared_signers_digest
                .clone()
                .unwrap_or_else(|| signers_digest.clone())
        ));
        runbook.push_str(&format!(
            "minimum-reviews = {}\n",
            declaration.minimum_reviews
        ));
        runbook.push_str(&format!("approver = \"{}\"\n", declaration.approver));
        runbook.push_str(&format!(
            "approver-may-review = {}\n",
            declaration.approver_may_review
        ));
        runbook.push_str("work = [\"work\"]\n");
        for (label, public) in &declaration.reviewers {
            runbook.push_str("\n[[review.reviewers]]\n");
            runbook.push_str(&format!("label = \"{label}\"\n"));
            runbook.push_str(&format!("key = \"{public}\"\n"));
        }
        repo.write(".ratmac/ratmac.toml", &runbook);
        repo.commit_all("fixture baseline");
        let runbook_bytes = runbook.into_bytes();
        let trust = declaration.trust.then(|| {
            install(
                &repo.root().join(".ratmac"),
                TestTrust {
                    manifest_bytes: manifest_bytes.clone(),
                    signers_path: signers_path.clone(),
                    verifier_path: PathBuf::from(KEYGEN),
                    verifier_sha256: declaration
                        .verifier_sha256
                        .clone()
                        .unwrap_or_else(|| verifier_digest.clone()),
                },
            )
        });
        Self {
            repo,
            keys,
            declaration,
            runbook_bytes,
            manifest_text: manifest,
            _trust: trust,
        }
    }

    /// A plain ungoverned fixture: the same body with no review table.
    fn plain(label: &str, body: &str) -> Self {
        let keys = KeySet::new(label, &["a", "b", "lead"]);
        let repo = TempRepo::new(label);
        repo.write("work/content.txt", "Contributor bytes under review.\n");
        repo.write(
            "work/blocker.txt",
            "Opaque blocker; its content is not a gate.\n",
        );
        repo.write(".ratmac/ratmac.toml", body);
        repo.commit_all("fixture baseline");
        Self {
            repo,
            keys,
            declaration: Declaration {
                reviewers: Vec::new(),
                approver: String::new(),
                minimum_reviews: 1,
                approver_may_review: false,
                builder_keys: Vec::new(),
                enrolled_policy_digest: None,
                declared_policy_digest: None,
                declared_verifier_digest: None,
                declared_signers_digest: None,
                manifest_signers_digest: None,
                verifier_sha256: None,
                trust: false,
                manifest_on_disk: false,
            },
            runbook_bytes: body.as_bytes().to_vec(),
            manifest_text: String::new(),
            _trust: None,
        }
    }

    /// Swap the installed trust reader for a substituted one.
    fn reseat_trust(&mut self, trust: TestTrust) {
        self._trust = None;
        self._trust = Some(install(&self.repo.root().join(".ratmac"), trust));
    }

    fn root(&self) -> &Path {
        self.repo.root()
    }

    fn run_dir(&self, run: &str) -> PathBuf {
        self.root().join(".ratmac/runs").join(run)
    }

    fn engine_root_shown(&self) -> String {
        format!("{}/.ratmac", shown(&resolved_primary(self.root())))
    }

    fn record_state(&self, run: &str) -> String {
        let source = fs::read_to_string(self.run_dir(run).join("run.toml"))
            .unwrap_or_else(|error| panic!("read the Run Record of {run}: {error}"));
        let parsed: toml::Value = source.parse().expect("the Run Record is valid TOML");
        parsed["state"]
            .as_str()
            .unwrap_or_else(|| panic!("the Run Record of {run} carries a string state"))
            .to_owned()
    }

    /// Every byte under the Engine root, tagged by kind and path.
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        support::capture(&self.root().join(".ratmac"), CaptureOptions::default())
            .expect("capture the fixture Engine root")
            .tagged()
    }

    fn authorizations(&self, run: &str) -> BTreeMap<String, Vec<u8>> {
        let dir = self.run_dir(run).join("authorizations");
        let mut records = BTreeMap::new();
        let Ok(entries) = fs::read_dir(&dir) else {
            return records;
        };
        for entry in entries {
            let entry = entry.expect("read an authorization record entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).expect("read an authorization record");
            records.insert(name, bytes);
        }
        records
    }

    /// Admit one Run and return its minted id.
    fn admit(&self) -> String {
        let started = rtm(self.root(), &["start"]);
        assert_eq!(
            started.code, 0,
            "WRAV: admission of a governed Run must succeed: {}{}",
            started.stdout, started.message
        );
        let id = started_run_id(&started);
        assert!(
            id.starts_with("run-"),
            "WRAV: start reports the minted run id: {}",
            started.stdout
        );
        id
    }

    /// Render one canonical intent to a fresh file and prove the pinned
    /// report line - purpose word and all - against the bytes on disk.
    fn intent(&self, run: &str, purpose: &str, extra: &[&str], tag: &str) -> PathBuf {
        let out = self.keys.dir.join(format!("{tag}.bin"));
        let out_argument = out.to_string_lossy().into_owned();
        let mut args = vec!["intent", "--run", run];
        args.extend_from_slice(extra);
        args.push("--out");
        args.push(&out_argument);
        let rendered = rtm(self.root(), &args);
        assert_eq!(
            rendered.code, 0,
            "WRAV: render the {tag} {purpose} intent for {run}: {}{}",
            rendered.stdout, rendered.message
        );
        let bytes = fs::read(&out).unwrap_or_else(|error| panic!("read {tag} bytes: {error}"));
        assert_eq!(
            rendered.stdout,
            format!(
                "rtm: canonical {purpose} intent for run {run} written to {} \
                 (sha256 {})\n",
                shown(&out),
                sha256_hex(&bytes)
            ),
            "WRAV: the {tag} intent report is the pinned one line"
        );
        out
    }

    /// One `rtm intent` with a relative `--out`, run with the process cwd
    /// outside the Engine root: the report echoes the relative string in
    /// the displayed spelling, never canonicalized or made absolute, and
    /// the bytes land under that cwd.
    fn intent_relative(&self, run: &str, purpose: &str, tag: &str) {
        let name = format!("{tag}.bin");
        let previous = std::env::current_dir().expect("read the process cwd");
        // The cwd is process-wide; the caller's test holds the shared
        // environment lane for its whole body, so no parallel test observes
        // this change.
        std::env::set_current_dir(self.keys.dir.path()).unwrap_or_else(|error| {
            panic!("move the process cwd outside the Engine root: {error}")
        });
        let args = vec!["intent", "--run", run, "--out", name.as_str()];
        let rendered = rtm(self.root(), &args);
        std::env::set_current_dir(&previous)
            .unwrap_or_else(|error| panic!("restore the process cwd: {error}"));
        assert_eq!(
            rendered.code, 0,
            "WRAV: render the {tag} {purpose} intent relatively: {}{}",
            rendered.stdout, rendered.message
        );
        let bytes = fs::read(self.keys.dir.join(&name))
            .unwrap_or_else(|error| panic!("read the relative {tag} bytes: {error}"));
        assert_eq!(
            rendered.stdout,
            format!(
                "rtm: canonical {purpose} intent for run {run} written to {name} \
                 (sha256 {})\n",
                sha256_hex(&bytes)
            ),
            "WRAV: the relative intent report echoes the given string, not an absolute one"
        );
    }

    /// One fully signed authorization bundle: the named reviewers' detached
    /// signatures over the (optionally blocked) review intent, and the
    /// Approver's signature over the approval intent computed over exactly
    /// those review files.
    fn authorize(&self, run: &str, blocker: Option<&str>, reviewers: &[&str], tag: &str) -> Bundle {
        let mut hold = Vec::new();
        if let Some(blocker) = blocker {
            hold.push("--hold".to_owned());
            hold.push("--blocker".to_owned());
            hold.push(blocker.to_owned());
        }
        let hold_refs: Vec<&str> = hold.iter().map(|value| value.as_str()).collect();
        let purpose = if blocker.is_some() { "hold" } else { "step" };
        let review_intent = self.intent(run, purpose, &hold_refs, &format!("{tag}-review"));
        let mut reviews = Vec::new();
        for label in reviewers {
            let signature = self.keys.sign(label, REVIEW_NAMESPACE, &review_intent);
            reviews.push(((*label).to_owned(), signature));
        }
        let mut approval_args = hold.clone();
        for (label, signature) in &reviews {
            approval_args.push("--review".to_owned());
            approval_args.push(format!("{label}:{}", shown(signature)));
        }
        let approval_refs: Vec<&str> = approval_args.iter().map(|value| value.as_str()).collect();
        let approval_intent = self.intent(run, purpose, &approval_refs, &format!("{tag}-approval"));
        let approve = self.keys.sign(
            &self.declaration.approver,
            APPROVE_NAMESPACE,
            &approval_intent,
        );
        Bundle { reviews, approve }
    }

    fn step_with(&self, run: &str, bundle: &Bundle) -> Ran {
        let mut args = vec!["step".to_owned(), "--run".to_owned(), run.to_owned()];
        for (label, signature) in &bundle.reviews {
            args.push("--review".to_owned());
            args.push(format!("{label}:{}", shown(signature)));
        }
        args.push("--approve".to_owned());
        args.push(shown(&bundle.approve));
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        rtm(self.root(), &refs)
    }

    fn step_without_signatures(&self, run: &str) -> Ran {
        rtm(self.root(), &["step", "--run", run])
    }

    fn hold_with(&self, run: &str, blocker: &str, bundle: Option<&Bundle>) -> Ran {
        let mut args = vec![
            "hold".to_owned(),
            "--run".to_owned(),
            run.to_owned(),
            "--blocker".to_owned(),
            blocker.to_owned(),
            "--confirm".to_owned(),
            format!("hold {run}"),
        ];
        if let Some(bundle) = bundle {
            for (label, signature) in &bundle.reviews {
                args.push("--review".to_owned());
                args.push(format!("{label}:{}", shown(signature)));
            }
            args.push("--approve".to_owned());
            args.push(shown(&bundle.approve));
        }
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        rtm(self.root(), &refs)
    }

    fn publish_verdict(&self, run: &str, state: &str, input: &str) {
        fs::write(
            self.run_dir(run).join("verdict.toml"),
            format!("state = {state:?}\ninput = {input:?}\nrationale = \"fixture verdict\"\n"),
        )
        .expect("publish the live verdict");
    }

    /// One accepted authorization record and its exact shape.
    fn assert_authorization(
        &self,
        run: &str,
        sequence: u32,
        purpose: &str,
        from: &str,
        to: &str,
        bundle: &Bundle,
    ) {
        let records = self.authorizations(run);
        let name = format!("{:06}.toml", sequence);
        let bytes = records
            .get(&name)
            .unwrap_or_else(|| panic!("the authorization archive holds {name}"))
            .clone();
        let text = String::from_utf8(bytes).expect("the record is UTF-8");
        let value: toml::Value = text.parse().expect("the record is strict TOML");
        assert_eq!(
            value["sequence"].as_integer(),
            Some(i64::from(sequence)),
            "WRAV: {name} records its sequence"
        );
        assert_eq!(
            value["purpose"].as_str(),
            Some(purpose),
            "WRAV: {name} records its pinned purpose"
        );
        for field in [
            "intent_sha256",
            "policy_sha256",
            "enrollment_sha256",
            "approval_signature_sha256",
        ] {
            let digest = value[field].as_str().unwrap_or_default();
            assert_eq!(digest.len(), 64, "WRAV: {name} carries the 64-hex {field}");
            assert!(
                digest
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "WRAV: {name} carries lowercase hex {field}"
            );
        }
        let approval = sha256_hex(&fs::read(&bundle.approve).expect("read the approval signature"));
        assert_eq!(
            value["approval_signature_sha256"].as_str(),
            Some(approval.as_str()),
            "WRAV: {name} records the approval signature's digest"
        );
        let rows = value["reviews"]
            .as_array()
            .unwrap_or_else(|| panic!("WRAV: {name} rows its accepted reviews"));
        assert_eq!(rows.len(), bundle.reviews.len());
        for (row, (label, signature)) in rows.iter().zip(&bundle.reviews) {
            assert_eq!(row["label"].as_str(), Some(label.as_str()));
            let digest = sha256_hex(&fs::read(signature).expect("read the review signature"));
            assert_eq!(
                row["signature_sha256"].as_str(),
                Some(digest.as_str()),
                "WRAV: {name} rows {label}'s signature digest"
            );
        }
        assert_eq!(value["from_state"].as_str(), Some(from));
        assert_eq!(value["to_state"].as_str(), Some(to));
        assert_eq!(
            value["origin"].as_str(),
            Some("test-injected"),
            "WRAV: {name} never claims host enrollment"
        );
    }
}

/// One signed authorization bundle ready to submit.
struct Bundle {
    reviews: Vec<(String, PathBuf)>,
    approve: PathBuf,
}

/// The review files of `bundle` as `--review` argument pairs.
fn review_args(bundle: &Bundle) -> Vec<String> {
    bundle
        .reviews
        .iter()
        .map(|(label, signature)| format!("--review={label}:{}", shown(signature)))
        .collect()
}

// --- contract text builders ---------------------------------------------------

fn signatures_none(minimum: u32, run: &str) -> String {
    format!(
        "signatures: observed none; expected {minimum} distinct review signature(s) and one \
         approval signature over the canonical intent (rtm intent --run {run})"
    )
}

fn review_unverified(label: &str) -> String {
    format!(
        "review: observed a signature at label \"{label}\" that verifies under no enrolled \
         reviewer key; expected a ratmac-review-v1 signature by an enrolled reviewer"
    )
}

fn distinct_count(observed: u32, minimum: u32) -> String {
    format!("reviews: observed {observed} distinct accepted review(s); expected at least {minimum}")
}

fn approval_member(member: &str, approver: &str) -> String {
    format!(
        "approval: observed a signature by member \"{member}\"; expected the designated \
         approver \"{approver}\""
    )
}

fn approval_no_identity(approver: &str) -> String {
    format!(
        "approval: observed a signature by no enrolled identity; expected the designated \
         approver \"{approver}\""
    )
}

fn replay_detail(engine_root: &str, run: &str, consumed: u32) -> String {
    format!(
        "replay: observed the approval already consumed as sequence {consumed} under \
         {engine_root}/runs/{run}/authorizations/; expected a fresh authorization at \
         sequence {}",
        consumed + 1
    )
}

fn start_trust_origin(detail: &str) -> String {
    format!(
        "start: the runbook declares a review policy; admission requires its enrolled trust \
         origin: {detail}"
    )
}

fn pin_mismatch(command: &str, run: &str, observed: &str, expected: &str) -> String {
    format!(
        "{command}: runbook pin mismatch: .ratmac/ratmac.toml drifted since rtm start — \
         observed sha256={observed}; expected sha256={expected}; restore the pinned runbook \
         bytes or retire the run (rtm abandon --run {run} --confirm \"abandon {run}\"); \
         nothing was modified\nnext: rtm doctor"
    )
}

/// The exact two stdout lines of one ordinary governed step refusal.
fn step_refused(run: &str, detail: &str) -> String {
    format!("rtm: step refused; authorization: {detail}\nnext: rtm step --run {run}\n")
}

// --- the six checks -----------------------------------------------------------

/// WRAV-001: an allowed independent group review plus its Approver authorizes exactly one fresh normal transition and one blocked transition.
#[test]
fn wrav_001() {
    // The Engine runs in-process and its seams and cwd are process-wide, so
    // each test holds the shared lane for its whole body.
    let _lane = support::hook_lane();
    let keys = KeySet::new("wrav001", &["a", "b", "lead"]);
    let declaration = Declaration::independent(&keys);
    let fixture = Fixture::governed("wrav001", ORDINARY_BODY, keys, declaration);
    let run = fixture.admit();

    // The review intent is exposed without writing one byte under the root.
    let before_intent = fixture.snapshot();
    let review_intent = fixture.intent(&run, "step", &[], "w1-expose");
    assert!(review_intent.is_file());
    assert_eq!(
        fixture.snapshot(),
        before_intent,
        "WRAV-001: rtm intent writes nothing under the Engine root"
    );

    // A relative `--out`: the echo stays the relative string in the
    // displayed spelling, and the bytes land outside the Engine root.
    let before_relative = fixture.snapshot();
    fixture.intent_relative(&run, "step", "w1-relative");
    assert_eq!(
        fixture.snapshot(),
        before_relative,
        "WRAV-001: a relative --out still writes nothing under the Engine root"
    );

    // One fresh normal transition.
    let first = fixture.authorize(&run, None, &["a", "b"], "w1-step");
    let stepped = fixture.step_with(&run, &first);
    assert_eq!(
        stepped.code, 0,
        "WRAV-001: the authorized step succeeds: {}{}",
        stepped.stdout, stepped.message
    );
    assert_eq!(
        stepped.stdout,
        "Build the change.\nnext: rtm step --run ".to_owned() + &run + "\n",
        "WRAV-001: a successful governed step prints today's success"
    );
    assert_eq!(fixture.record_state(&run), "build");
    fixture.assert_authorization(&run, 1, "step", "intake", "build", &first);

    // One blocked transition through the same boundary.
    let held = fixture.authorize(&run, Some("work/blocker.txt"), &["a", "b"], "w1-hold");
    let paused = fixture.hold_with(&run, "work/blocker.txt", Some(&held));
    assert_eq!(
        paused.code, 0,
        "WRAV-001: the authorized hold succeeds: {}{}",
        paused.stdout, paused.message
    );
    assert_eq!(
        paused.stdout,
        format!(
            "rtm: run {run} paused against work/blocker.txt; routed build -> intake\n\
             The Run is not passed; a human resumes it when the blocker clears.\n"
        ),
        "WRAV-001: the authorized hold prints today's success"
    );
    fixture.assert_authorization(&run, 2, "hold", "build", "intake", &held);

    // Exactly the two authorizations this Run accepted exist, nothing more.
    let records = fixture.authorizations(&run);
    assert_eq!(
        records.keys().cloned().collect::<Vec<_>>(),
        vec!["000001.toml", "000002.toml"],
        "WRAV-001: one authorization per authorized transition",
    );
}

/// WRAV-002: builder-only review, an outsider, a non-approving member, a forged signer field, a missing signature, and an insufficient count all refuse with state and evidence byte-identical.
#[test]
fn wrav_002() {
    let _lane = support::hook_lane();
    let keys = KeySet::new("wrav002", &["a", "b", "lead", "builder", "outsider"]);
    let declaration = Declaration::independent(&keys);
    let fixture = Fixture::governed("wrav002", ORDINARY_BODY, keys, declaration);

    // Positive control: the same fixture's first Run authorizes and moves.
    let control = fixture.admit();
    let control_bundle = fixture.authorize(&control, None, &["a", "b"], "w2-control");
    let stepped = fixture.step_with(&control, &control_bundle);
    assert_eq!(
        stepped.code, 0,
        "WRAV-002: the positive control step succeeds: {}{}",
        stepped.stdout, stepped.message
    );

    // Every refusal twin lands on this fresh Run at intake.
    let run = fixture.admit();
    assert_eq!(fixture.record_state(&run), "intake");

    // Twin: a missing signature - no options at all.
    let before = fixture.snapshot();
    let none = fixture.step_without_signatures(&run);
    assert_eq!(
        none.code, 0,
        "WRAV-002: a refusal is a StepOutcome::Refused"
    );
    assert_eq!(
        none.stdout,
        step_refused(&run, &signatures_none(2, &run)),
        "WRAV-002: the missing-signature refusal is exact"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the missing-signature refusal writes nothing"
    );

    // Twin: a builder-only review - the builder's own key at both labels
    // over the true review intent, with an internally consistent approval.
    let review_intent = fixture.intent(&run, "step", &[], "w2-builder-intent");
    // One builder signature, deliberately submitted under both labels.
    let builder_sig = fixture
        .keys
        .sign("builder", REVIEW_NAMESPACE, &review_intent);
    let builder_approval = {
        let mut args = Vec::new();
        for label in ["a", "b"] {
            args.push("--review".to_owned());
            args.push(format!("{label}:{}", shown(&builder_sig)));
        }
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let approval_intent = fixture.intent(&run, "step", &refs, "w2-builder-approval");
        fixture
            .keys
            .sign("lead", APPROVE_NAMESPACE, &approval_intent)
    };
    let before = fixture.snapshot();
    let builder_ran = fixture.step_with(
        &run,
        &Bundle {
            reviews: vec![
                ("a".to_owned(), builder_sig.clone()),
                ("b".to_owned(), builder_sig),
            ],
            approve: builder_approval,
        },
    );
    assert_eq!(builder_ran.code, 0);
    assert_eq!(
        builder_ran.stdout,
        step_refused(&run, &review_unverified("a")),
        "WRAV-002: a builder-only review refuses at its first label"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the builder-only refusal writes nothing"
    );

    // Twin: an outsider reviewing under its own unenrolled label.
    let outsider_review = {
        let intent = fixture.intent(&run, "step", &[], "w2-outsider-intent");
        fixture.keys.sign("outsider", REVIEW_NAMESPACE, &intent)
    };
    let outsider_approval = {
        let args = [
            "--review".to_owned(),
            format!("outsider:{}", shown(&outsider_review)),
        ];
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let approval_intent = fixture.intent(&run, "step", &refs, "w2-outsider-approval");
        fixture
            .keys
            .sign("lead", APPROVE_NAMESPACE, &approval_intent)
    };
    let before = fixture.snapshot();
    let outsider_ran = fixture.step_with(
        &run,
        &Bundle {
            reviews: vec![("outsider".to_owned(), outsider_review)],
            approve: outsider_approval,
        },
    );
    assert_eq!(outsider_ran.code, 0);
    assert_eq!(
        outsider_ran.stdout,
        step_refused(&run, &review_unverified("outsider")),
        "WRAV-002: an outsider's own label refuses"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the outsider refusal writes nothing"
    );

    // Twin: a forged signer field - an enrolled label over an outsider's
    // signature, beside one honest review.
    let honest = fixture.authorize(&run, None, &["b"], "w2-honest-b");
    let forged = {
        let intent = fixture.intent(&run, "step", &[], "w2-forged-intent");
        fixture.keys.sign("outsider", REVIEW_NAMESPACE, &intent)
    };
    let forged_approval = {
        let args = [
            "--review".to_owned(),
            format!("a:{}", shown(&forged)),
            "--review".to_owned(),
            format!("b:{}", shown(&honest.reviews[0].1)),
        ];
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let approval_intent = fixture.intent(&run, "step", &refs, "w2-forged-approval");
        fixture
            .keys
            .sign("lead", APPROVE_NAMESPACE, &approval_intent)
    };
    let before = fixture.snapshot();
    let forged_ran = fixture.step_with(
        &run,
        &Bundle {
            reviews: vec![
                ("a".to_owned(), forged),
                ("b".to_owned(), honest.reviews[0].1.clone()),
            ],
            approve: forged_approval,
        },
    );
    assert_eq!(forged_ran.code, 0);
    assert_eq!(
        forged_ran.stdout,
        step_refused(&run, &review_unverified("a")),
        "WRAV-002: a forged signer field refuses at the claimed label"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the forged-field refusal writes nothing"
    );

    // Twin: a non-approving member signs the approval.
    let member_reviews = {
        let intent = fixture.intent(&run, "step", &[], "w2-member-intent");
        (
            fixture.keys.sign("a", REVIEW_NAMESPACE, &intent),
            fixture.keys.sign("b", REVIEW_NAMESPACE, &intent),
        )
    };
    let member_approval = {
        let args = [
            "--review".to_owned(),
            format!("a:{}", shown(&member_reviews.0)),
            "--review".to_owned(),
            format!("b:{}", shown(&member_reviews.1)),
        ];
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let approval_intent = fixture.intent(&run, "step", &refs, "w2-member-approval");
        fixture.keys.sign("b", APPROVE_NAMESPACE, &approval_intent)
    };
    let before = fixture.snapshot();
    let member_ran = fixture.step_with(
        &run,
        &Bundle {
            reviews: vec![
                ("a".to_owned(), member_reviews.0),
                ("b".to_owned(), member_reviews.1),
            ],
            approve: member_approval,
        },
    );
    assert_eq!(member_ran.code, 0);
    assert_eq!(
        member_ran.stdout,
        step_refused(&run, &approval_member("b", "lead")),
        "WRAV-002: a non-approving member cannot approve"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the member-approval refusal writes nothing"
    );

    // Twin: an insufficient count - one honest review of two required.
    let short = fixture.authorize(&run, None, &["a"], "w2-short");
    let before = fixture.snapshot();
    let short_ran = fixture.step_with(&run, &short);
    assert_eq!(short_ran.code, 0);
    assert_eq!(
        short_ran.stdout,
        step_refused(&run, &distinct_count(1, 2)),
        "WRAV-002: an insufficient distinct count refuses"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the insufficient-count refusal writes nothing"
    );

    // Every twin left the Run exactly where it was, and a genuinely signed
    // bundle still moves it - the refusals were the signatures' fault.
    assert_eq!(fixture.record_state(&run), "intake");
    let final_bundle = fixture.authorize(&run, None, &["a", "b"], "w2-final");
    let final_ran = fixture.step_with(&run, &final_bundle);
    assert_eq!(
        final_ran.code, 0,
        "WRAV-002: the twins refused for the signatures, not the fixture: {}{}",
        final_ran.stdout, final_ran.message
    );
    assert_eq!(fixture.record_state(&run), "build");

    // Twin: the blocked route demands the same boundary - a governed hold
    // with no signatures writes nothing and refuses on stderr.
    let blocked_run = fixture.admit();
    let blocked_bundle = fixture.authorize(&blocked_run, None, &["a", "b"], "w2-blocked");
    let blocked_step = fixture.step_with(&blocked_run, &blocked_bundle);
    assert_eq!(blocked_step.code, 0);
    let before = fixture.snapshot();
    let hold_ran = fixture.hold_with(&blocked_run, "work/blocker.txt", None);
    assert_eq!(hold_ran.code, 1, "WRAV-002: a hold refusal exits 1");
    assert_eq!(
        hold_ran.message,
        format!(
            "hold refused; authorization: {}",
            signatures_none(2, &blocked_run)
        ),
        "WRAV-002: the hold's missing-signature refusal is exact"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-002: the hold refusal writes nothing"
    );
}

/// WRAV-003: changes to work, policy, Run, State, target, selected input, or sequence invalidate the same otherwise-valid approval.
#[test]
fn wrav_003() {
    let _lane = support::hook_lane();
    let body = "\
[roots]
issue = \"work\"

[states.intake]
prompt = \"Intake the request.\"

[states.build]
prompt = \"Build the change.\"
inputs = [\"ship\", \"rework\", \"fast\", \"slow\"]

[states.shipped]
prompt = \"The change shipped.\"

[states.rework]
prompt = \"The change needs rework.\"

[states.done]
prompt = \"The change is done.\"

[[transitions]]
from = \"intake\"
to = \"build\"

[[transitions]]
from = \"build\"
to = \"shipped\"
input = \"ship\"

[[transitions]]
from = \"build\"
to = \"rework\"
input = \"rework\"

[[transitions]]
from = \"build\"
to = \"done\"
input = \"fast\"

[[transitions]]
from = \"build\"
to = \"done\"
input = \"slow\"

[[transitions]]
from = \"rework\"
to = \"done\"
";
    let keys = KeySet::new("wrav003", &["a", "b", "lead"]);
    let declaration = Declaration::independent(&keys);
    let fixture = Fixture::governed("wrav003", body, keys, declaration);

    // Positive control: one authorized transition moves intake -> build.
    let run = fixture.admit();
    let first = fixture.authorize(&run, None, &["a", "b"], "w3-control");
    let stepped = fixture.step_with(&run, &first);
    assert_eq!(
        stepped.code, 0,
        "WRAV-003: the positive control succeeds: {}{}",
        stepped.stdout, stepped.message
    );
    assert_eq!(fixture.record_state(&run), "build");

    // `build` is a branching State and verdict validation precedes the
    // authorization boundary (the ticket's step order), so the lanes below
    // need a live verdict to reach their pinned authorization refusals at
    // all; refusals write nothing, so it stays live for both.
    fixture.publish_verdict(&run, "build", "ship");

    // State: reviews signed at intake no longer verify at build, even with
    // a freshly rendered approval over exactly those review files.
    let stale_approval = {
        let args = review_args(&first);
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let approval_intent = fixture.intent(&run, "step", &refs, "w3-state-approval");
        fixture
            .keys
            .sign("lead", APPROVE_NAMESPACE, &approval_intent)
    };
    let before = fixture.snapshot();
    let state_ran = fixture.step_with(
        &run,
        &Bundle {
            reviews: first.reviews.clone(),
            approve: stale_approval,
        },
    );
    assert_eq!(state_ran.code, 0);
    assert_eq!(
        state_ran.stdout,
        step_refused(&run, &review_unverified("a")),
        "WRAV-003: a changed current State invalidates the stale reviews"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-003: the stale-State refusal writes nothing"
    );

    // Sequence: fresh reviews at the next sequence beside the consumed
    // approval meet the replay refusal.
    let fresh_reviews = {
        let intent = fixture.intent(&run, "step", &[], "w3-seq-review");
        (
            fixture.keys.sign("a", REVIEW_NAMESPACE, &intent),
            fixture.keys.sign("b", REVIEW_NAMESPACE, &intent),
        )
    };
    let before = fixture.snapshot();
    let replay_ran = fixture.step_with(
        &run,
        &Bundle {
            reviews: vec![
                ("a".to_owned(), fresh_reviews.0),
                ("b".to_owned(), fresh_reviews.1),
            ],
            approve: first.approve.clone(),
        },
    );
    assert_eq!(replay_ran.code, 0);
    assert_eq!(
        replay_ran.stdout,
        step_refused(&run, &replay_detail(&fixture.engine_root_shown(), &run, 1)),
        "WRAV-003: the consumed approval is refused at the next sequence"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-003: the replay refusal writes nothing"
    );

    // Target: the bundle signed for `ship` refuses once the verdict selects
    // `rework`.
    let target_run = fixture.admit();
    let target_first = fixture.authorize(&target_run, None, &["a", "b"], "w3-target-move");
    let moved = fixture.step_with(&target_run, &target_first);
    assert_eq!(
        moved.code, 0,
        "WRAV-003: the target twin's Run reaches build"
    );
    fixture.publish_verdict(&target_run, "build", "ship");
    let ship_bundle = fixture.authorize(&target_run, None, &["a", "b"], "w3-target-ship");
    fixture.publish_verdict(&target_run, "build", "rework");
    let before = fixture.snapshot();
    let target_ran = fixture.step_with(&target_run, &ship_bundle);
    assert_eq!(target_ran.code, 0);
    assert_eq!(
        target_ran.stdout,
        step_refused(&target_run, &review_unverified("a")),
        "WRAV-003: a changed target invalidates the signed bundle"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-003: the changed-target refusal writes nothing"
    );

    // Selected input: two inputs to the same target - the bundle signed for
    // `fast` refuses once the verdict selects `slow`.
    let input_run = fixture.admit();
    let input_first = fixture.authorize(&input_run, None, &["a", "b"], "w3-input-move");
    let moved = fixture.step_with(&input_run, &input_first);
    assert_eq!(
        moved.code, 0,
        "WRAV-003: the input twin's Run reaches build"
    );
    fixture.publish_verdict(&input_run, "build", "fast");
    let fast_bundle = fixture.authorize(&input_run, None, &["a", "b"], "w3-input-fast");
    fixture.publish_verdict(&input_run, "build", "slow");
    let before = fixture.snapshot();
    let input_ran = fixture.step_with(&input_run, &fast_bundle);
    assert_eq!(input_ran.code, 0);
    assert_eq!(
        input_ran.stdout,
        step_refused(&input_run, &review_unverified("a")),
        "WRAV-003: a changed selected input invalidates the signed bundle"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-003: the changed-input refusal writes nothing"
    );

    // Work: editing a declared work file invalidates the bundle.
    let work_run = fixture.admit();
    let work_bundle = fixture.authorize(&work_run, None, &["a", "b"], "w3-work");
    let original =
        fs::read(fixture.root().join("work/content.txt")).expect("read the declared work file");
    fs::write(
        fixture.root().join("work/content.txt"),
        "Changed contributor bytes.\n",
    )
    .expect("mutate the declared work file");
    let before = fixture.snapshot();
    let work_ran = fixture.step_with(&work_run, &work_bundle);
    assert_eq!(work_ran.code, 0);
    assert_eq!(
        work_ran.stdout,
        step_refused(&work_run, &review_unverified("a")),
        "WRAV-003: changed declared work invalidates the signed bundle"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-003: the changed-work refusal writes nothing"
    );
    fs::write(fixture.root().join("work/content.txt"), &original)
        .expect("restore the declared work file");

    // Run: a bundle minted for one Run refuses on another.
    let signer_run = fixture.admit();
    let submit_run = fixture.admit();
    let wrong_run_bundle = fixture.authorize(&signer_run, None, &["a", "b"], "w3-cross");
    let before = fixture.snapshot();
    let cross_ran = fixture.step_with(&submit_run, &wrong_run_bundle);
    assert_eq!(cross_ran.code, 0);
    assert_eq!(
        cross_ran.stdout,
        step_refused(&submit_run, &review_unverified("a")),
        "WRAV-003: a bundle bound to another Run id refuses"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-003: the wrong-Run refusal writes nothing"
    );

    // Policy: an amended declaration cannot reuse the approval signed under
    // the pinned policy - the pin itself refuses first, and nothing moves.
    let policy_keys = KeySet::new("wrav003-policy", &["a", "b", "lead"]);
    let policy_declaration = Declaration::independent(&policy_keys);
    let policy_fixture = Fixture::governed(
        "wrav003-policy",
        ORDINARY_BODY,
        policy_keys,
        policy_declaration,
    );
    let policy_run = policy_fixture.admit();
    let policy_bundle = policy_fixture.authorize(&policy_run, None, &["a", "b"], "w3-policy");
    let pinned = sha256_hex(&policy_fixture.runbook_bytes);
    let amended = String::from_utf8_lossy(&policy_fixture.runbook_bytes)
        .replace("approver-may-review = false", "approver-may-review = true");
    fs::write(policy_fixture.root().join(".ratmac/ratmac.toml"), &amended)
        .expect("amend the declared policy");
    let before = policy_fixture.snapshot();
    let policy_ran = policy_fixture.step_with(&policy_run, &policy_bundle);
    assert_eq!(
        policy_ran.code, 1,
        "WRAV-003: the drifted runbook is a hard refusal"
    );
    assert_eq!(
        policy_ran.message,
        pin_mismatch(
            "step",
            &policy_run,
            &sha256_hex(amended.as_bytes()),
            &pinned
        ),
        "WRAV-003: the amended policy cannot authorize under the old pin"
    );
    assert_eq!(
        policy_fixture.snapshot(),
        before,
        "WRAV-003: the amended-policy refusal writes nothing"
    );
}

/// WRAV-004: replaying consumed approval and interrupting at each evidence/state boundary never authorizes a second transition or loses accepted history.
#[test]
fn wrav_004() {
    let _lane = support::hook_lane();
    let body = "\
[roots]
issue = \"work\"

[states.intake]
prompt = \"Intake the request.\"

[states.gate]
prompt = \"Pass the gate.\"
inputs = [\"pass\", \"regress\"]

[states.mid]
prompt = \"Wait in the middle.\"

[states.done]
prompt = \"Ship the change.\"

[[transitions]]
from = \"intake\"
to = \"gate\"

[[transitions]]
from = \"gate\"
to = \"done\"
input = \"pass\"

[[transitions]]
from = \"gate\"
to = \"mid\"
input = \"regress\"

[[transitions]]
from = \"mid\"
to = \"gate\"
";
    let keys = KeySet::new("wrav004", &["a", "b", "lead"]);
    let declaration = Declaration::independent(&keys);
    let fixture = Fixture::governed("wrav004", body, keys, declaration);
    let run = fixture.admit();

    // Interruption before the authorization archive: nothing is written and
    // the same signatures stay valid at the same sequence.
    let first = fixture.authorize(&run, None, &["a", "b"], "w4-one");
    let before = fixture.snapshot();
    let faulted = fault_step(&fixture, &run, &first, "before-authorization-archive");
    assert_eq!(
        faulted.code, 1,
        "WRAV-004: an injected fault is a hard refusal"
    );
    assert_eq!(
        faulted.message,
        "step: injected step fault at before-authorization-archive\nnext: rtm doctor",
        "WRAV-004: the fault names its boundary inside today's family"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-004: a fault before the authorization archive writes nothing"
    );
    assert!(
        fixture.authorizations(&run).is_empty(),
        "WRAV-004: no sequence is spent before the archive"
    );
    let retried = fixture.step_with(&run, &first);
    assert_eq!(
        retried.code, 0,
        "WRAV-004: the same signatures still authorize the same sequence: {}{}",
        retried.stdout, retried.message
    );
    assert_eq!(fixture.record_state(&run), "gate");
    fixture.assert_authorization(&run, 1, "step", "intake", "gate", &first);

    // Interruption after the authorization archive is still graceful: the
    // pre-transition bytes return, the sequence is not spent, and the same
    // signatures succeed at the same sequence.
    fixture.publish_verdict(&run, "gate", "regress");
    let second = fixture.authorize(&run, None, &["a", "b"], "w4-two");
    let before = fixture.snapshot();
    let faulted = fault_step(&fixture, &run, &second, "after-authorization-archive");
    assert_eq!(faulted.code, 1);
    assert_eq!(
        faulted.message,
        "step: injected step fault at after-authorization-archive\nnext: rtm doctor",
        "WRAV-004: the post-archive fault names its boundary"
    );
    assert_eq!(
        fixture.record_state(&run),
        "gate",
        "WRAV-004: the Run Record stays at the old State"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-004: the post-archive rollback returns the pre-transition bytes"
    );
    assert!(
        !fixture.authorizations(&run).contains_key("000002.toml"),
        "WRAV-004: the interrupted sequence is not spent"
    );
    fixture.publish_verdict(&run, "gate", "regress");
    let retried = fixture.step_with(&run, &second);
    assert_eq!(
        retried.code, 0,
        "WRAV-004: the same signatures succeed at the same sequence: {}{}",
        retried.stdout, retried.message
    );
    fixture.assert_authorization(&run, 2, "step", "gate", "mid", &second);
    assert_eq!(fixture.record_state(&run), "mid");

    // Straight back to the gate, then interrupt before the State replace.
    let third = fixture.authorize(&run, None, &["a", "b"], "w4-three");
    let moved = fixture.step_with(&run, &third);
    assert_eq!(moved.code, 0);
    assert_eq!(fixture.record_state(&run), "gate");

    fixture.publish_verdict(&run, "gate", "regress");
    let fourth = fixture.authorize(&run, None, &["a", "b"], "w4-four");
    let before = fixture.snapshot();
    let faulted = fault_step(&fixture, &run, &fourth, "before-state-replace");
    assert_eq!(faulted.code, 1);
    assert_eq!(
        faulted.message, "step: injected step fault at before-state-replace\nnext: rtm doctor",
        "WRAV-004: the pre-replace fault names its boundary"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-004: the pre-replace rollback returns the pre-transition bytes"
    );
    fixture.publish_verdict(&run, "gate", "regress");
    let retried = fixture.step_with(&run, &fourth);
    assert_eq!(
        retried.code, 0,
        "WRAV-004: the same bundle completes after the rollback: {}{}",
        retried.stdout, retried.message
    );
    assert_eq!(fixture.record_state(&run), "mid");

    let fifth = fixture.authorize(&run, None, &["a", "b"], "w4-five");
    let moved = fixture.step_with(&run, &fifth);
    assert_eq!(moved.code, 0);
    assert_eq!(fixture.record_state(&run), "gate");

    // Interruption after the State replace: the transition is committed and
    // the authorization stays as truthful history, but no second transition
    // can ride it.
    fixture.publish_verdict(&run, "gate", "regress");
    let sixth = fixture.authorize(&run, None, &["a", "b"], "w4-six");
    let faulted = fault_step(&fixture, &run, &sixth, "after-state-replace");
    assert_eq!(faulted.code, 1);
    assert_eq!(
        faulted.message, "step: injected step fault at after-state-replace\nnext: rtm doctor",
        "WRAV-004: the post-replace fault names its boundary"
    );
    assert_eq!(
        fixture.record_state(&run),
        "mid",
        "WRAV-004: a post-replace fault observes the successor State"
    );
    fixture.assert_authorization(&run, 6, "step", "gate", "mid", &sixth);
    assert!(
        !fixture.run_dir(&run).join("verdict.toml").exists(),
        "WRAV-004: the committed transition consumed its verdict"
    );

    // Replaying the consumed approval never authorizes a second transition.
    let before = fixture.snapshot();
    let replayed = fixture.step_with(&run, &sixth);
    assert_eq!(replayed.code, 0);
    assert_eq!(
        replayed.stdout,
        step_refused(&run, &replay_detail(&fixture.engine_root_shown(), &run, 6)),
        "WRAV-004: the consumed approval is refused at the next sequence"
    );
    assert_eq!(
        fixture.snapshot(),
        before,
        "WRAV-004: the replay refusal writes nothing"
    );

    // A fresh bundle completes the motion, and the accepted history is
    // intact: every archived authorization, in order, and the appended
    // transitions with only today's deliberate post-replace gap.
    let seventh = fixture.authorize(&run, None, &["a", "b"], "w4-seven");
    let finished = fixture.step_with(&run, &seventh);
    assert_eq!(
        finished.code, 0,
        "WRAV-004: a fresh intent completes the motion: {}{}",
        finished.stdout, finished.message
    );
    assert_eq!(fixture.record_state(&run), "gate");
    let records = fixture.authorizations(&run);
    let expected: Vec<String> = (1..=7)
        .map(|sequence| format!("{:06}.toml", sequence))
        .collect();
    assert_eq!(
        records.keys().cloned().collect::<Vec<_>>(),
        expected,
        "WRAV-004: every accepted authorization stays archived in order",
    );
    for (index, bytes) in records.values().enumerate() {
        let text = String::from_utf8(bytes.to_vec()).expect("the record is UTF-8");
        let value: toml::Value = text.parse().expect("the record is strict TOML");
        assert_eq!(
            value["sequence"].as_integer(),
            Some(index as i64 + 1),
            "WRAV-004: the archive sequences stay monotonic"
        );
        assert_eq!(
            value["origin"].as_str(),
            Some("test-injected"),
            "WRAV-004: no accepted history claims host enrollment"
        );
    }
    let history =
        fs::read_to_string(fixture.root().join(".ratmac/log.md")).expect("read the history");
    assert_eq!(
        history.matches("- Transition: intake -> gate; Run").count(),
        1,
        "WRAV-004: completed transitions append their history"
    );
    assert_eq!(
        history.matches("- Transition: gate -> mid; Run").count(),
        2,
        "WRAV-004: only the post-replace interruption leaves today's deliberate gap"
    );
    assert_eq!(
        history.matches("- Transition: mid -> gate; Run").count(),
        3,
        "WRAV-004: the fresh retry after the gap still records its transition"
    );
}

/// One governed step under one injected `RATMAC_TEST_STEP_FAULT` boundary.
/// The caller's test holds the shared environment lane for its whole body,
/// so the fault variable reaches this in-process call alone.
fn fault_step(fixture: &Fixture, run: &str, bundle: &Bundle, boundary: &str) -> Ran {
    let _fault = support::StepFault::set(boundary);
    fixture.step_with(run, bundle)
}

/// WRAV-005: malformed policy refuses at parsing/diagnosis, historical Runs retain truthful no-policy provenance, and the live governed workflow refuses without its independent trust anchor.
#[test]
fn wrav_005() {
    let _lane = support::hook_lane();
    // A malformed policy refuses at parse through each loader's own shape.
    let keys = KeySet::new("wrav005", &["a", "lead"]);
    let malformed = |minimum: u32, approver: &str, duplicate: bool| {
        let mut table = format!(
            "\n[review]\nmanifest = \"{}/absent-enrollment.toml\"\n\
             policy-digest = \"{}\"\nverifier-digest = \"{}\"\nsigners-digest = \"{}\"\n\
             minimum-reviews = {minimum}\napprover = \"{approver}\"\n\
             approver-may-review = false\nwork = [\"work\"]\n",
            shown(keys.dir.path()),
            "0".repeat(64),
            "0".repeat(64),
            "0".repeat(64),
        );
        let mut reviewers = vec![("a".to_owned(), keys.public("a").to_owned())];
        if duplicate {
            reviewers.push(("b".to_owned(), keys.public("a").to_owned()));
        } else {
            reviewers.push(("lead".to_owned(), keys.public("lead").to_owned()));
        }
        for (label, public) in reviewers {
            table.push_str(&format!(
                "\n[[review.reviewers]]\nlabel = \"{label}\"\nkey = \"{public}\"\n"
            ));
        }
        table
    };

    // RB703: the approver names no declared reviewer, at `rtm start`.
    let start_fixture = TempRepo::new("wrav005-rb703");
    start_fixture.write("work/content.txt", "Contributor bytes.\n");
    start_fixture.write(
        ".ratmac/ratmac.toml",
        &format!("{ORDINARY_BODY}{}", malformed(1, "ghost", false)),
    );
    start_fixture.commit_all("fixture baseline");
    let started = rtm(start_fixture.root(), &["start"]);
    assert_eq!(
        started.code, 1,
        "WRAV-005: a malformed review declaration refuses at start"
    );
    assert_eq!(
        started.message,
        "start: parse .ratmac/ratmac.toml [RB703]: invalid review declaration: \
         approver \"ghost\" names no declared reviewer",
        "WRAV-005: RB703 renders exactly through today's start parse path"
    );

    // `rtm doctor` reports the same defect as an error finding, exiting 2.
    let doctored = rtm(start_fixture.root(), &["doctor"]);
    assert_eq!(
        doctored.code, 2,
        "WRAV-005: an error finding exits 2: {}{}",
        doctored.stdout, doctored.message
    );
    assert!(
        doctored.stdout.contains("RB703")
            && doctored.stdout.contains(
                "invalid review declaration: approver \"ghost\" names no declared reviewer"
            ),
        "WRAV-005: the doctor finding bears its code and detail: {}",
        doctored.stdout
    );

    // RB704 and RB702 refuse through status, step, and hold on a Run that
    // was admitted while the runbook was still plain.
    let loader_fixture = TempRepo::new("wrav005-loaders");
    loader_fixture.write("work/content.txt", "Contributor bytes.\n");
    loader_fixture.write("work/blocker.txt", "Opaque blocker.\n");
    loader_fixture.write(".ratmac/ratmac.toml", ORDINARY_BODY);
    loader_fixture.commit_all("fixture baseline");
    let started = rtm(loader_fixture.root(), &["start"]);
    assert_eq!(
        started.code, 0,
        "WRAV-005: the plain fixture admits its Run: {}{}",
        started.stdout, started.message
    );
    let loader_run = started_run_id(&started);

    // RB704 through `rtm status`: an impossible count.
    fs::write(
        loader_fixture.root().join(".ratmac/ratmac.toml"),
        format!("{ORDINARY_BODY}{}", malformed(3, "lead", false)),
    )
    .expect("rewrite the runbook");
    let status = rtm(loader_fixture.root(), &["status", "--run", &loader_run]);
    assert_eq!(status.code, 1, "WRAV-005: a parse refusal exits 1");
    assert_eq!(
        status.message,
        "status: parse .ratmac/ratmac.toml [RB704]: invalid review declaration: \
         minimum-reviews 3 exceeds the 1 distinct reviews the declared roles can supply\n\
         next: rtm doctor",
        "WRAV-005: RB704 renders exactly through today's status parse path"
    );

    // RB703 through `rtm step`.
    fs::write(
        loader_fixture.root().join(".ratmac/ratmac.toml"),
        format!("{ORDINARY_BODY}{}", malformed(1, "ghost", false)),
    )
    .expect("rewrite the runbook");
    let stepped = rtm(loader_fixture.root(), &["step", "--run", &loader_run]);
    assert_eq!(stepped.code, 1);
    assert_eq!(
        stepped.message,
        "step: parse .ratmac/ratmac.toml [RB703]: invalid review declaration: \
         approver \"ghost\" names no declared reviewer\nnext: rtm doctor",
        "WRAV-005: RB703 renders exactly through today's step parse path"
    );

    // RB702 through `rtm hold`: the same public key under two labels.
    fs::write(
        loader_fixture.root().join(".ratmac/ratmac.toml"),
        format!("{ORDINARY_BODY}{}", malformed(1, "a", true)),
    )
    .expect("rewrite the runbook");
    let held = rtm(
        loader_fixture.root(),
        &[
            "hold",
            "--run",
            &loader_run,
            "--blocker",
            "work/blocker.txt",
            "--confirm",
            &format!("hold {loader_run}"),
        ],
    );
    assert_eq!(held.code, 1);
    assert_eq!(
        held.message,
        "hold refused; parse .ratmac/ratmac.toml [RB702]: invalid review declaration: \
         the same public key appears under labels \"a\" and \"b\"",
        "WRAV-005: RB702 renders exactly through today's hold parse path"
    );

    // Historical Runs keep their truthful no-policy provenance.
    let historic = Fixture::plain("wrav005-historic", ORDINARY_BODY);
    let started = rtm(historic.root(), &["start"]);
    assert_eq!(
        started.code, 0,
        "WRAV-005: the ungoverned fixture admits its Run: {}{}",
        started.stdout, started.message
    );
    let historic_run = started_run_id(&started);
    let report = rtm(historic.root(), &["status", "--run", &historic_run]);
    assert_eq!(report.code, 0);
    assert!(
        !report.stdout.to_lowercase().contains("review"),
        "WRAV-005: no surface reports an ungoverned Run as reviewed: {}",
        report.stdout
    );
    let moved = rtm(historic.root(), &["step", "--run", &historic_run]);
    assert_eq!(
        moved.code, 0,
        "WRAV-005: the ungoverned Run moves without one signature"
    );
    assert_eq!(
        moved.stdout,
        format!("Build the change.\nnext: rtm step --run {historic_run}\n"),
        "WRAV-005: the ungoverned step keeps today's bytes"
    );
    let flagged = rtm(
        historic.root(),
        &[
            "step",
            "--run",
            &historic_run,
            "--review",
            "a:nowhere.sig",
            "--approve",
            "nowhere-approve.sig",
        ],
    );
    assert_eq!(
        flagged.code, 2,
        "WRAV-005: review options on an ungoverned Run are a usage refusal"
    );
    assert_eq!(
        flagged.message, "step: --review and --approve address no declared review policy",
        "WRAV-005: the ungoverned refusal is exact"
    );

    // Even a later runbook that declares review cannot govern that Run: the
    // pinned class is what it was admitted under.
    fs::write(
        historic.root().join(".ratmac/ratmac.toml"),
        format!(
            "{ORDINARY_BODY}\n[review]\nmanifest = \"{}/absent.toml\"\n\
             policy-digest = \"{}\"\nverifier-digest = \"{}\"\nsigners-digest = \"{}\"\n\
             minimum-reviews = 1\napprover = \"a\"\napprover-may-review = false\n\
             work = [\"work\"]\n\n[[review.reviewers]]\nlabel = \"a\"\nkey = \"{}\"\n",
            shown(historic.keys.dir.path()),
            "0".repeat(64),
            "0".repeat(64),
            "0".repeat(64),
            historic.keys.public("a"),
        ),
    )
    .expect("declare review after admission");
    let flagged = rtm(
        historic.root(),
        &[
            "step",
            "--run",
            &historic_run,
            "--review",
            "a:x.sig",
            "--approve",
            "y.sig",
        ],
    );
    assert_eq!(
        flagged.code, 2,
        "WRAV-005: a Run admitted under no policy stays ungoverned"
    );
    assert_eq!(
        flagged.message, "step: --review and --approve address no declared review policy",
        "WRAV-005: governance is fixed by the runbook pin recorded at admission"
    );

    // The live governed workflow refuses without its independent trust
    // anchor: no enrollment manifest exists, and nothing is written.
    let anchor_keys = KeySet::new("wrav005-anchor", &["a", "b", "lead"]);
    let mut anchor_declaration = Declaration::independent(&anchor_keys);
    anchor_declaration.trust = false;
    anchor_declaration.manifest_on_disk = false;
    let anchor = Fixture::governed(
        "wrav005-anchor",
        ORDINARY_BODY,
        anchor_keys,
        anchor_declaration,
    );
    let before = anchor.snapshot();
    let anchored = rtm(anchor.root(), &["start"]);
    assert_eq!(
        anchored.code, 1,
        "WRAV-005: a governed admission without its trust anchor refuses"
    );
    assert_eq!(
        anchored.message,
        start_trust_origin(&format!(
            "the enrollment manifest {}/enrollment-manifest.toml is missing or unreadable",
            shown(anchor.keys.dir.path())
        )),
        "WRAV-005: the missing trust anchor names the manifest exactly"
    );
    assert_eq!(
        anchor.snapshot(),
        before,
        "WRAV-005: the refused admission writes nothing and mints no Run"
    );
}

/// WRAV-006: the production signature verifier accepts independently enrolled fixture identities but rejects self-enrollment, a builder key under a second label, the builder acting as Approver after an outside review, a review signature reused as approval, a substituted policy or verifier, and an authority amendment signed only by its proposed replacement authority.
#[test]
fn wrav_006() {
    let _lane = support::hook_lane();
    // Positive: the production verifier accepts the independently enrolled
    // fixture identities and the transition moves.
    let positive_keys = KeySet::new("wrav006-positive", &["a", "b", "lead"]);
    let positive_declaration = Declaration::independent(&positive_keys);
    let positive = Fixture::governed(
        "wrav006-positive",
        ORDINARY_BODY,
        positive_keys,
        positive_declaration,
    );
    let positive_run = positive.admit();
    let bundle = positive.authorize(&positive_run, None, &["a", "b"], "w6-positive");
    let moved = positive.step_with(&positive_run, &bundle);
    assert_eq!(
        moved.code, 0,
        "WRAV-006: the host verifier accepts the enrolled identities: {}{}",
        moved.stdout, moved.message
    );
    positive.assert_authorization(&positive_run, 1, "step", "intake", "build", &bundle);

    // Self-enrollment: the builder's own key among the reviewers.
    let self_keys = KeySet::new("wrav006-self", &["builder", "lead"]);
    let mut self_declaration = Declaration::with_reviewers(vec![
        ("a".to_owned(), self_keys.public("builder").to_owned()),
        ("lead".to_owned(), self_keys.public("lead").to_owned()),
    ]);
    self_declaration.minimum_reviews = 1;
    self_declaration.builder_keys = vec![self_keys.public("builder").to_owned()];
    let self_fixture =
        Fixture::governed("wrav006-self", ORDINARY_BODY, self_keys, self_declaration);
    let refused = rtm(self_fixture.root(), &["start"]);
    assert_eq!(
        refused.code, 1,
        "WRAV-006: self-enrollment refuses admission"
    );
    assert_eq!(
        refused.message,
        start_trust_origin("the reviewer \"a\" carries the builder's enrolled key"),
        "WRAV-006: a builder key among the reviewers is named exactly"
    );

    // The same builder key under a second label.
    let alias_keys = KeySet::new("wrav006-alias", &["a", "builder", "lead"]);
    let mut alias_declaration = Declaration::with_reviewers(vec![
        ("a".to_owned(), alias_keys.public("a").to_owned()),
        ("b".to_owned(), alias_keys.public("builder").to_owned()),
        ("lead".to_owned(), alias_keys.public("lead").to_owned()),
    ]);
    alias_declaration.minimum_reviews = 1;
    alias_declaration.builder_keys = vec![alias_keys.public("builder").to_owned()];
    let alias_fixture = Fixture::governed(
        "wrav006-alias",
        ORDINARY_BODY,
        alias_keys,
        alias_declaration,
    );
    let refused = rtm(alias_fixture.root(), &["start"]);
    assert_eq!(refused.code, 1, "WRAV-006: a relabeled builder key refuses");
    assert_eq!(
        refused.message,
        start_trust_origin("the reviewer \"b\" carries the builder's enrolled key"),
        "WRAV-006: membership compares key identity, not only labels"
    );

    // The builder acting as Approver after an outside review.
    let approver_keys = KeySet::new("wrav006-approver", &["a", "builder"]);
    let mut approver_declaration = Declaration::with_reviewers(vec![
        ("a".to_owned(), approver_keys.public("a").to_owned()),
        (
            "lead".to_owned(),
            approver_keys.public("builder").to_owned(),
        ),
    ]);
    approver_declaration.minimum_reviews = 1;
    approver_declaration.builder_keys = vec![approver_keys.public("builder").to_owned()];
    let approver_fixture = Fixture::governed(
        "wrav006-approver",
        ORDINARY_BODY,
        approver_keys,
        approver_declaration,
    );
    let refused = rtm(approver_fixture.root(), &["start"]);
    assert_eq!(refused.code, 1, "WRAV-006: a builder Approver refuses");
    assert_eq!(
        refused.message,
        start_trust_origin("the approver \"lead\" carries the builder's enrolled key"),
        "WRAV-006: the builder cannot be the Approver"
    );

    // A review signature reused as the approval.
    let reuse_keys = KeySet::new("wrav006-reuse", &["a", "lead"]);
    let mut reuse_declaration = Declaration::with_reviewers(vec![
        ("a".to_owned(), reuse_keys.public("a").to_owned()),
        ("lead".to_owned(), reuse_keys.public("lead").to_owned()),
    ]);
    reuse_declaration.minimum_reviews = 1;
    let reuse_fixture = Fixture::governed(
        "wrav006-reuse",
        ORDINARY_BODY,
        reuse_keys,
        reuse_declaration,
    );
    let reuse_run = reuse_fixture.admit();
    let reused_review = {
        let intent = reuse_fixture.intent(&reuse_run, "step", &[], "w6-reuse-review");
        reuse_fixture.keys.sign("a", REVIEW_NAMESPACE, &intent)
    };
    let before = reuse_fixture.snapshot();
    let reused = reuse_fixture.step_with(
        &reuse_run,
        &Bundle {
            reviews: vec![("a".to_owned(), reused_review.clone())],
            approve: reused_review,
        },
    );
    assert_eq!(
        reused.code, 0,
        "WRAV-006: the reused review refuses the step"
    );
    assert_eq!(
        reused.stdout,
        step_refused(&reuse_run, &approval_no_identity("lead")),
        "WRAV-006: a review-purpose signature is no approval under the production verifier"
    );
    assert_eq!(
        reuse_fixture.snapshot(),
        before,
        "WRAV-006: the reused-signature refusal writes nothing"
    );

    // A substituted verifier: the enrollment binds a digest the real pinned
    // verifier does not hash to.
    let substituted_digest = sha256_hex(b"not the verifier");
    let verifier_keys = KeySet::new("wrav006-verifier", &["a", "b", "lead"]);
    let mut verifier_declaration = Declaration::independent(&verifier_keys);
    verifier_declaration.verifier_sha256 = Some(substituted_digest.clone());
    let verifier_fixture = Fixture::governed(
        "wrav006-verifier",
        ORDINARY_BODY,
        verifier_keys,
        verifier_declaration,
    );
    let refused = rtm(verifier_fixture.root(), &["start"]);
    assert_eq!(refused.code, 1, "WRAV-006: a substituted verifier refuses");
    let real = sha256_hex(&fs::read(KEYGEN).expect("read the pinned verifier"));
    assert_eq!(
        refused.message,
        start_trust_origin(&format!(
            "the verifier at {KEYGEN} hashes {real}, the enrollment binds {substituted_digest}"
        )),
        "WRAV-006: the verifier digest is named from both sides"
    );

    // A substituted policy: the enrollment records another policy's digest.
    let policy_keys = KeySet::new("wrav006-policy", &["a", "b", "lead"]);
    let enrolled = [
        ("a".to_owned(), policy_keys.public("a").to_owned()),
        ("b".to_owned(), policy_keys.public("b").to_owned()),
        ("lead".to_owned(), policy_keys.public("lead").to_owned()),
    ];
    let true_digest = policy_digest("lead", 2, false, &enrolled);
    let substituted_policy = policy_digest("a", 2, false, &enrolled);
    let mut policy_declaration = Declaration::independent(&policy_keys);
    policy_declaration.enrolled_policy_digest = Some(substituted_policy.clone());
    let policy_fixture = Fixture::governed(
        "wrav006-policy",
        ORDINARY_BODY,
        policy_keys,
        policy_declaration,
    );
    let refused = rtm(policy_fixture.root(), &["start"]);
    assert_eq!(refused.code, 1, "WRAV-006: a substituted policy refuses");
    assert_eq!(
        refused.message,
        start_trust_origin(&format!(
            "the declared policy digest {true_digest} does not match the enrollment's \
             {substituted_policy}"
        )),
        "WRAV-006: the policy digest is named from both sides"
    );

    // A declared policy digest that disagrees with the recomputed one.
    let fake_declared_policy = sha256_hex(b"w9-declared-policy");
    let recomputed_keys = KeySet::new("wrav006-recomputed", &["a", "b", "lead"]);
    let recomputed_roles: Vec<(String, String)> = ["a", "b", "lead"]
        .iter()
        .map(|label| {
            (
                (*label).to_owned(),
                recomputed_keys.public(label).to_owned(),
            )
        })
        .collect();
    let mut recomputed_declaration = Declaration::independent(&recomputed_keys);
    recomputed_declaration.declared_policy_digest = Some(fake_declared_policy.clone());
    let recomputed_fixture = Fixture::governed(
        "wrav006-recomputed",
        ORDINARY_BODY,
        recomputed_keys,
        recomputed_declaration,
    );
    let recomputed = policy_digest("lead", 2, false, &recomputed_roles);
    let before = recomputed_fixture.snapshot();
    let refused = rtm(recomputed_fixture.root(), &["start"]);
    assert_eq!(
        refused.code, 1,
        "WRAV-006: a declared digest off the recomputed one refuses admission"
    );
    assert_eq!(
        refused.message,
        start_trust_origin(&format!(
            "the declared policy digest {fake_declared_policy} does not match the recomputed \
             {recomputed}"
        )),
        "WRAV-006: the recomputed disagreement is named from both sides"
    );
    assert_eq!(
        recomputed_fixture.snapshot(),
        before,
        "WRAV-006: the recomputed-digest refusal writes nothing"
    );

    // A declared verifier digest that disagrees with the enrollment binding.
    let real_verifier = sha256_hex(&fs::read(KEYGEN).expect("read the pinned verifier"));
    let fake_declared_verifier = sha256_hex(b"w9-declared-verifier");
    let declared_verifier_keys = KeySet::new("wrav006-declared-verifier", &["a", "b", "lead"]);
    let mut declared_verifier_declaration = Declaration::independent(&declared_verifier_keys);
    declared_verifier_declaration.declared_verifier_digest = Some(fake_declared_verifier.clone());
    let declared_verifier_fixture = Fixture::governed(
        "wrav006-declared-verifier",
        ORDINARY_BODY,
        declared_verifier_keys,
        declared_verifier_declaration,
    );
    let before = declared_verifier_fixture.snapshot();
    let refused = rtm(declared_verifier_fixture.root(), &["start"]);
    assert_eq!(
        refused.code, 1,
        "WRAV-006: a substituted declared verifier digest refuses admission"
    );
    assert_eq!(
        refused.message,
        start_trust_origin(&format!(
            "the declared verifier digest {fake_declared_verifier} does not match the \
             enrollment's {real_verifier}"
        )),
        "WRAV-006: the declared verifier digest is named from both sides"
    );
    assert_eq!(
        declared_verifier_fixture.snapshot(),
        before,
        "WRAV-006: the declared-verifier refusal writes nothing"
    );

    // A declared signers digest that disagrees with the enrollment binding.
    let declared_signers_keys = KeySet::new("wrav006-declared-signers", &["a", "b", "lead"]);
    let fake_declared_signers = sha256_hex(b"w9-declared-signers");
    let mut declared_signers_declaration = Declaration::independent(&declared_signers_keys);
    declared_signers_declaration.declared_signers_digest = Some(fake_declared_signers.clone());
    let declared_signers_fixture = Fixture::governed(
        "wrav006-declared-signers",
        ORDINARY_BODY,
        declared_signers_keys,
        declared_signers_declaration,
    );
    let real_declared_signers = sha256_hex(
        &fs::read(declared_signers_fixture.keys.dir.join("allowed_signers"))
            .expect("read the allowed-signers file"),
    );
    let before = declared_signers_fixture.snapshot();
    let refused = rtm(declared_signers_fixture.root(), &["start"]);
    assert_eq!(
        refused.code, 1,
        "WRAV-006: a substituted declared signers digest refuses admission"
    );
    assert_eq!(
        refused.message,
        start_trust_origin(&format!(
            "the declared signers digest {fake_declared_signers} does not match the \
             enrollment's {real_declared_signers}"
        )),
        "WRAV-006: the declared signers digest is named from both sides"
    );
    assert_eq!(
        declared_signers_fixture.snapshot(),
        before,
        "WRAV-006: the declared-signers refusal writes nothing"
    );

    // The stated order: with a substituted declared verifier digest AND a
    // substituted declared signers digest, the verifier pair refuses first.
    let order_keys = KeySet::new("wrav006-order", &["a", "b", "lead"]);
    let fake_order_verifier = sha256_hex(b"w9-order-verifier");
    let mut order_declaration = Declaration::independent(&order_keys);
    order_declaration.declared_verifier_digest = Some(fake_order_verifier.clone());
    order_declaration.declared_signers_digest = Some(sha256_hex(b"w9-order-signers"));
    let order_fixture = Fixture::governed(
        "wrav006-order",
        ORDINARY_BODY,
        order_keys,
        order_declaration,
    );
    let before = order_fixture.snapshot();
    let refused = rtm(order_fixture.root(), &["start"]);
    assert_eq!(refused.code, 1, "WRAV-006: two digest disagreements refuse");
    assert_eq!(
        refused.message,
        start_trust_origin(&format!(
            "the declared verifier digest {fake_order_verifier} does not match the \
             enrollment's {real_verifier}"
        )),
        "WRAV-006: the digest checks refuse in the stated order, verifier before signers"
    );
    assert_eq!(
        order_fixture.snapshot(),
        before,
        "WRAV-006: the ordered refusal writes nothing"
    );

    // The same boundary composes as the enrollment sub-boundary of one
    // governed step: the Run is admitted under a valid origin, the signers
    // binding turns substituted, and an otherwise-valid authorization meets
    // the refusal at the step's verification boundary.
    let frame_keys = KeySet::new("wrav006-frame", &["a", "b", "lead"]);
    let frame_declaration = Declaration::independent(&frame_keys);
    let mut frame_fixture = Fixture::governed(
        "wrav006-frame",
        ORDINARY_BODY,
        frame_keys,
        frame_declaration,
    );
    let frame_run = frame_fixture.admit();
    let frame_bundle = frame_fixture.authorize(&frame_run, None, &["a", "b"], "w6-frame");
    let frame_signers = frame_fixture.keys.dir.join("allowed_signers");
    let real_frame_signers =
        sha256_hex(&fs::read(&frame_signers).expect("read the allowed-signers file"));
    let fake_frame_signers = sha256_hex(b"w9-frame-signers");
    let edited_manifest = frame_fixture
        .manifest_text
        .replace(&real_frame_signers, &fake_frame_signers);
    let frame_trust = TestTrust {
        manifest_bytes: edited_manifest.into_bytes(),
        signers_path: frame_signers.clone(),
        verifier_path: PathBuf::from(KEYGEN),
        verifier_sha256: real_verifier.clone(),
    };
    frame_fixture.reseat_trust(frame_trust);
    let before = frame_fixture.snapshot();
    let framed = frame_fixture.step_with(&frame_run, &frame_bundle);
    assert_eq!(
        framed.code, 0,
        "WRAV-006: the enrollment refusal is a refused step"
    );
    assert_eq!(
        framed.stdout,
        step_refused(
            &frame_run,
            &format!(
                "enrollment: observed the allowed-signers file at {} hashes {real_frame_signers}, \
                 the enrollment binds {fake_frame_signers}; expected the enrolled trust origin",
                shown(&frame_signers)
            ),
        ),
        "WRAV-006: the step frame composes the enrollment detail exactly"
    );
    assert_eq!(
        frame_fixture.snapshot(),
        before,
        "WRAV-006: the enrollment step refusal writes nothing"
    );

    // An authority amendment signed only by its proposed replacement: the
    // pinned policy governs the Run for its whole life, so the amendment
    // authenticates nothing and the accepted history stays byte-identical.
    let amendment_keys = KeySet::new("wrav006-amendment", &["a", "b", "lead", "newlead"]);
    let amendment_declaration = Declaration::independent(&amendment_keys);
    let amendment_fixture = Fixture::governed(
        "wrav006-amendment",
        ORDINARY_BODY,
        amendment_keys,
        amendment_declaration,
    );
    let amendment_run = amendment_fixture.admit();
    let amendment_bundle =
        amendment_fixture.authorize(&amendment_run, None, &["a", "b"], "w6-amendment");
    let moved = amendment_fixture.step_with(&amendment_run, &amendment_bundle);
    assert_eq!(moved.code, 0, "WRAV-006: the old authority still governs");

    // The proposed replacement authority signs the next transition's intents
    // before any amendment lands.
    let new_authority_review = {
        let intent = amendment_fixture.intent(&amendment_run, "step", &[], "w6-new-authority");
        amendment_fixture
            .keys
            .sign("newlead", REVIEW_NAMESPACE, &intent)
    };
    let new_authority_approval = {
        let args = [
            "--review".to_owned(),
            format!("newlead:{}", shown(&new_authority_review)),
        ];
        let refs: Vec<&str> = args.iter().map(|value| value.as_str()).collect();
        let approval_intent =
            amendment_fixture.intent(&amendment_run, "step", &refs, "w6-new-approval");
        amendment_fixture
            .keys
            .sign("newlead", APPROVE_NAMESPACE, &approval_intent)
    };

    // Amend the declared authority: the approver becomes newlead.
    let pinned = sha256_hex(&amendment_fixture.runbook_bytes);
    let amendment = String::from_utf8_lossy(&amendment_fixture.runbook_bytes)
        .replace(
            &format!("key = \"{}\"", amendment_fixture.keys.public("lead")),
            &format!("key = \"{}\"", amendment_fixture.keys.public("newlead")),
        )
        .replace("approver = \"lead\"", "approver = \"newlead\"")
        .replace("label = \"lead\"", "label = \"newlead\"");
    fs::write(
        amendment_fixture.root().join(".ratmac/ratmac.toml"),
        &amendment,
    )
    .expect("write the amended runbook");
    let before = amendment_fixture.snapshot();
    let archives_before = amendment_fixture.authorizations(&amendment_run);
    let amendment_ran = amendment_fixture.step_with(
        &amendment_run,
        &Bundle {
            reviews: vec![("newlead".to_owned(), new_authority_review)],
            approve: new_authority_approval,
        },
    );
    assert_eq!(
        amendment_ran.code, 1,
        "WRAV-006: an amendment signed by its proposed replacement authorizes nothing"
    );
    assert_eq!(
        amendment_ran.message,
        pin_mismatch(
            "step",
            &amendment_run,
            &sha256_hex(amendment.as_bytes()),
            &pinned
        ),
        "WRAV-006: the amendment drifts the pin the Run was admitted under"
    );
    assert_eq!(
        amendment_fixture.snapshot(),
        before,
        "WRAV-006: the refused amendment writes nothing"
    );
    assert_eq!(
        amendment_fixture.authorizations(&amendment_run),
        archives_before,
        "WRAV-006: the accepted history under the old authority is untouched"
    );
}
