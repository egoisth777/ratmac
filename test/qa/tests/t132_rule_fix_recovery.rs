//! t-132 / WRS-004: rule fix recovery - the public suite.
//!
//! WRSV-004-01 `wrsv_004_01` judges the vertical: an aged-record rule
//! refusal reproduces, an approved amendment repairs the rule, and the same
//! Run progresses from its retained State through fresh checks.
//! WRSV-004-02 `wrsv_004_02` judges the refusal twins against a valid
//! positive case: wrong confirmation, absent authorization, changed class,
//! missing State, terminal Run, and corrupt prior evidence.
//! WRSV-004-03 `wrsv_004_03` judges interruption before the amendment
//! rename, after it, and during the history append, plus the reader
//! reports and the exactly-once reconciliation.
//! WRSV-004-04 `wrsv_004_04` judges stale receipts after a changed rule,
//! forged approval prose, replayed verdicts, and the child's retained
//! parent ledger ownership.
//! WRSV-004-05 `wrsv_004_05` judges the authorization boundary: a
//! self-authorizing candidate refuses, the independently signed exact
//! amendment under the prior policy succeeds, and omitted changes,
//! substituted identities, and a stale chain tip refuse.
//! WRSV-004-06 `wrsv_004_06` judges the legacy Run: no enrolled recovery
//! trust refuses despite builder-writable manifests and generated
//! signatures, and independent external enrollment supplies the
//! prerequisite without recovery ever enrolling its own approver.
//! WRSV-004-07 `wrsv_004_07` judges what a fully authorized amendment
//! preserves: blocked status, hold history, original pins, no consumed
//! verdict, no laundered receipts, and the separately required resume and
//! transition authorization.
//! WRSV-004-08 `wrsv_004_08` judges chain validation: a two-entry chain
//! parses, each malformed member refuses, a preparation file is ignored,
//! and successful motion uses only the latest verified head.
//! WRSV-004-09 `wrsv_004_09` judges head binding: source bytes stay
//! identical while the authority moves, receipts and approvals bound to
//! the previous effective head refuse, fresh proof bound to the new head
//! passes, and the older evidence keeps its bytes.
//!
//! Every fixture is its own temporary Git repository; every signing key is
//! generated with the host's real `ssh-keygen`; every Engine invocation
//! goes through the in-process library entry so the t-129 trust injection
//! participates where a fixture is governed. Nothing here drives a
//! `recover` module directly: the Engine is reached only through
//! `ratmac::cli::run_from`, the built binary, and files on disk.

use ratmac::cli;
use ratmac::review::test::{install, TestTrust, TrustGuard};
use ratmac_qa::support::{self, TempTree};
use ratmac_qa::tempgit::TempRepo;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;

/// The host verifier this suite pins, per the accepted design.
const KEYGEN: &str = "C:/WINDOWS/System32/OpenSSH/ssh-keygen.exe";
const REVIEW_NAMESPACE: &str = "ratmac-review-v1";
const APPROVE_NAMESPACE: &str = "ratmac-approve-v1";
const RESUME_NAMESPACE: &str = "ratmac-resume-v1";
const RECOVER_NAMESPACE: &str = "ratmac-recover-v1";

/// The blocker reference beneath the declared `work` root.
const BLOCKER: &str = "work/blocker.txt";

/// The green output a completion receipt records.
const GREEN: &str = "test result: ok. 1 passed; 0 failed\n";

// --- shared plumbing ---------------------------------------------------------

/// One in-process Engine invocation: its exit code, its stdout bytes, and
/// its refusal message (empty on success).
struct Ran {
    code: i32,
    stdout: String,
    message: String,
}

impl std::fmt::Display for Ran {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "exit {} stdout {:?} message {:?}",
            self.code, self.stdout, self.message
        )
    }
}

/// Run one `rtm` command line in `root` through the library entry, so an
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

/// The authority digest of one member's authority table, recomputed from
/// the ticket's own bytes: `ratmac-authority-v1`, one NUL, the six
/// identities length-framed in the fixed order engine, runbook, goal,
/// gate-artifact, policy, enrollment; gate-artifact as its framed decimal
/// count then each sorted `program=sha256` pair framed one by one; a
/// historically absent identity - gate-artifact included - as the framed
/// literal bytes `absent`.
fn ticket_authority_digest(table: &toml::Value) -> String {
    let mut body = Vec::new();
    body.extend_from_slice(b"ratmac-authority-v1");
    body.push(0);
    for field in [
        "engine",
        "runbook",
        "goal",
        "gate-artifact",
        "policy",
        "enrollment",
    ] {
        let value = table
            .get(field)
            .and_then(toml::Value::as_str)
            .unwrap_or("absent");
        if field == "gate-artifact" && value != "absent" {
            let mut pairs: Vec<&str> = value.split(';').collect();
            pairs.sort_by_key(|pair| &pair[..pair.find('=').unwrap_or(pair.len())]);
            frame(&mut body, pairs.len().to_string().as_bytes());
            for pair in pairs {
                frame(&mut body, pair.as_bytes());
            }
        } else {
            frame(&mut body, value.as_bytes());
        }
    }
    sha256_hex(&body)
}

/// The change-set digest of one member's enumerated changes, recomputed
/// from the ticket's own bytes: `ratmac-change-set-v1`, one NUL, the
/// framed count and framed sorted changed field names, then the framed
/// old- and new-authority digests.
fn ticket_change_set_digest(changes: &[&str], old: &str, new: &str) -> String {
    let mut body = Vec::new();
    body.extend_from_slice(b"ratmac-change-set-v1");
    body.push(0);
    frame(&mut body, changes.len().to_string().as_bytes());
    for change in changes {
        frame(&mut body, change.as_bytes());
    }
    frame(&mut body, old.as_bytes());
    frame(&mut body, new.as_bytes());
    sha256_hex(&body)
}

/// Decode a member's signed-intent spelling and split its canonical
/// frames: the `ratmac-authorization-v1` prefix, one NUL, then every
/// length-framed field.
fn signed_intent_frames(spelled: &str) -> Vec<Vec<u8>> {
    let mut decoded = Vec::with_capacity(spelled.len() / 2);
    for pair in spelled.as_bytes().chunks(2) {
        let pair = std::str::from_utf8(pair).expect("hex pairs are ASCII");
        decoded.push(u8::from_str_radix(pair, 16).expect("lowercase hex"));
    }
    let prefix = b"ratmac-authorization-v1";
    assert!(
        decoded.starts_with(prefix) && decoded[prefix.len()] == 0,
        "the signed intent carries the canonical prefix"
    );
    let mut fields = Vec::new();
    let mut cursor = prefix.len() + 1;
    while cursor < decoded.len() {
        let length = u64::from_be_bytes(decoded[cursor..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8;
        fields.push(decoded[cursor..cursor + length].to_vec());
        cursor += length;
    }
    fields
}

/// The declared policy digest exactly as t-129 frames it.
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

/// The primary checkout Git itself reports for `root`.
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
    text.split("\n\n")
        .find(|record| !record.lines().any(|line| line == "bare"))
        .and_then(|record| {
            record
                .lines()
                .find_map(|line| line.strip_prefix("worktree "))
        })
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("the fixture repository reports a primary worktree: {text}"))
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

// --- fixture identities ------------------------------------------------------

/// Fixture identities generated at test time with the host's real tool. The
/// key tree is shared, so one bench can serve several fixtures of a test.
#[derive(Clone)]
struct KeySet {
    dir: Arc<TempTree>,
    publics: BTreeMap<String, String>,
}

impl KeySet {
    fn new(label: &str, names: &[&str]) -> Self {
        let dir = TempTree::new(&format!("{label}-keys")).expect("create the fixture key tree");
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
                "generate the fixture key {name}: {}",
                support::text(&generated)
            );
            let public = fs::read_to_string(dir.join(format!("{name}.pub")))
                .unwrap_or_else(|error| panic!("read {name}.pub: {error}"));
            publics.insert((*name).to_owned(), public.trim().to_owned());
        }
        Self {
            dir: Arc::new(dir),
            publics,
        }
    }

    fn public(&self, name: &str) -> &str {
        self.publics
            .get(name)
            .unwrap_or_else(|| panic!("the fixture key {name} exists"))
    }

    /// One detached signature over `payload`'s bytes under `namespace`, on a
    /// fresh per-signer copy so the host tool never meets an existing `.sig`
    /// it refuses to overwrite.
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
            "each signature needs a fresh payload copy: {}",
            shown(&copy)
        );
        fs::copy(payload, &copy)
            .unwrap_or_else(|error| panic!("copy the payload for {name}: {error}"));
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

/// One review bench: the designated Approver and the enrolled reviewers.
struct Bench {
    approver: &'static str,
    reviewers: &'static [&'static str],
}

/// The bench every fixture enrolls before any recovery is attempted.
const ENROLLED: Bench = Bench {
    approver: "lead",
    reviewers: &["a", "b", "lead"],
};

/// The bench a self-authorizing candidate declares: its own Approver is the
/// outsider key that exists only in the proposed policy (never under the
/// enrolled bench), while staying a well-formed policy an admit accepts.
const CANDIDATE: Bench = Bench {
    approver: "outsider",
    reviewers: &["a", "b", "outsider"],
};

/// Two further benches whose policy digests differ from the enrolled line,
/// so one Run can adopt distinct authority changes.
const REPLACEMENT_A: Bench = Bench {
    approver: "outsider",
    reviewers: &["a", "b", "outsider"],
};
const REPLACEMENT_B: Bench = Bench {
    approver: "a",
    reviewers: &["a", "b", "lead"],
};

/// One enrollment manifest over `bench`, its bytes, its path, and its
/// facts, exactly as the landed boundary reads them.
struct Manifest {
    bytes: Vec<u8>,
    path: PathBuf,
    signers_path: PathBuf,
    policy_sha256: String,
    enrollment_sha256: String,
}

/// Build one enrollment manifest for `bench` inside the key tree, bound to
/// the fixture at `scope`.
fn build_manifest(keys: &KeySet, bench: &Bench, scope: &str) -> Manifest {
    let reviewers: Vec<(String, String)> = bench
        .reviewers
        .iter()
        .map(|label| ((*label).to_owned(), keys.public(label).to_owned()))
        .collect();
    let verifier_digest = sha256_hex(&fs::read(KEYGEN).expect("read the pinned verifier"));
    let mut signers = String::new();
    for (label, public) in &reviewers {
        let parts: Vec<&str> = public.split_whitespace().collect();
        assert!(
            parts.len() >= 2 && parts[0] == "ssh-ed25519",
            "the fixture public key for {label} is one OpenSSH line"
        );
        signers.push_str(&format!("{} {} {}\n", label, parts[0], parts[1]));
    }
    keys.dir.write("allowed_signers", &signers);
    let signers_path = keys.dir.join("allowed_signers");
    let policy_sha256 = policy_digest(bench.approver, 2, false, &reviewers);
    let mut manifest = String::new();
    manifest.push_str(&format!("project-scope = \"{scope}\"\n"));
    manifest.push_str(&format!("policy-digest = \"{policy_sha256}\"\n"));
    manifest.push_str(&format!("approver = \"{}\"\n", bench.approver));
    manifest.push_str("minimum-reviews = 2\n");
    manifest.push_str("approver-may-review = false\n");
    manifest.push_str(&format!("verifier-path = \"{KEYGEN}\"\n"));
    manifest.push_str(&format!("verifier-digest = \"{verifier_digest}\"\n"));
    manifest.push_str(&format!("signers-path = \"{}\"\n", shown(&signers_path)));
    manifest.push_str(&format!(
        "signers-digest = \"{}\"\n",
        sha256_hex(signers.as_bytes())
    ));
    manifest.push_str("[builder]\n");
    manifest.push_str(&format!("actor = {:?}\n", builder_actor()));
    manifest.push_str("keys = []\n");
    for (label, public) in &reviewers {
        manifest.push_str("\n[[reviewers]]\n");
        manifest.push_str(&format!("label = \"{label}\"\n"));
        manifest.push_str(&format!("key = \"{public}\"\n"));
    }
    let bytes = manifest.into_bytes();
    keys.dir.write("enrollment-manifest.toml", &bytes);
    Manifest {
        enrollment_sha256: sha256_hex(&bytes),
        path: keys.dir.join("enrollment-manifest.toml"),
        signers_path,
        policy_sha256,
        bytes,
    }
}

// --- the fixture runbook -----------------------------------------------------

/// One machine body: intake -> build -> done with blocked self-routes for
/// the human-confirmed pause, an optional completion gate on `build`, an
/// optional spawned child class, an optional goal root, and the working
/// State spelled `build` unless another name is given.
fn machine_body(gate: Option<&str>, item: bool, goal: bool, build: &str) -> String {
    let mut body = String::new();
    body.push_str("[roots]\nissue = \"work\"\nticket = \"tickets\"\n");
    if goal {
        body.push_str("goal = \"goal\"\n");
    }
    body.push_str("\n[states.intake]\nprompt = \"Intake the request.\"\n");
    body.push_str(&format!(
        "\n[states.{build}]\nprompt = \"Build the change.\"\n"
    ));
    if let Some(ticket) = gate {
        body.push_str(&format!(
            "guards = [{{ kind = \"completion_gate\", ticket = \"{ticket}\", root = \"ticket\", \
             declaration-format = \"front-matter-string-lists\", focused-field = \
             \"focused-tests\", hidden-lane-field = \"hidden-lanes\", quality-field = \
             \"quality-commands\" }}]\n"
        ));
    }
    if item {
        body.push_str(&format!(
            "\n[[states.{build}.spawns]]\nclass = \"item\"\nname = \"unit\"\nbind = [\"ref\"]\n"
        ));
    }
    body.push_str("\n[states.done]\nprompt = \"Ship the change.\"\n");
    body.push_str(&format!(
        "\n[[transitions]]\nfrom = \"intake\"\nto = \"{build}\"\n\n[[transitions]]\nfrom = \
         \"{build}\"\nto = \"done\"\n\n[[transitions]]\nfrom = \"intake\"\nto = \
         \"intake\"\nblocked-route = true\n\n[[transitions]]\nfrom = \"{build}\"\nto = \
         \"{build}\"\nblocked-route = true\n"
    ));
    if item {
        body.push_str(
            "\n[classes.item.bindings.ref]\nrequired = true\n\n[classes.item.states.make]\n\
             prompt = \"Make the unit.\"\n\n[classes.item.states.made]\nprompt = \"The unit is \
             made.\"\n\n[[classes.item.transitions]]\nfrom = \"make\"\nto = \"made\"\n\n\
             [[classes.item.transitions]]\nfrom = \"make\"\nto = \"make\"\nblocked-route = \
             true\n",
        );
    }
    body
}

/// The aged-record rule correction: byte-level rule repair with no
/// structural change.
fn correction(body: &str) -> String {
    format!("{body}# corrected by the archived-record fix: the aged rule was wrong.\n")
}

/// One work item the completion gate reads, with the declaration lists the
/// gate derives its checks from.
fn ticket_file(id: &str, focused: &str, hidden: &str) -> String {
    format!(
        "---\nticket-id: \"{id}\"\nplanned-test-refs:\n  - \"{focused}\"\nfocused-tests:\n  - \
         \"{focused}\"\nhidden-lanes:\n  - \"{hidden}\"\nquality-commands:\n  - \"cargo \
         --version\"\nstatus: \"approved\"\n---\n\n# Ticket {id}\n\n## Vertical Outcome\n\n\
         Outcome.\n"
    )
}

// --- the project fixture -----------------------------------------------------

/// A temporary Git repository carrying one machine body and the artifacts
/// its guards read.
struct Project {
    repo: TempRepo,
}

impl Project {
    fn new(label: &str, body: &str) -> Self {
        let repo = TempRepo::new(label);
        repo.write("work/content.txt", "Contributor bytes.\n");
        repo.write(BLOCKER, "Opaque blocker; its content is not a gate.\n");
        repo.write(
            "tickets/t-100.md",
            &ticket_file("t-100", "PT-100-01", "HT-100-01"),
        );
        repo.write(
            "tickets/t-101.md",
            &ticket_file("t-101", "PT-101-01", "HT-101-01"),
        );
        repo.write("goal/spec.md", "# Goal\n\nThe frozen goal baseline.\n");
        repo.write(".ratmac/ratmac.toml", body);
        repo.commit_all("fixture baseline");
        Self { repo }
    }

    fn root(&self) -> &Path {
        self.repo.root()
    }

    fn commit_all(&self, message: &str) {
        self.repo.commit_all(message);
    }

    fn write(&self, relative: &str, body: &str) {
        self.repo.write(relative, body);
    }

    fn runbook_text(&self) -> String {
        fs::read_to_string(self.root().join(".ratmac/ratmac.toml"))
            .unwrap_or_else(|error| panic!("read the fixture runbook: {error}"))
    }

    fn rewrite_runbook(&self, body: &str) {
        self.repo.write(".ratmac/ratmac.toml", body);
    }

    fn run_dir(&self, run: &str) -> PathBuf {
        self.root().join(format!(".ratmac/runs/{run}"))
    }

    fn record_text(&self, run: &str) -> String {
        fs::read_to_string(self.run_dir(run).join("run.toml"))
            .unwrap_or_else(|error| panic!("read the Run Record of {run}: {error}"))
    }

    fn record_value(&self, run: &str, key: &str) -> String {
        let parsed: toml::Value = self
            .record_text(run)
            .parse()
            .expect("the Run Record is valid TOML");
        match &parsed[key] {
            toml::Value::String(text) => text.clone(),
            other => other.to_string(),
        }
    }

    fn state(&self, run: &str) -> String {
        self.record_value(run, "state")
    }

    fn status(&self, run: &str) -> String {
        self.record_value(run, "status")
    }

    /// The Run Record carries exactly the public seven fields.
    fn assert_seven_fields(&self, run: &str) {
        let parsed: toml::Value = self.record_text(run).parse().expect("valid Run Record");
        let mut keys: Vec<&str> = parsed
            .as_table()
            .expect("the Run Record is a table")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "active_refs",
                "blocker",
                "goal_revision",
                "input_revision",
                "output_revision",
                "state",
                "status",
            ],
            "WRS-004: the public Run Record of {run} keeps its seven fields"
        );
    }

    fn evidence_bytes(&self, run: &str) -> Vec<u8> {
        fs::read(self.run_dir(run).join("evidence.toml"))
            .unwrap_or_else(|error| panic!("read the evidence of {run}: {error}"))
    }

    /// The runbook pin recorded at Run start.
    fn runbook_pin(&self, run: &str) -> String {
        let parsed: toml::Value = String::from_utf8_lossy(&self.evidence_bytes(run))
            .parse()
            .expect("the evidence is valid TOML");
        parsed["runbook"]["sha256"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    }

    fn history(&self) -> String {
        fs::read_to_string(self.root().join(".ratmac/log.md"))
            .unwrap_or_else(|error| panic!("read the history log: {error}"))
    }

    fn start(&self) -> String {
        let started = rtm(self.root(), &["start"]);
        assert_eq!(
            started.code, 0,
            "WRS-004: admission must succeed: {}{}",
            started.stdout, started.message
        );
        started
            .stdout
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("rtm: started run "))
            .and_then(|rest| rest.split(" at ").next())
            .unwrap_or_default()
            .to_owned()
    }

    fn spawn_child(&self, parent: &str, bind: &str) -> String {
        let spawned = rtm(
            self.root(),
            &["spawn", "unit", "--run", parent, "--bind", bind],
        );
        assert_eq!(
            spawned.code, 0,
            "WRS-004: spawn must succeed: {}{}",
            spawned.stdout, spawned.message
        );
        spawned
            .stdout
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("rtm: spawned run "))
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_default()
            .to_owned()
    }

    fn amendments_dir(&self, run: &str) -> PathBuf {
        self.run_dir(run).join("amendments")
    }

    fn amendment_bytes(&self, run: &str, sequence: u64) -> Option<Vec<u8>> {
        fs::read(self.amendments_dir(run).join(format!("{sequence}.toml"))).ok()
    }

    /// Every authorization record of `run`, keyed by filename.
    fn authorizations(&self, run: &str) -> BTreeMap<String, String> {
        let dir = self.run_dir(run).join("authorizations");
        let mut records = BTreeMap::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries {
                let entry = entry.expect("read an authorization entry");
                let name = entry.file_name().to_string_lossy().into_owned();
                let text = fs::read_to_string(entry.path()).expect("read an authorization record");
                records.insert(name, text);
            }
        }
        records
    }

    /// Every receipt file recorded for `run`, keyed by its filename.
    fn receipt_files(&self, run: &str) -> BTreeMap<String, Vec<u8>> {
        let dir = self.root().join(format!(".ratmac/evidence/{run}"));
        let mut result = BTreeMap::new();
        fn visit(at: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
            if let Ok(entries) = fs::read_dir(at) {
                for entry in entries {
                    let entry = entry.expect("receipt entry is readable");
                    let path = entry.path();
                    if entry.file_type().expect("receipt file kind").is_dir() {
                        visit(&path, result);
                    } else {
                        result.insert(
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            fs::read(&path).expect("receipt is readable"),
                        );
                    }
                }
            }
        }
        visit(&dir, &mut result);
        result
    }

    /// Every byte under the fixture except Git's own, keyed by displayed
    /// forward-slash path.
    fn snapshot(&self) -> BTreeMap<String, Vec<u8>> {
        let mut result = BTreeMap::new();
        fn visit(root: &Path, at: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(at).expect("snapshot directory is readable") {
                let entry = entry.expect("snapshot entry is readable");
                if at == root && entry.file_name() == ".git" {
                    continue;
                }
                let path = entry.path();
                if entry.file_type().expect("snapshot file kind").is_dir() {
                    visit(root, &path, result);
                } else {
                    result.insert(
                        shown(path.strip_prefix(root).expect("snapshot stays local")),
                        fs::read(&path).expect("snapshot file is readable"),
                    );
                }
            }
        }
        visit(self.root(), self.root(), &mut result);
        result
    }

    fn assert_unchanged(&self, before: &BTreeMap<String, Vec<u8>>, what: &str) {
        assert_eq!(
            self.snapshot(),
            *before,
            "WRS-004: the {what} wrote a byte somewhere"
        );
    }

    /// Record green completion receipts for one ticket's checks, optionally
    /// bound to an authority head.
    fn write_receipts(&self, run: &str, ticket: &str, head: Option<&str>) {
        let dir = self
            .root()
            .join(format!(".ratmac/evidence/{run}/completion"));
        fs::create_dir_all(&dir).expect("create the completion directory");
        let digest = ratmac::completion::tree_digest(self.root(), &["work".to_owned()])
            .expect("digest work");
        let (focused, hidden) = if ticket == "t-100" {
            ("PT-100-01", "HT-100-01")
        } else {
            ("PT-101-01", "HT-101-01")
        };
        for (check, kind) in [
            (focused, "focused"),
            (hidden, "hidden-lane"),
            ("cargo --version", "quality"),
        ] {
            let command = if kind == "quality" {
                "cargo --version"
            } else {
                "cargo test --test fixture_test"
            };
            let mut body = format!(
                "ticket-id = \"{ticket}\"\ncheck-id = \"{check}\"\nkind = \"{kind}\"\ncommand = \
                 \"{command}\"\nworking-dir = \".\"\nexit-status = 0\noutput-sha256 = \
                 \"{}\"\ntree-roots = [\"work\"]\ntree-sha256 = \"{digest}\"\n",
                ratmac::receipt::sha256_text(GREEN)
            );
            if let Some(head) = head {
                body.push_str(&format!("authority-head-sha256 = \"{head}\"\n"));
            }
            body.push_str(&format!("output = \"\"\"\n{GREEN}\"\"\"\n"));
            fs::write(
                dir.join(format!("{}.toml", ratmac::completion::check_slug(check))),
                body,
            )
            .expect("write the completion receipt");
        }
    }
}

// --- the governed fixture ----------------------------------------------------

/// A project whose runbook declares a review policy over the top-level and
/// the item class, with the enrollment installed through the t-129 seam.
struct Governed {
    project: Project,
    keys: KeySet,
    _trust: TrustGuard,
    policy_sha256: String,
    enrollment_sha256: String,
}

impl Governed {
    /// One governed fixture over `body` with the enrolled bench and a fresh
    /// key set carrying a, b, lead, and an outsider.
    fn new(label: &str, body: &str, item: bool) -> Self {
        Self::with_keys(
            label,
            body,
            item,
            KeySet::new(label, &["a", "b", "lead", "outsider"]),
        )
    }

    fn with_keys(label: &str, body: &str, item: bool, keys: KeySet) -> Self {
        let project = Project::new(label, body);
        let scope = shown(
            &fs::canonicalize(resolved_primary(project.root()))
                .expect("canonicalize the fixture primary"),
        );
        let manifest = build_manifest(&keys, &ENROLLED, &scope);
        let verifier_sha256 = sha256_hex(&fs::read(KEYGEN).expect("read the verifier"));
        let trust = install(
            &project.root().join(".ratmac"),
            TestTrust {
                manifest_bytes: manifest.bytes.clone(),
                signers_path: manifest.signers_path.clone(),
                verifier_path: PathBuf::from(KEYGEN),
                verifier_sha256: verifier_sha256.clone(),
            },
        );
        let governed = Self {
            project,
            keys,
            _trust: trust,
            policy_sha256: manifest.policy_sha256,
            enrollment_sha256: manifest.enrollment_sha256,
        };
        let full = governed.runbook_with(body, &ENROLLED, item);
        governed.project.rewrite_runbook(&full);
        governed.project.commit_all("governed fixture baseline");
        governed
    }

    /// One review table for `bench`, appended to a machine body.
    fn review_table(&self, bench: &Bench, table: &str) -> String {
        let reviewers: Vec<(String, String)> = bench
            .reviewers
            .iter()
            .map(|label| ((*label).to_owned(), self.keys.public(label).to_owned()))
            .collect();
        let digest = policy_digest(bench.approver, 2, false, &reviewers);
        let verifier_digest = sha256_hex(&fs::read(KEYGEN).expect("read the verifier"));
        let manifest = self.keys.dir.join("enrollment-manifest.toml");
        let mut text = format!("\n[{table}]\n");
        text.push_str(&format!("manifest = \"{}\"\n", shown(&manifest)));
        text.push_str(&format!("policy-digest = \"{digest}\"\n"));
        text.push_str(&format!("verifier-digest = \"{verifier_digest}\"\n"));
        text.push_str(&format!(
            "signers-digest = \"{}\"\n",
            sha256_hex(&fs::read(self.keys.dir.join("allowed_signers")).expect("read signers"))
        ));
        text.push_str("minimum-reviews = 2\n");
        text.push_str(&format!("approver = \"{}\"\n", bench.approver));
        text.push_str("approver-may-review = false\n");
        text.push_str("work = [\"work\"]\n");
        for (label, public) in &reviewers {
            text.push_str(&format!("\n[[{table}.reviewers]]\n"));
            text.push_str(&format!("label = \"{label}\"\n"));
            text.push_str(&format!("key = \"{public}\"\n"));
        }
        text
    }

    /// The full runbook for `body` under `bench`, governing the top-level
    /// class and the item class when the body declares one.
    fn runbook_with(&self, body: &str, bench: &Bench, item: bool) -> String {
        let mut text = body.to_owned();
        text.push_str(&self.review_table(bench, "review"));
        if item {
            text.push_str(&self.review_table(bench, "classes.item.review"));
        }
        text
    }

    fn rewrite(&self, body: &str, bench: &Bench, item: bool) {
        let full = self.runbook_with(body, bench, item);
        self.project.rewrite_runbook(&full);
    }

    fn admit(&self) -> String {
        self.project.start()
    }

    /// Render one intent with the given purpose arguments and prove the
    /// pinned report line.
    fn intent(&self, run: &str, extra: &[&str], tag: &str) -> PathBuf {
        let out = self.keys.dir.join(format!("{tag}.bin"));
        let mut args = vec!["intent".to_owned(), "--run".to_owned(), run.to_owned()];
        args.extend(extra.iter().map(|value| (*value).to_owned()));
        args.push("--out".to_owned());
        args.push(shown(&out));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let rendered = rtm(self.project.root(), &refs);
        assert_eq!(
            rendered.code, 0,
            "WRS-004: render the {tag} intent for {run}: {}{}",
            rendered.stdout, rendered.message
        );
        let bytes = fs::read(&out).unwrap_or_else(|error| panic!("read {tag}: {error}"));
        let purpose = if extra.contains(&"--recover") {
            "recover"
        } else if extra.contains(&"--hold") {
            "hold"
        } else if extra.contains(&"--resume") {
            "resume"
        } else {
            "step"
        };
        assert_eq!(
            rendered.stdout,
            format!(
                "rtm: canonical {purpose} intent for run {run} written to {} (sha256 {})\n",
                shown(&out),
                sha256_hex(&bytes)
            ),
            "WRS-004: the {tag} intent report is the pinned one line"
        );
        out
    }

    /// The canonical recovery intent of `run`.
    fn recover_intent(&self, run: &str, enrollment: Option<&Path>, tag: &str) -> PathBuf {
        let mut extra = vec!["--recover".to_owned()];
        if let Some(path) = enrollment {
            extra.push("--enrollment".to_owned());
            extra.push(shown(path));
        }
        let refs: Vec<&str> = extra.iter().map(String::as_str).collect();
        self.intent(run, &refs, tag)
    }

    /// One decision signature by the designated Approver.
    fn decision_over(&self, intent: &Path, _tag: &str) -> PathBuf {
        self.keys.sign("lead", RECOVER_NAMESPACE, intent)
    }

    /// Run one recover invocation with the given parts; `None` omits the
    /// part.
    fn recover(
        &self,
        run: &str,
        decision: Option<&Path>,
        enrollment: Option<&Path>,
        confirm: Option<&str>,
    ) -> Ran {
        let mut args = vec!["recover".to_owned(), "--run".to_owned(), run.to_owned()];
        if let Some(path) = enrollment {
            args.push("--enrollment".to_owned());
            args.push(shown(path));
        }
        if let Some(path) = decision {
            args.push("--decision".to_owned());
            args.push(shown(path));
        }
        if let Some(phrase) = confirm {
            args.push("--confirm".to_owned());
            args.push(phrase.to_owned());
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(self.project.root(), &refs)
    }

    /// One recover invocation shaped exactly as the contract's form line.
    fn recover_ok(&self, run: &str, decision: &Path, sequence: u64, tag: &str) {
        let recovered = self.recover(run, Some(decision), None, Some(&format!("recover {run}")));
        assert_eq!(
            recovered.code, 0,
            "WRS-004: the authorized recovery of {run} ({tag}) succeeds: {recovered}"
        );
        assert_eq!(
            recovered.stdout,
            success_lines(run, sequence),
            "WRS-004: the recovery success bytes are the contract's two lines ({tag})"
        );
    }

    /// One full recovery flow: render the intent, sign it with the
    /// Approver, and recover at the expected sequence.
    fn amend(&self, run: &str, sequence: u64, tag: &str) -> PathBuf {
        let intent = self.recover_intent(run, None, &format!("{tag}-intent"));
        let decision = self.decision_over(&intent, tag);
        self.recover_ok(run, &decision, sequence, tag);
        decision
    }

    /// One fully signed authorization bundle's `--review`/`--approve`
    /// arguments over the (optionally blocked) motion intent of `run`.
    fn authorization_args(&self, run: &str, hold: Option<&str>, tag: &str) -> Vec<String> {
        let mut purpose_args: Vec<String> = Vec::new();
        if let Some(blocker) = hold {
            purpose_args.push("--hold".to_owned());
            purpose_args.push("--blocker".to_owned());
            purpose_args.push(blocker.to_owned());
        }
        let purpose_refs: Vec<&str> = purpose_args.iter().map(String::as_str).collect();
        let review_intent = self.intent(run, &purpose_refs, &format!("{tag}-review"));
        let mut offered: Vec<String> = purpose_args.clone();
        for label in ["a", "b"] {
            let signature = self.keys.sign(label, REVIEW_NAMESPACE, &review_intent);
            offered.push("--review".to_owned());
            offered.push(format!("{label}:{}", shown(&signature)));
        }
        let mut approval_args = purpose_args.clone();
        let reviews = &offered[purpose_args.len()..];
        for pair in reviews.chunks(2) {
            approval_args.push(pair[0].clone());
            approval_args.push(pair[1].clone());
        }
        let approval_refs: Vec<&str> = approval_args.iter().map(String::as_str).collect();
        let approval_intent = self.intent(run, &approval_refs, &format!("{tag}-approval"));
        let approve = self.keys.sign("lead", APPROVE_NAMESPACE, &approval_intent);
        offered.push("--approve".to_owned());
        offered.push(shown(&approve));
        offered
    }

    /// One governed ordinary step of `run`.
    fn step_governed(&self, run: &str, tag: &str) -> Ran {
        let auth = self.authorization_args(run, None, tag);
        self.step_with(run, auth)
    }

    /// One governed step carrying an already rendered authorization bundle.
    fn step_with(&self, run: &str, auth: Vec<String>) -> Ran {
        let mut args = vec!["step".to_owned(), "--run".to_owned(), run.to_owned()];
        args.extend(auth);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(self.project.root(), &refs)
    }

    /// One governed step that must succeed.
    fn step_ok(&self, run: &str, tag: &str) {
        let stepped = self.step_governed(run, tag);
        assert_eq!(
            stepped.code, 0,
            "WRS-004: the governed step of {run} ({tag}) succeeds: {}{}",
            stepped.stdout, stepped.message
        );
        assert!(
            !stepped.stdout.contains("step refused"),
            "WRS-004: the governed step of {run} ({tag}) must not refuse: {}",
            stepped.stdout
        );
    }

    /// One governed hold of `run` against the fixture blocker.
    fn hold_governed(&self, run: &str, tag: &str) -> Ran {
        let auth = self.authorization_args(run, Some(BLOCKER), tag);
        let mut args = vec![
            "hold".to_owned(),
            "--run".to_owned(),
            run.to_owned(),
            "--blocker".to_owned(),
            BLOCKER.to_owned(),
            "--confirm".to_owned(),
            format!("hold {run}"),
        ];
        args.extend(auth[3..].to_vec());
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(self.project.root(), &refs)
    }

    /// One governed resume of `run`, optionally carrying one resolution
    /// signature.
    fn resume_governed(&self, run: &str, resolution: Option<&Path>) -> Ran {
        let mut args = vec![
            "resume".to_owned(),
            "--run".to_owned(),
            run.to_owned(),
            "--blocker".to_owned(),
            BLOCKER.to_owned(),
            "--confirm".to_owned(),
            format!("resume {run}"),
        ];
        if let Some(path) = resolution {
            args.push("--resolution".to_owned());
            args.push(shown(path));
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(self.project.root(), &refs)
    }

    /// The canonical resolution intent of `run`.
    fn resume_intent(&self, run: &str, tag: &str) -> PathBuf {
        self.intent(run, &["--resume"], tag)
    }
}

/// The success lines of a recovery naming `sequence`.
fn success_lines(run: &str, sequence: u64) -> String {
    format!(
        "rtm: run {run} amended at sequence {sequence}; the corrected authority is effective\n\
         The Run keeps its address, State, class, bindings, workspace, and original pins; an \
         amendment neither resumes a paused Run nor approves a transition.\n"
    )
}

/// The strict-field key set of one amendment record, sorted.
fn amendment_keys(text: &str) -> Vec<String> {
    let parsed: toml::Value = text.parse().expect("the amendment record is TOML");
    let mut keys: Vec<String> = parsed
        .as_table()
        .expect("the amendment record is a table")
        .keys()
        .cloned()
        .collect();
    keys.sort_unstable();
    keys
}

/// Replace the first top-level `key =` line of a record with `new_line`.
fn replace_key_line(text: &str, key: &str, new_line: &str) -> String {
    let prefix = format!("{key} =");
    let mut result = String::new();
    let mut replaced = false;
    for line in text.lines() {
        if !replaced && line.starts_with(&prefix) {
            result.push_str(new_line);
            result.push('\n');
            replaced = true;
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    assert!(replaced, "the record carries a {key} field");
    result
}

/// Drop the first top-level `key =` line of a record.
fn drop_key_line(text: &str, key: &str) -> String {
    let prefix = format!("{key} =");
    let mut result = String::new();
    let mut dropped = false;
    for line in text.lines() {
        if !dropped && line.starts_with(&prefix) {
            dropped = true;
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    assert!(dropped, "the record carries a {key} field");
    result
}

/// Replace `key`'s value inside the named table of a TOML document.
fn replace_in_table(text: &str, table: &str, key: &str, value: &str) -> String {
    let header = format!("[{table}]");
    let mut result = String::new();
    let mut inside = false;
    let mut replaced = false;
    for line in text.lines() {
        if line.starts_with('[') {
            inside = line == header;
        }
        if inside && !replaced && line.starts_with(&format!("{key} =")) {
            result.push_str(&format!("{key} = \"{value}\"\n"));
            replaced = true;
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    assert!(replaced, "the {table} table carries a {key} field");
    result
}

/// The lowercase-hex test one digest carries.
fn is_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

// --- WRSV-004-01 ---------------------------------------------------------------

/// WRSV-004-01: an aged-record rule refusal reproduces, the approved amendment repairs the rule, and the same Run progresses from its retained State through fresh checks.
#[test]
fn wrsv_004_01() {
    let _lane = support::hook_lane();
    let body = machine_body(None, false, false, "build");
    let governed = Governed::new("wrsv00401", &body, false);
    let run = governed.admit();
    assert_eq!(governed.project.state(&run), "intake");

    // The fixture control: the governed step works under the enrolled
    // policy before any rule changes.
    let control = governed.step_governed(&run, "w1-control");
    assert_eq!(
        control.code, 0,
        "WRS-004: the control step succeeds: {}{}",
        control.stdout, control.message
    );
    let pin_before = governed.project.runbook_pin(&run);
    let evidence_before = governed.project.evidence_bytes(&run);
    let record_before = governed.project.record_text(&run);

    // The aged-record rule refusal: the corrected runbook drifts past the
    // recorded pin, and ordinary motion refuses with today's text. The
    // bundle is rendered while the pin still matches, then the rule moves.
    let corrected = correction(&governed.project.runbook_text());
    let aged_auth = governed.authorization_args(&run, None, "w1-aged");
    governed.project.rewrite_runbook(&corrected);
    let aged = governed.step_with(&run, aged_auth);
    assert_eq!(aged.code, 1, "WRS-004: the aged step refuses: {aged}");
    assert!(
        aged.message.contains("runbook pin mismatch"),
        "WRS-004: the aged step names the drifted pin: {aged}"
    );
    assert_eq!(governed.project.evidence_bytes(&run), evidence_before);

    // The verb joins the one public route table after resume.
    let usage = rtm(governed.project.root(), &["--help"]);
    assert_eq!(usage.code, 0);
    let listed = usage
        .stdout
        .lines()
        .find(|line| line.starts_with("Commands:"))
        .expect("the general usage carries a Commands line");
    assert_eq!(
        listed,
        // t-134 (WEB-006): `endpoint` joins the route table last.
        "Commands: init, start, status, step, hold, intent, resume, recover, abandon, spawn, \
         respawn, doctor, scaffold, skill, endpoint",
        "WRS-004: the route table lists recover after resume"
    );

    // The verb's own help prints its usage form and one paragraph.
    let help = rtm(governed.project.root(), &["recover", "--help"]);
    assert_eq!(help.code, 0, "WRS-004: recover --help exits 0: {help}");
    assert!(
        help.stdout.starts_with(
            "Usage: rtm recover --run <id> [--enrollment <path>] --decision <sig> --confirm \
             \"recover <id>\" [--help]\n\n"
        ),
        "WRS-004: recover's help prints its usage form first: {}",
        help.stdout
    );
    assert!(
        help.stdout.contains("resumes a paused Run")
            && help.stdout.contains("original pins")
            && help.stdout.ends_with('\n'),
        "WRS-004: recover's help carries its one paragraph: {}",
        help.stdout
    );

    // The repair: render the canonical recovery intent, sign it with the
    // designated Approver, and recover.
    let intent = governed.recover_intent(&run, None, "w1-recover");
    let decision = governed.decision_over(&intent, "w1");
    governed.recover_ok(&run, &decision, 1, "w1");

    // The Run keeps its address, State, class, and seven fields; the
    // original pins are byte-identical.
    assert_eq!(governed.project.state(&run), "build");
    // Landed step never writes `executing` (only resume does), so the Run
    // stays `planned`; recovery changes no Run Record byte.
    assert_eq!(governed.project.status(&run), "planned");
    governed.project.assert_seven_fields(&run);
    assert_eq!(
        governed.project.evidence_bytes(&run),
        evidence_before,
        "WRS-004: the recovery keeps the original pins byte-identical"
    );
    assert_eq!(
        governed.project.record_text(&run),
        record_before,
        "WRS-004: the recovery changes no Run Record byte"
    );

    // The amendment record: one strict version-1 member at sequence 1.
    let amendment = governed
        .project
        .amendment_bytes(&run, 1)
        .expect("the recovery publishes amendment 1");
    let text = String::from_utf8_lossy(&amendment).into_owned();
    assert_eq!(
        amendment_keys(&text),
        [
            "affected-proof-boundary",
            "changes",
            "context",
            "new-authorities",
            "old-authorities",
            "previous-amendment-digest",
            "run",
            "sequence",
            "signature-identities",
            "signed-intent",
            "version",
        ]
        .iter()
        .map(|key| (*key).to_owned())
        .collect::<Vec<_>>(),
        "WRS-004: the amendment record holds exactly the contract's fields"
    );
    let parsed: toml::Value = text.parse().expect("the amendment record is TOML");
    assert_eq!(parsed["version"].as_integer(), Some(1));
    assert_eq!(parsed["sequence"].as_integer(), Some(1));
    assert_eq!(parsed["run"].as_str(), Some(run.as_str()));
    assert!(
        is_digest(
            parsed["previous-amendment-digest"]
                .as_str()
                .unwrap_or_default()
        ),
        "WRS-004: the first member binds the pin-record digest"
    );
    let context = parsed["context"].as_table().expect("the context table");
    let mut context_keys: Vec<&str> = context.keys().map(String::as_str).collect();
    context_keys.sort_unstable();
    assert_eq!(
        context_keys,
        vec!["bindings", "class", "state", "workspace"]
    );
    assert_eq!(context["state"].as_str(), Some("build"));
    assert_eq!(context["class"].as_str(), Some("runbook"));
    assert_eq!(context["bindings"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        context["workspace"].as_str(),
        Some(shown(&fs::canonicalize(governed.project.root()).unwrap()).as_str()),
        "WRS-004: the context's workspace is canonical, forward slashes"
    );
    let changes: Vec<&str> = parsed["changes"]
        .as_array()
        .expect("changes is an array")
        .iter()
        .map(|value| value.as_str().unwrap_or_default())
        .collect();
    assert_eq!(changes, vec!["runbook"], "the rule fix changes one field");
    for table in ["old-authorities", "new-authorities"] {
        let mut keys: Vec<&str> = parsed[table]
            .as_table()
            .expect("the authority table")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "engine",
                "enrollment",
                "gate-artifact",
                "goal",
                "policy",
                "runbook",
            ],
            "WRS-004: the {table} table holds the complete identities"
        );
    }
    assert_eq!(
        parsed["old-authorities"]["runbook"].as_str(),
        Some(pin_before.as_str()),
        "the old runbook identity is the recorded pin"
    );
    assert_eq!(
        parsed["new-authorities"]["runbook"].as_str(),
        Some(sha256_hex(corrected.as_bytes()).as_str()),
        "the new runbook identity is the corrected bytes"
    );
    assert_eq!(
        parsed["old-authorities"]["policy"].as_str(),
        Some(governed.policy_sha256.as_str()),
        "the rule fix keeps the policy identity"
    );
    assert_eq!(
        parsed["new-authorities"]["policy"].as_str(),
        Some(governed.policy_sha256.as_str())
    );
    assert_eq!(
        parsed["old-authorities"]["enrollment"].as_str(),
        Some(governed.enrollment_sha256.as_str())
    );
    assert_eq!(
        parsed["new-authorities"]["enrollment"].as_str(),
        Some(governed.enrollment_sha256.as_str())
    );
    assert_eq!(
        parsed["old-authorities"]["goal"].as_str(),
        Some("absent"),
        "a historically absent authority is the literal absent"
    );
    assert_eq!(parsed["new-authorities"]["goal"].as_str(), Some("absent"));
    let intent_hex: String = fs::read(&intent)
        .unwrap()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        parsed["signed-intent"].as_str(),
        Some(intent_hex.as_str()),
        "the signed-intent slot holds the exact canonical bytes as lowercase hex"
    );
    let signatures = parsed["signature-identities"]
        .as_array()
        .expect("signature-identities is an array");
    assert_eq!(signatures.len(), 1, "the Approver is the one signer");
    assert_eq!(signatures[0]["label"].as_str(), Some("lead"));
    assert_eq!(
        signatures[0]["key"].as_str(),
        Some(governed.keys.public("lead")),
        "the signer row carries the verifying enrolled key"
    );
    assert_eq!(
        signatures[0]["signature-sha256"].as_str(),
        Some(sha256_hex(&fs::read(&decision).unwrap()).as_str()),
        "the signer row carries the decision signature's digest"
    );
    assert!(
        governed.project.amendment_bytes(&run, 2).is_none(),
        "one recovery mints exactly one member"
    );
    assert!(
        !governed
            .project
            .amendments_dir(&run)
            .join("1.prepare")
            .is_file(),
        "the committed recovery leaves no preparation file"
    );

    // The durable history event, exactly once.
    assert_eq!(
        governed
            .project
            .history()
            .matches(&format!(
                "- Recover: run {run} adopted amendment 1 on an explicit signed recovery \
                 decision.\n"
            ))
            .count(),
        1,
        "WRS-004: the recovery appends its uniquely identified event"
    );

    // The authorization archive gains exactly one strict record.
    let records = governed.project.authorizations(&run);
    assert_eq!(
        records.len(),
        2,
        "the step and the recovery archive one record each"
    );
    let recover_record = records
        .values()
        .find(|text| text.contains("ratmac-recover-v1"))
        .expect("the recovery archive record exists");
    let archive: toml::Value = recover_record.parse().expect("the archive record is TOML");
    assert_eq!(archive["purpose"].as_str(), Some("ratmac-recover-v1"));
    assert_eq!(archive["amendment_sequence"].as_integer(), Some(1));
    assert_eq!(archive["from_state"].as_str(), Some("build"));
    assert_eq!(archive["to_state"].as_str(), Some("build"));
    assert_eq!(
        archive["decision_signature_sha256"].as_str(),
        Some(sha256_hex(&fs::read(&decision).unwrap()).as_str())
    );
    assert_eq!(
        archive["intent_sha256"].as_str(),
        Some(sha256_hex(&fs::read(&intent).unwrap()).as_str())
    );
    assert_eq!(archive["origin"].as_str(), Some("test-injected"));
    // "Exactly" those ten fields, and zero [[reviews]] rows in TOML is no
    // `reviews` key at all - not an empty array a stray writer could add.
    let mut archive_keys: Vec<&str> = archive
        .as_table()
        .expect("the archive record is a table")
        .keys()
        .map(String::as_str)
        .collect();
    archive_keys.sort_unstable();
    assert_eq!(
        archive_keys,
        [
            "amendment_sequence",
            "decision_signature_sha256",
            "enrollment_sha256",
            "from_state",
            "intent_sha256",
            "origin",
            "policy_sha256",
            "purpose",
            "sequence",
            "to_state",
        ],
        "WRS-004: the archive record holds exactly the ten fields with zero review rows"
    );

    // The same Run progresses from its retained State using fresh checks
    // under the corrected authority.
    governed.step_ok(&run, "w1-fresh-build");
    assert_eq!(governed.project.state(&run), "done");
    assert_eq!(governed.project.status(&run), "passed");
    assert_eq!(
        governed.project.evidence_bytes(&run),
        evidence_before,
        "WRS-004: motion under the corrected authority still rewrites no pin"
    );

    // A gate-artifact amendment: the Run is stopped by a failing guard
    // whose program bytes are then corrected at the same path. The
    // recovery adopts only the gate-artifact change, and the next step
    // evaluates the corrected guard and progresses.
    {
        // The guard program lives outside the project tree under a plain
        // absolute path: the Engine evaluates command_exit guards from the
        // canonical workspace, and a script resolved through a verbatim
        // prefix cannot be spawned by the interpreter.
        let probe_tree = TempTree::new("wrsv00401g-probe").expect("create the guard program tree");
        let probe = probe_tree.join("probe.cmd");
        fs::write(&probe, b"@echo off\r\nexit /b 3\r\n").expect("write the failing guard");
        // A second guard program whose name extends the first: the shorter
        // program name is a prefix of the longer and the longer's next
        // byte (the dot before its cmd suffix) sorts below the pair
        // separator, so program-name order and whole-pair-text order
        // disagree - the digest framing cannot hide behind one spelling.
        let second = probe_tree.join("probe.cmd.cmd");
        fs::write(&second, b"@echo off\r\nexit /b 0\r\n").expect("write the passing guard");
        let escape = |path: &str| path.replace('\\', "\\\\");
        let mut guarded = machine_body(None, false, false, "build");
        let guard_row = format!(
            "guards = [\n  {{ kind = \"command_exit\", program = \"{}\", expected = 0 }},\n  {{ \
             kind = \"command_exit\", program = \"{}\", expected = 0 }},\n]\n",
            escape(&probe.to_string_lossy()),
            escape(&second.to_string_lossy())
        );
        guarded = guarded.replacen(
            "[states.build]\n",
            &format!("[states.build]\n{guard_row}"),
            1,
        );
        let governed = Governed::new("wrsv00401g", &guarded, false);
        let run = governed.admit();
        governed.step_ok(&run, "w1g-enter-build");
        assert_eq!(governed.project.state(&run), "build");
        let blocked = governed.step_governed(&run, "w1g-blocked");
        assert_eq!(blocked.code, 0, "WRS-004: {blocked}");
        assert!(
            blocked.stdout.contains("step refused")
                && blocked.stdout.contains("command_exit")
                && blocked.stdout.contains("observed exit 3"),
            "WRS-004: the drifted guard program refuses before recovery: {blocked}"
        );
        assert_eq!(governed.project.state(&run), "build");
        fs::write(&probe, b"@echo off\r\nexit /b 0\r\n").expect("correct the guard program");
        governed.amend(&run, 1, "w1g-gate");
        let member: toml::Value =
            String::from_utf8_lossy(&governed.project.amendment_bytes(&run, 1).unwrap())
                .parse()
                .expect("the gate amendment is TOML");
        assert_eq!(
            member["changes"].as_array().map(|rows| rows.len()),
            Some(1),
            "WRS-004: the change set is only the gate artifact: {:?}",
            member["changes"]
        );
        assert_eq!(member["changes"][0].as_str(), Some("gate-artifact"));
        // The gate encoding itself is observable: recomputed from the
        // ticket's own bytes over the record's gate tables - the present
        // corrected pair framed as its count and one framed pair, the
        // historical side as it is spelled - the digests equal the frames
        // the signed intent carries.
        let old_digest = ticket_authority_digest(&member["old-authorities"]);
        let new_digest = ticket_authority_digest(&member["new-authorities"]);
        let change_set = ticket_change_set_digest(&["gate-artifact"], &old_digest, &new_digest);
        let fields = signed_intent_frames(member["signed-intent"].as_str().expect("signed-intent"));
        assert_eq!(fields.len(), 24, "the canonical intent frames 24 fields");
        assert_eq!(
            fields[21],
            new_digest.as_bytes(),
            "WRS-004: the intent binds the ticket-recomputed gate-carrying authority digest"
        );
        assert_eq!(
            fields[22],
            change_set.as_bytes(),
            "WRS-004: the intent binds the ticket-recomputed gate change-set digest"
        );
        governed.step_ok(&run, "w1g-fresh");
        assert_eq!(governed.project.state(&run), "done");
    }
}

// --- WRSV-004-02 ---------------------------------------------------------------

/// WRSV-004-02: wrong confirmation, absent authorization, changed class, missing State, terminal Run, corrupt prior evidence, and a mismatched workspace each refuse unchanged after a valid positive case.
#[test]
fn wrsv_004_02() {
    let _lane = support::hook_lane();
    let body = machine_body(None, true, false, "build");
    let governed = Governed::new("wrsv00402", &body, true);
    let run = governed.admit();

    // The valid positive case first: one authorized repair succeeds.
    let corrected = correction(&governed.project.runbook_text());
    governed.project.rewrite_runbook(&corrected);
    let intent = governed.recover_intent(&run, None, "w2-positive");
    let decision = governed.decision_over(&intent, "w2");
    governed.recover_ok(&run, &decision, 1, "w2-positive");
    governed.step_ok(&run, "w2-drive");
    assert_eq!(governed.project.state(&run), "build");
    let before = governed.project.snapshot();

    // A missing --run addresses nothing and prints the roster.
    let missing = rtm(
        governed.project.root(),
        &["recover", "--confirm", &format!("recover {run}")],
    );
    assert_eq!(missing.code, 1, "WRS-004: {missing}");
    assert!(
        missing
            .message
            .starts_with("recover refused; recover requires --run <id>; runs: "),
        "WRS-004: the missing --run refusal carries the roster line: {missing}"
    );

    // An unknown id is not an exact roster member.
    let unknown = governed.recover("ghost-run-900", None, None, Some("recover run-001"));
    assert_eq!(unknown.code, 1, "WRS-004: {unknown}");
    assert!(
        unknown.message.starts_with(
            "recover refused; run id \"ghost-run-900\" is not an exact roster member; runs: ",
        ),
        "WRS-004: the unknown id refusal carries the roster line: {unknown}"
    );

    // A --run with no value is a usage refusal.
    let bare = rtm(governed.project.root(), &["recover", "--run"]);
    assert_eq!(bare.code, 2, "WRS-004: {bare}");
    assert!(
        bare.message.starts_with("recover: --run needs a run id; "),
        "WRS-004: the bare --run refusal mirrors hold's shape: {bare}"
    );

    // The typed phrase: missing, then wrong.
    let unconfirmed = governed.recover(&run, Some(&decision), None, None);
    assert_eq!(unconfirmed.code, 1, "WRS-004: {unconfirmed}");
    assert_eq!(
        unconfirmed.message,
        format!(
            "recover refused; recover is unconfirmed: a human must type --confirm {:?}",
            format!("recover {run}")
        ),
        "WRS-004: the unconfirmed refusal mirrors hold's family byte-for-byte"
    );
    let wrong_phrase = governed.recover(&run, Some(&decision), None, Some("recover run-999"));
    assert_eq!(wrong_phrase.code, 1, "WRS-004: {wrong_phrase}");
    assert_eq!(
        wrong_phrase.message,
        format!(
            "recover refused; recover is unconfirmed: confirmation {:?} does not match the \
             required phrase {:?}",
            "recover run-999",
            format!("recover {run}")
        ),
        "WRS-004: the wrong-phrase refusal names the wrong Run identity"
    );
    governed
        .project
        .assert_unchanged(&before, "unconfirmed recovery");

    // The argument shapes: an extra argument and missing values.
    let extra = rtm(
        governed.project.root(),
        &[
            "recover",
            "--run",
            &run,
            "stray-positional",
            "--decision",
            &shown(&decision),
            "--confirm",
            &format!("recover {run}"),
        ],
    );
    assert_eq!(extra.code, 1, "WRS-004: {extra}");
    assert_eq!(
        extra.message, "recover: unexpected extra argument stray-positional",
        "WRS-004: the extra-argument refusal mirrors hold's shape"
    );
    let bare_decision = rtm(
        governed.project.root(),
        &[
            "recover",
            "--run",
            &run,
            "--decision",
            "--confirm",
            &format!("recover {run}"),
        ],
    );
    assert_eq!(bare_decision.code, 1, "WRS-004: {bare_decision}");
    assert_eq!(
        bare_decision.message, "recover: --decision needs a path to the signed recovery decision",
        "WRS-004: the bare --decision refusal mirrors hold's shape"
    );
    let bare_enrollment = rtm(
        governed.project.root(),
        &[
            "recover",
            "--run",
            &run,
            "--enrollment",
            "--decision",
            &shown(&decision),
            "--confirm",
            &format!("recover {run}"),
        ],
    );
    assert_eq!(bare_enrollment.code, 1, "WRS-004: {bare_enrollment}");
    assert_eq!(
        bare_enrollment.message,
        "recover: --enrollment needs a path to the externally enrolled authority",
        "WRS-004: the bare --enrollment refusal mirrors hold's shape"
    );

    // A valueless --confirm is an argument shape, checked while the
    // arguments are read, before every later check.
    let bare_confirm = rtm(
        governed.project.root(),
        &[
            "recover",
            "--run",
            &run,
            "--decision",
            &shown(&decision),
            "--confirm",
        ],
    );
    assert_eq!(bare_confirm.code, 1, "WRS-004: {bare_confirm}");
    assert_eq!(
        bare_confirm.message, "recover: --confirm needs the exact confirmation phrase",
        "WRS-004: the valueless --confirm refusal mirrors hold's constructor"
    );
    governed
        .project
        .assert_unchanged(&before, "valueless --confirm recovery");

    // Supplying --enrollment for a Run whose prior trusted policy exists is
    // an argument-validation usage refusal.
    let manifest_path = governed.keys.dir.join("enrollment-manifest.toml");
    let redundant = governed.recover(
        &run,
        Some(&decision),
        Some(&manifest_path),
        Some(&format!("recover {run}")),
    );
    assert_eq!(redundant.code, 2, "WRS-004: {redundant}");
    assert_eq!(
        redundant.message, "recover: --enrollment addresses no missing recovery trust",
        "WRS-004: the redundant enrollment refusal mirrors the landed resolution shape"
    );

    // Absent authorization: no decision observes none.
    let none = governed.recover(&run, None, None, Some(&format!("recover {run}")));
    assert_eq!(none.code, 1, "WRS-004: {none}");
    assert_eq!(
        none.message,
        format!(
            "recover refused; authorization: decision: observed none; expected one \
             ratmac-recover-v1 signature by the designated approver over the canonical intent \
             (rtm intent --run {run} --recover)"
        ),
        "WRS-004: the absent authorization refusal is one line naming the intent verb"
    );
    assert_eq!(none.message.lines().count(), 1);
    governed
        .project
        .assert_unchanged(&before, "unauthorized recovery");

    // A changed owning class refuses: the corrected runbook drops the item
    // class of a ledger-recorded child. The compatibility rule fires before
    // the authorization boundary, so the parent's decision file serves.
    let child = governed.project.spawn_child(&run, "ref=t-100.md");
    assert_eq!(governed.project.state(&child), "make");
    governed.rewrite(&machine_body(None, false, false, "build"), &ENROLLED, false);
    let dropped = governed.recover(
        &child,
        Some(&decision),
        None,
        Some(&format!("recover {child}")),
    );
    assert_eq!(dropped.code, 1, "WRS-004: {dropped}");
    assert_eq!(
        dropped.message,
        "recover refused; the corrected runbook declares no class item; recovery keeps the \
         owning class",
        "WRS-004: a dropped owning class refuses"
    );

    // A removed State refuses while the class stays.
    governed.rewrite(
        &machine_body(None, true, false, "assemble"),
        &ENROLLED,
        true,
    );
    let removed = governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
    assert_eq!(removed.code, 1, "WRS-004: {removed}");
    assert_eq!(
        removed.message,
        "recover refused; the corrected runbook declares no State build in class runbook; \
         recovery refuses a removed State",
        "WRS-004: a removed State refuses"
    );

    // A terminal Run may not be amended; no drift is needed for the
    // terminal family to fire.
    governed.rewrite(&machine_body(None, true, false, "build"), &ENROLLED, true);
    let finished = governed.admit();
    governed.step_ok(&finished, "w2-finish-1");
    governed.step_ok(&finished, "w2-finish-2");
    assert_eq!(governed.project.status(&finished), "passed");
    let terminal = governed.recover(&finished, None, None, Some(&format!("recover {finished}")));
    assert_eq!(terminal.code, 1, "WRS-004: {terminal}");
    assert_eq!(
        terminal.message,
        format!(
            "recover refused; run {finished} is terminal (status passed): recovery may not \
             amend a terminal Run"
        ),
        "WRS-004: a terminal Run refuses"
    );

    // Corrupt prior evidence refuses through the strict parse detail.
    let corroded = governed.admit();
    fs::write(
        governed.project.run_dir(&corroded).join("evidence.toml"),
        "not a strict record [[[\n",
    )
    .expect("plant the corrupt evidence");
    let corrupt = governed.recover(&corroded, None, None, Some(&format!("recover {corroded}")));
    assert_eq!(corrupt.code, 1, "WRS-004: {corrupt}");
    assert!(
        corrupt.message.starts_with(&format!(
            "recover refused; the recorded evidence for run {corroded} does not parse: "
        )),
        "WRS-004: corrupt prior evidence refuses through the strict detail: {corrupt}"
    );

    // The workspace twin: a held top-level Run records the primary checkout
    // as its workspace, so a recovery invoked from a linked checkout of the
    // same repository resolves a different workspace and refuses. Every
    // earlier check passes: the linked checkout reads the committed
    // runbook, which declares the Run's class and State.
    let spaced = governed.admit();
    let held = governed.hold_governed(&spaced, "w2-space-hold");
    assert_eq!(
        held.code, 0,
        "WRS-004: the governed hold succeeds: {}{}",
        held.stdout, held.message
    );
    let pause_text = fs::read_to_string(governed.project.run_dir(&spaced).join("pauses/1.toml"))
        .expect("read the pause record");
    let pause: toml::Value = pause_text.parse().expect("the pause record is TOML");
    let recorded = pause["workspace"].as_str().unwrap_or_default().to_owned();
    let primary = shown(&fs::canonicalize(governed.project.root()).unwrap());
    assert_eq!(
        recorded, primary,
        "WRS-004: the pause record's workspace is the primary, canonical, forward slashes"
    );
    let linked_parent = TempTree::new("wrsv00402-linked").expect("own the linked checkout tree");
    let linked = linked_parent.join("checkout");
    let added = support::git(governed.project.root())
        .args(["worktree", "add", &linked.to_string_lossy(), "--detach"])
        .output()
        .expect("add the linked checkout");
    assert!(
        added.status.success(),
        "WRS-004: the linked checkout is created: {}",
        support::text(&added)
    );
    assert!(
        linked.join(".ratmac/ratmac.toml").is_file(),
        "WRS-004: the linked checkout carries the committed runbook"
    );
    let observed = shown(&fs::canonicalize(&linked).unwrap());
    assert_ne!(recorded, observed, "the linked checkout resolves elsewhere");
    let before_space = governed.project.snapshot();
    let mismatched = rtm(
        &linked,
        &[
            "recover",
            "--run",
            &spaced,
            "--decision",
            &shown(&decision),
            "--confirm",
            &format!("recover {spaced}"),
        ],
    );
    assert_eq!(mismatched.code, 1, "WRS-004: {mismatched}");
    assert!(
        mismatched
            .message
            .starts_with("recover refused; the recorded workspace ")
            && mismatched
                .message
                .contains(" differs from the resolved workspace ")
            && mismatched.message.contains(&recorded)
            && mismatched.message.contains(&observed)
            && mismatched
                .message
                .ends_with("; recovery keeps the workspace"),
        "WRS-004: the workspace twin names both canonical spellings: {mismatched}"
    );
    governed
        .project
        .assert_unchanged(&before_space, "workspace-mismatched recovery");
    let removed = support::git(governed.project.root())
        .args(["worktree", "remove", "--force", &linked.to_string_lossy()])
        .output()
        .expect("remove the linked checkout");
    assert!(
        removed.status.success(),
        "WRS-004: the linked checkout is removed: {}",
        support::text(&removed)
    );
    drop(linked_parent);

    // The chained workspace twin: a committed amendment records its own
    // context workspace, so a non-held top-level Run with one amendment
    // refuses a recovery from a different checkout the same way.
    let chained = governed.admit();
    governed
        .project
        .rewrite_runbook(&correction(&governed.runbook_with(&body, &ENROLLED, false)));
    governed.amend(&chained, 1, "w2-chained");
    let member: toml::Value =
        String::from_utf8_lossy(&governed.project.amendment_bytes(&chained, 1).unwrap())
            .parse()
            .expect("the chained member is TOML");
    let recorded = member["context"]["workspace"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(
        recorded, primary,
        "WRS-004: the member's recorded workspace is the primary, canonical, forward slashes"
    );
    let chained_intent = governed.recover_intent(&chained, None, "w2-chained-linked-intent");
    let chained_decision = governed.decision_over(&chained_intent, "w2-chained-linked");
    let linked_parent = TempTree::new("wrsv00402-linked2").expect("own the second linked tree");
    let linked = linked_parent.join("checkout");
    let added = support::git(governed.project.root())
        .args(["worktree", "add", &linked.to_string_lossy(), "--detach"])
        .output()
        .expect("add the second linked checkout");
    assert!(
        added.status.success(),
        "WRS-004: the second linked checkout is created: {}",
        support::text(&added)
    );
    let observed = shown(&fs::canonicalize(&linked).unwrap());
    let before_chained = governed.project.snapshot();
    let chained_refused = rtm(
        &linked,
        &[
            "recover",
            "--run",
            &chained,
            "--decision",
            &shown(&chained_decision),
            "--confirm",
            &format!("recover {chained}"),
        ],
    );
    assert_eq!(chained_refused.code, 1, "WRS-004: {chained_refused}");
    assert!(
        chained_refused
            .message
            .starts_with("recover refused; the recorded workspace ")
            && chained_refused
                .message
                .contains(" differs from the resolved workspace ")
            && chained_refused.message.contains(&recorded)
            && chained_refused.message.contains(&observed)
            && chained_refused
                .message
                .ends_with("; recovery keeps the workspace"),
        "WRS-004: the chained twin names both canonical spellings: {chained_refused}"
    );
    governed
        .project
        .assert_unchanged(&before_chained, "chained workspace-mismatched recovery");
    let removed = support::git(governed.project.root())
        .args(["worktree", "remove", "--force", &linked.to_string_lossy()])
        .output()
        .expect("remove the second linked checkout");
    assert!(
        removed.status.success(),
        "WRS-004: the second linked checkout is removed: {}",
        support::text(&removed)
    );
    drop(linked_parent);
}

// --- WRSV-004-03 ---------------------------------------------------------------

/// WRSV-004-03: interruption before the amendment rename, after it, and during the history append leaves the old head effective until publication commits, reports committed/history-pending, and a retry reconciles exactly once while readers report without repair.
#[test]
fn wrsv_004_03() {
    let _lane = support::hook_lane();
    let body = machine_body(None, false, false, "build");
    let keys = KeySet::new("wrsv00403", &["a", "b", "lead", "outsider"]);

    // The valid positive case: a completed recovery leaves no pending
    // report anywhere.
    {
        let governed = Governed::with_keys("wrsv00403a", &body, false, keys.clone());
        let run = governed.admit();
        governed.step_ok(&run, "w3a-control");
        governed
            .project
            .rewrite_runbook(&correction(&governed.project.runbook_text()));
        governed.amend(&run, 1, "w3a");
        let status = rtm(governed.project.root(), &["status", "--run", &run]);
        assert_eq!(status.code, 0, "WRS-004: {status}");
        assert!(
            !status.stdout.contains("Amendment pending"),
            "WRS-004: a completed recovery leaves no pending line: {}",
            status.stdout
        );
        assert!(
            governed
                .project
                .history()
                .contains(&format!("- Recover: run {run} adopted amendment 1")),
            "the completed recovery records its event"
        );
    }

    // Interruption before the rename: the old head stays effective, the
    // preparation file is left and ignored, and the repeat commits.
    {
        let governed = Governed::with_keys("wrsv00403b", &body, false, keys.clone());
        let run = governed.admit();
        let evidence_before = governed.project.evidence_bytes(&run);
        governed
            .project
            .rewrite_runbook(&correction(&governed.project.runbook_text()));
        let intent = governed.recover_intent(&run, None, "w3b-intent");
        let decision = governed.decision_over(&intent, "w3b");
        std::env::set_var("RATMAC_TEST_RECOVER_FAULT", "before-rename");
        let interrupted =
            governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
        std::env::remove_var("RATMAC_TEST_RECOVER_FAULT");
        assert_eq!(
            interrupted.code, 1,
            "WRS-004: the before-rename fault refuses: {interrupted}"
        );
        assert_eq!(
            interrupted.message, "recover refused; injected recover fault at before-rename",
            "WRS-004: the before-rename refusal names the injected fault"
        );
        assert!(
            governed
                .project
                .amendments_dir(&run)
                .join("1.prepare")
                .is_file(),
            "WRS-004: the complete preparation file is left behind"
        );
        assert!(
            governed.project.amendment_bytes(&run, 1).is_none(),
            "WRS-004: the old head remains effective before the rename"
        );
        assert_eq!(
            governed.project.evidence_bytes(&run),
            evidence_before,
            "WRS-004: the interrupted recovery keeps the original pins"
        );
        assert!(
            !governed.project.history().contains("adopted amendment"),
            "WRS-004: no history event is recorded before the rename"
        );
        // The reader keeps today's refusal under the drifted runbook: the
        // preparation file is ignored, not read.
        let before_status = governed.project.snapshot();
        let status = rtm(governed.project.root(), &["status", "--run", &run]);
        assert_eq!(
            status.code, 1,
            "WRS-004: the drifted runbook refuses the reader: {status}"
        );
        assert!(
            status.message.contains(
                "runbook pin mismatch: .ratmac/ratmac.toml drifted since rtm start — observed sha256="
            ),
            "WRS-004: the preparation file never relaxes the pin comparison: {status}"
        );
        assert!(
            status.message.contains("; nothing was modified"),
            "WRS-004: the refusal keeps today's closing text: {status}"
        );
        governed
            .project
            .assert_unchanged(&before_status, "status over a drifted runbook");
        // The repeat mints the same unused sequence and commits.
        let repeat = governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
        assert_eq!(repeat.code, 0, "WRS-004: the repeat commits: {repeat}");
        assert_eq!(
            repeat.stdout,
            success_lines(&run, 1),
            "WRS-004: the repeat prints the success lines naming sequence 1"
        );
        assert!(governed.project.amendment_bytes(&run, 1).is_some());
        assert!(
            !governed
                .project
                .amendments_dir(&run)
                .join("1.prepare")
                .is_file(),
            "WRS-004: the committed repeat leaves no preparation file"
        );
        assert_eq!(
            governed
                .project
                .history()
                .matches(&format!("- Recover: run {run} adopted amendment 1"))
                .count(),
            1,
            "WRS-004: the committed repeat records the event exactly once"
        );
    }

    // Interruption after the rename: the amendment is committed, the
    // history is pending, mutation refuses, readers report without repair,
    // and the repeat reconciles exactly once.
    {
        let governed = Governed::with_keys("wrsv00403c", &body, false, keys.clone());
        let run = governed.admit();
        let evidence_before = governed.project.evidence_bytes(&run);
        governed
            .project
            .rewrite_runbook(&correction(&governed.project.runbook_text()));
        let intent = governed.recover_intent(&run, None, "w3c-intent");
        let decision = governed.decision_over(&intent, "w3c");
        std::env::set_var("RATMAC_TEST_RECOVER_FAULT", "after-rename");
        let interrupted =
            governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
        std::env::remove_var("RATMAC_TEST_RECOVER_FAULT");
        assert_eq!(
            interrupted.code, 1,
            "WRS-004: the after-rename fault reports its committed state: {interrupted}"
        );
        assert_eq!(
            interrupted.message,
            "recover: committed amendment 1 with history pending: injected recover fault at \
             after-rename; repeat this command to reconcile it exactly once",
            "WRS-004: the committed-history-pending rendering names the fault and the repeat"
        );
        assert!(
            governed.project.amendment_bytes(&run, 1).is_some(),
            "WRS-004: the rename committed the new verified head"
        );
        assert_eq!(
            governed.project.evidence_bytes(&run),
            evidence_before,
            "WRS-004: the committed amendment keeps the original pins byte-identical"
        );
        assert!(
            !governed.project.history().contains("adopted amendment"),
            "WRS-004: the history event is the missing tail"
        );
        assert!(
            governed.project.authorizations(&run).is_empty(),
            "WRS-004: the archive append never happened before the fault"
        );

        // Mutation refuses until the history is reconciled.
        let stepped = rtm(governed.project.root(), &["step", "--run", &run]);
        assert_eq!(stepped.code, 1, "WRS-004: {stepped}");
        assert!(
            stepped.message.contains(&format!(
                "amendment history is incomplete for run {run}: sequence 1 is committed with \
                 no history event; repeat rtm recover --run {run} to reconcile it"
            )),
            "WRS-004: the step refuses further mutation naming the reconciliation: {stepped}"
        );

        // Readers report without repair.
        let status = rtm(governed.project.root(), &["status", "--run", &run]);
        assert_eq!(status.code, 0, "WRS-004: {status}");
        let next_at = status
            .stdout
            .rfind("next: ")
            .expect("the ordinary report carries its next line");
        let pending_at = status
            .stdout
            .find(&format!(
                "Amendment pending: sequence 1 for this Run is committed with its history \
                 event missing; a repeated rtm recover --run {run} reconciles it exactly \
                 once.\n"
            ))
            .unwrap_or_else(|| {
                panic!(
                    "WRS-004: the reader appends its pending line: {}",
                    status.stdout
                )
            });
        assert!(
            pending_at > next_at,
            "WRS-004: the pending line follows the next line: {}",
            status.stdout
        );
        assert!(
            !governed.project.history().contains("adopted amendment"),
            "WRS-004: the reader repaired nothing"
        );

        // Doctor reports one warning finding.
        let doctor = rtm(governed.project.root(), &["doctor"]);
        assert_eq!(doctor.code, 1, "WRS-004: warning-only exits 1: {doctor}");
        let engine_root = shown(&governed.project.root().join(".ratmac"));
        assert!(
            doctor.stdout.contains(&format!(
                "RB902 warning: run {run:?}: amendment pending: {engine_root}/runs/{run}/\
                 amendments/1.toml is committed with no history event; a repeated rtm recover \
                 reconciles it"
            )),
            "WRS-004: doctor's finding carries the pinned RB902 code and run \
             location: {}",
            doctor.stdout
        );

        // The repeat reconciles exactly once: one archive record, one event,
        // no second amendment, and the replay then refuses.
        let reconcile = governed.recover(&run, None, None, Some(&format!("recover {run}")));
        assert_eq!(
            reconcile.code, 0,
            "WRS-004: the repeat reconciles: {reconcile}"
        );
        assert_eq!(
            reconcile.stdout,
            success_lines(&run, 1),
            "WRS-004: the reconciliation prints the success lines naming sequence 1"
        );
        assert_eq!(
            governed
                .project
                .history()
                .matches(&format!("- Recover: run {run} adopted amendment 1"))
                .count(),
            1,
            "WRS-004: the reconciliation appends exactly one event"
        );
        assert!(
            governed.project.amendment_bytes(&run, 2).is_none(),
            "WRS-004: the reconciliation mints no second amendment"
        );
        let records = governed.project.authorizations(&run);
        assert_eq!(
            records.len(),
            1,
            "the reconciliation appends one archive record"
        );
        let replay = governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
        assert_eq!(replay.code, 1, "WRS-004: {replay}");
        assert!(
            replay.message.starts_with(
                "recover refused; authorization: replay: observed the decision already \
                 consumed as sequence 1 under "
            ) && replay.message.contains(&format!(
                "/runs/{run}/authorizations/; expected a fresh authorization at sequence 2"
            )),
            "WRS-004: the replayed decision refuses naming the consumed sequence: {replay}"
        );
        let doctor_after = rtm(governed.project.root(), &["doctor"]);
        assert!(
            !doctor_after.stdout.contains("amendment pending"),
            "WRS-004: the reconciliation clears the warning finding: {}",
            doctor_after.stdout
        );
    }

    // Interruption during the history append: the archive record exists,
    // the event is missing, and the repeat reconciles.
    {
        let governed = Governed::with_keys("wrsv00403d", &body, false, keys.clone());
        let run = governed.admit();
        governed
            .project
            .rewrite_runbook(&correction(&governed.project.runbook_text()));
        let intent = governed.recover_intent(&run, None, "w3d-intent");
        let decision = governed.decision_over(&intent, "w3d");
        std::env::set_var("RATMAC_TEST_RECOVER_FAULT", "history-append");
        let interrupted =
            governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
        std::env::remove_var("RATMAC_TEST_RECOVER_FAULT");
        assert_eq!(
            interrupted.code, 1,
            "WRS-004: the history-append fault reports its committed state: {interrupted}"
        );
        assert_eq!(
            interrupted.message,
            "recover: committed amendment 1 with history pending: injected recover fault at \
             history-append; repeat this command to reconcile it exactly once",
            "WRS-004: the history-append rendering names the fault and the repeat"
        );
        assert!(governed.project.amendment_bytes(&run, 1).is_some());
        assert_eq!(
            governed.project.authorizations(&run).len(),
            1,
            "WRS-004: the archive record was appended before the history fault"
        );
        assert!(
            !governed.project.history().contains("adopted amendment"),
            "WRS-004: the history event is still missing"
        );
        let reconcile = governed.recover(&run, None, None, Some(&format!("recover {run}")));
        assert_eq!(
            reconcile.code, 0,
            "WRS-004: the repeat reconciles: {reconcile}"
        );
        assert_eq!(reconcile.stdout, success_lines(&run, 1));
        assert_eq!(
            governed.project.authorizations(&run).len(),
            1,
            "WRS-004: the reconciliation appends no duplicate archive record"
        );
        assert_eq!(
            governed
                .project
                .history()
                .matches(&format!("- Recover: run {run} adopted amendment 1"))
                .count(),
            1,
            "WRS-004: the reconciliation appends the one missing event"
        );
    }

    // A torn history tail: a log fragment that is a proper prefix of the
    // expected event is completed by appending the missing suffix; a
    // fragment that is no prefix refuses, writing nothing.
    {
        let governed = Governed::with_keys("wrsv00403e", &body, false, keys.clone());
        let run = governed.admit();
        governed
            .project
            .rewrite_runbook(&correction(&governed.project.runbook_text()));
        let intent = governed.recover_intent(&run, None, "w3e-intent");
        let decision = governed.decision_over(&intent, "w3e");
        std::env::set_var("RATMAC_TEST_RECOVER_FAULT", "history-append");
        let committed =
            governed.recover(&run, Some(&decision), None, Some(&format!("recover {run}")));
        std::env::remove_var("RATMAC_TEST_RECOVER_FAULT");
        assert_eq!(committed.code, 1, "WRS-004: {committed}");
        assert!(governed.project.amendment_bytes(&run, 1).is_some());
        let event = format!(
            "- Recover: run {run} adopted amendment 1 on an explicit signed recovery decision.\n"
        );
        let log = governed.project.root().join(".ratmac/log.md");
        let base_history = governed.project.history();

        // A fragment that is no prefix of the expected event.
        let torn_at = event.find("on an explicit").expect("the event continues") + 3;
        let mut wrong: Vec<u8> = event.as_bytes()[..torn_at].to_vec();
        wrong[torn_at - 2] = b'x';
        let wrong = String::from_utf8_lossy(&wrong).into_owned();
        fs::write(&log, format!("{base_history}{wrong}")).expect("plant the no-prefix fragment");
        let before_wrong = governed.project.snapshot();
        let refused = governed.recover(&run, None, None, Some(&format!("recover {run}")));
        assert_eq!(refused.code, 1, "WRS-004: {refused}");
        assert_eq!(
            refused.message,
            format!(
                "recover refused; the amendment history for run {run} holds a fragment that is \
                 no prefix of the expected event for sequence 1; repair it outside the Engine"
            ),
            "WRS-004: a no-prefix fragment names the repair boundary"
        );
        governed
            .project
            .assert_unchanged(&before_wrong, "reconcile over a no-prefix fragment");

        // A fragment torn before the sequence number, carrying one wrong
        // byte, holds neither the full expected event prefix - so the
        // earlier history prefilter does not answer - nor any prefix of
        // the event: the history loop itself refuses it.
        let short_at = event.find("adopted").expect("the event names the adoption");
        let mut short_wrong: Vec<u8> = event.as_bytes()[..short_at].to_vec();
        short_wrong[short_at - 2] = b'x';
        let short_wrong = String::from_utf8_lossy(&short_wrong).into_owned();
        fs::write(&log, format!("{base_history}{short_wrong}"))
            .expect("plant the short no-prefix fragment");
        let before_short = governed.project.snapshot();
        let short_refused = governed.recover(&run, None, None, Some(&format!("recover {run}")));
        assert_eq!(short_refused.code, 1, "WRS-004: {short_refused}");
        assert_eq!(
            short_refused.message,
            format!(
                "recover refused; the amendment history for run {run} holds a fragment that \
                 is no prefix of the expected event for sequence 1; repair it outside the Engine"
            ),
            "WRS-004: a short no-prefix fragment reaches the history loop's own refusal"
        );
        governed
            .project
            .assert_unchanged(&before_short, "reconcile over a short no-prefix fragment");

        // A proper prefix, torn past the sequence number, is completed.
        fs::write(&log, format!("{base_history}{}", &event[..torn_at]))
            .expect("plant the torn prefix");
        let completed = governed.recover(&run, None, None, Some(&format!("recover {run}")));
        assert_eq!(
            completed.code, 0,
            "WRS-004: the repeat completes the torn tail: {completed}"
        );
        assert_eq!(completed.stdout, success_lines(&run, 1));
        let history = governed.project.history();
        assert!(
            history.ends_with(&event),
            "WRS-004: the log ends with exactly one full event and nothing truncated"
        );
        assert_eq!(history.matches(&event).count(), 1);
    }
}

// --- WRSV-004-04 ---------------------------------------------------------------

/// WRSV-004-04: a changed rule makes old receipts insufficient, forged approval text and replayed verdicts cannot advance the Run, and the child keeps its parent ledger ownership throughout.
#[test]
fn wrsv_004_04() {
    let _lane = support::hook_lane();
    let body = machine_body(Some("t-100.md"), true, false, "build");
    let governed = Governed::new("wrsv00404", &body, true);
    let run = governed.admit();
    governed.step_ok(&run, "w4-enter-build");
    assert_eq!(governed.project.state(&run), "build");

    // The child and its parent ledger ownership.
    let child = governed.project.spawn_child(&run, "ref=t-100.md");
    let ledger = governed.project.run_dir(&run).join("spawn-ledger");
    let ledger_before = fs::read(&ledger).expect("read the spawn ledger");

    // Old receipts recorded against the original authority.
    governed.project.write_receipts(&run, "t-100", None);
    let receipts_before = governed.project.receipt_files(&run);
    assert!(
        !receipts_before.is_empty(),
        "WRS-004: the fixture recorded completion receipts"
    );

    // The rule change and the authorized repair.
    governed
        .project
        .rewrite_runbook(&correction(&governed.project.runbook_text()));
    governed.amend(&run, 1, "w4");
    let head = sha256_hex(&governed.project.amendment_bytes(&run, 1).unwrap());
    assert_eq!(
        fs::read(&ledger).unwrap(),
        ledger_before,
        "WRS-004: the parent's spawn ledger keeps its bytes through the amendment"
    );

    // Old receipts are insufficient: the step refuses naming the authority
    // head they must carry.
    let stale = governed.step_governed(&run, "w4-stale");
    assert_eq!(stale.code, 0, "WRS-004: {stale}");
    assert!(
        stale
            .stdout
            .contains("receipt is stale: it is bound to authority head"),
        "WRS-004: the changed rule makes the old receipts stale: {}",
        stale.stdout
    );
    assert!(
        stale.stdout.contains(&head),
        "WRS-004: the stale refusal names the effective head: {}",
        stale.stdout
    );

    // Forged approval text substitutes nothing: the amended authority file
    // alone advances no Run and authorizes no motion.
    governed.project.write(
        ".arca/amendment-res-188.md",
        "# Accepted authority amendment\n\nThe pinned rule itself changed; the recovery was \
         accepted under the rule-fix contract, so work may continue.\n",
    );
    let forged = governed.recover(&run, None, None, Some(&format!("recover {run}")));
    assert_eq!(forged.code, 1, "WRS-004: {forged}");
    assert!(
        forged.message.starts_with(
            "recover refused; authorization: decision: observed none; expected one \
             ratmac-recover-v1"
        ),
        "WRS-004: forged approval prose observes no decision: {forged}"
    );
    let prose_step = governed.step_governed(&run, "w4-prose");
    assert!(
        prose_step.stdout.contains("step refused"),
        "WRS-004: the forged prose alone advances no Run: {}",
        prose_step.stdout
    );

    // Fresh proof under the corrected authority passes, and its verdict
    // machinery still refuses a replay of the consumed bundle.
    governed.project.write_receipts(&run, "t-100", Some(&head));
    let consumed = governed.authorization_args(&run, None, "w4-fresh");
    let mut fresh_args = vec!["step".to_owned(), "--run".to_owned(), run.to_owned()];
    fresh_args.extend(consumed.clone());
    let fresh_refs: Vec<&str> = fresh_args.iter().map(String::as_str).collect();
    let fresh = rtm(governed.project.root(), &fresh_refs);
    assert_eq!(
        fresh.code, 0,
        "WRS-004: fresh proof passes under the corrected authority: {}{}",
        fresh.stdout, fresh.message
    );
    assert!(!fresh.stdout.contains("step refused"), "{fresh}");
    assert_eq!(governed.project.status(&run), "passed");

    // A replayed verdict cannot advance the finished Run back.
    let replayed = rtm(governed.project.root(), &fresh_refs);
    let replayed_all = format!("{}{}", replayed.stdout, replayed.message);
    assert!(
        replayed_all.contains("step refused") || replayed_all.contains("terminal"),
        "WRS-004: the replayed authorization bundle does not advance the Run: {replayed_all}"
    );

    // The child keeps its parent ledger ownership through its own
    // recovery: the ledger's bytes never change.
    let child_intent = governed.recover_intent(&child, None, "w4-child-intent");
    let child_decision = governed.decision_over(&child_intent, "w4-child");
    let child_recovered = governed.recover(
        &child,
        Some(&child_decision),
        None,
        Some(&format!("recover {child}")),
    );
    assert_eq!(
        child_recovered.code, 0,
        "WRS-004: the child's own recovery succeeds: {child_recovered}"
    );
    assert_eq!(child_recovered.stdout, success_lines(&child, 1));
    assert_eq!(
        fs::read(&ledger).unwrap(),
        ledger_before,
        "WRS-004: the child's recovery writes no ledger byte"
    );
    assert!(
        !governed.project.receipt_files(&run).is_empty(),
        "WRS-004: the receipts remain recorded evidence through the child's recovery"
    );
}

// --- WRSV-004-05 ---------------------------------------------------------------

/// WRSV-004-05: a candidate that replaces reviewer keys cannot authorize itself, the independently signed exact amendment under the prior policy succeeds, and omitted changes, substituted identities, and a stale chain tip refuse.
#[test]
fn wrsv_004_05() {
    let _lane = support::hook_lane();
    let body = machine_body(None, false, false, "build");
    let governed = Governed::new("wrsv00405", &body, false);

    // The intent routing: the recover purpose refuses its rivals.
    let routed = governed.admit();
    let pair_out = governed.keys.dir.join("w5-pair.bin");
    let review_out = governed.keys.dir.join("w5-review.bin");
    let plain_out = governed.keys.dir.join("w5-plain.bin");
    let hold_pair = rtm(
        governed.project.root(),
        &[
            "intent",
            "--run",
            &routed,
            "--hold",
            "--blocker",
            BLOCKER,
            "--recover",
            "--out",
            &shown(&pair_out),
        ],
    );
    assert_eq!(hold_pair.code, 1, "WRS-004: {hold_pair}");
    assert_eq!(
        hold_pair.message,
        "intent refused; --recover and --hold name different intents; pass exactly one",
        "WRS-004: the hold pair refuses in the landed pair shape"
    );
    let resume_pair = rtm(
        governed.project.root(),
        &[
            "intent",
            "--run",
            &routed,
            "--resume",
            "--recover",
            "--out",
            &shown(&pair_out),
        ],
    );
    assert_eq!(resume_pair.code, 1, "WRS-004: {resume_pair}");
    assert_eq!(
        resume_pair.message,
        "intent refused; --recover and --resume name different intents; pass exactly one",
        "WRS-004: the resume pair refuses in the landed pair shape"
    );
    let review_misuse = rtm(
        governed.project.root(),
        &[
            "intent",
            "--run",
            &routed,
            "--recover",
            "--review",
            "a:whatever.sig",
            "--out",
            &shown(&review_out),
        ],
    );
    assert_eq!(review_misuse.code, 1, "WRS-004: {review_misuse}");
    assert_eq!(
        review_misuse.message, "intent refused; --review addresses no recovery intent",
        "WRS-004: a review signature addresses no recovery intent"
    );
    let enrollment_misuse = rtm(
        governed.project.root(),
        &[
            "intent",
            "--run",
            &routed,
            "--enrollment",
            "some-manifest.toml",
            "--out",
            &shown(&plain_out),
        ],
    );
    assert_eq!(enrollment_misuse.code, 1, "WRS-004: {enrollment_misuse}");
    assert_eq!(
        enrollment_misuse.message, "intent refused; --enrollment addresses no recovery intent",
        "WRS-004: an enrollment argument addresses no ordinary intent"
    );

    // The positive control: an independently signed exact amendment under
    // the prior policy.
    let run = governed.admit();
    let rule_fix = correction(&governed.runbook_with(&body, &ENROLLED, false));
    governed.project.rewrite_runbook(&rule_fix);
    governed.amend(&run, 1, "w5-positive");

    // A candidate that replaces the reviewer keys cannot authorize itself:
    // its own Approver exists only in the proposed policy.
    governed
        .project
        .rewrite_runbook(&governed.runbook_with(&body, &ENROLLED, false));
    let candidate = governed.admit();
    let candidate_runbook = correction(&governed.runbook_with(&body, &CANDIDATE, false));
    governed.project.rewrite_runbook(&candidate_runbook);
    let outsider_intent = governed.recover_intent(&candidate, None, "w5-outsider");
    let outsider_decision = governed
        .keys
        .sign("outsider", RECOVER_NAMESPACE, &outsider_intent);
    let candidate_before = governed.project.snapshot();
    let self_authorized = governed.recover(
        &candidate,
        Some(&outsider_decision),
        None,
        Some(&format!("recover {candidate}")),
    );
    assert_eq!(self_authorized.code, 1, "WRS-004: {self_authorized}");
    assert_eq!(
        self_authorized.message,
        "recover refused; authorization: decision: observed a signature by no enrolled \
         identity; expected the designated approver \"lead\"",
        "WRS-004: a key that exists only in the proposed policy fails attribution"
    );
    assert_eq!(self_authorized.message.lines().count(), 1);
    assert!(
        governed.project.amendment_bytes(&candidate, 1).is_none(),
        "WRS-004: the self-authorized candidate wrote nothing"
    );
    governed
        .project
        .assert_unchanged(&candidate_before, "self-authorized candidate");

    // The admit runs under the enrolled runbook: recover opens the Run
    // through today's runbook parse, and no relaxed parse exists, so the
    // candidate policy must be off disk before a Run is admitted.
    governed
        .project
        .rewrite_runbook(&governed.runbook_with(&body, &ENROLLED, false));
    // Omitted changes: a decision over the earlier intent omits the policy
    // change the current artifacts derive.
    let omitted = governed.admit();
    governed.project.rewrite_runbook(&rule_fix);
    let early_intent = governed.recover_intent(&omitted, None, "w5-early");
    let early_decision = governed.decision_over(&early_intent, "w5-early");
    let replaced = correction(&governed.runbook_with(&body, &REPLACEMENT_A, false));
    governed.project.rewrite_runbook(&replaced);
    let current_intent = governed.recover_intent(&omitted, None, "w5-current");
    let omitted_ran = governed.recover(
        &omitted,
        Some(&early_decision),
        None,
        Some(&format!("recover {omitted}")),
    );
    assert_eq!(omitted_ran.code, 1, "WRS-004: {omitted_ran}");
    assert_eq!(
        omitted_ran.message,
        format!(
            "recover refused; authorization: decision: observed a signature that does not \
             verify against the recomputed canonical intent sha256 {}; expected a fresh \
             ratmac-recover-v1 decision of that exact intent (rtm intent --run {omitted} \
             --recover)",
            sha256_hex(&fs::read(&current_intent).unwrap())
        ),
        "WRS-004: an omitted change refuses through the recomputed intent digest"
    );

    // Substituted identities: a decision over a different corrected runbook
    // refuses through the same family.
    let substituted = format!("{replaced}# a different correction content\n");
    governed.project.rewrite_runbook(&substituted);
    let substituted_intent = governed.recover_intent(&omitted, None, "w5-substituted");
    let substituted_ran = governed.recover(
        &omitted,
        Some(&early_decision),
        None,
        Some(&format!("recover {omitted}")),
    );
    assert_eq!(substituted_ran.code, 1, "WRS-004: {substituted_ran}");
    assert_eq!(
        substituted_ran.message,
        format!(
            "recover refused; authorization: decision: observed a signature that does not \
             verify against the recomputed canonical intent sha256 {}; expected a fresh \
             ratmac-recover-v1 decision of that exact intent (rtm intent --run {omitted} \
             --recover)",
            sha256_hex(&fs::read(&substituted_intent).unwrap())
        ),
        "WRS-004: a substituted identity refuses through the recomputed intent digest"
    );

    // A stale chain tip: a decision bound to an older head refuses while
    // the authority it proposes is current again. The chain walks forward
    // and back - a second content correction, then a revert to the head-1
    // content - so the effective head's old authority is the same enrolled
    // line the stale decision was signed under, and only the bound head
    // differs. The stale decision proposes different content than either
    // amend that lands: ssh-keygen signatures are deterministic, so signing
    // the exact bytes a landed amend signs would replay, not go stale.
    let second_rule = format!(
        "{}# the second archived-record correction.\n",
        governed.runbook_with(&body, &ENROLLED, false)
    );
    let stale_rule = format!(
        "{}# the second archived-record correction, proposed differently.\n",
        governed.runbook_with(&body, &ENROLLED, false)
    );
    governed.project.rewrite_runbook(&stale_rule);
    let older_intent = governed.recover_intent(&run, None, "w5-older");
    let older_decision = governed.decision_over(&older_intent, "w5-older");
    governed.project.rewrite_runbook(&second_rule);
    governed.amend(&run, 2, "w5-second");
    // The revert restores the head-1 authority, making the stale decision's
    // old line effective again.
    governed.project.rewrite_runbook(&rule_fix);
    governed.amend(&run, 3, "w5-revert");
    let head_one = sha256_hex(&governed.project.amendment_bytes(&run, 1).unwrap());
    let head_three = sha256_hex(&governed.project.amendment_bytes(&run, 3).unwrap());
    governed.project.rewrite_runbook(&stale_rule);
    let stale_tip = governed.recover(
        &run,
        Some(&older_decision),
        None,
        Some(&format!("recover {run}")),
    );
    assert_eq!(stale_tip.code, 1, "WRS-004: {stale_tip}");
    assert_eq!(
        stale_tip.message,
        format!(
            "recover refused; authorization: chain: observed a decision bound to the head \
             that hashes {head_one}; expected the effective head {head_three}"
        ),
        "WRS-004: a decision bound to an older head refuses naming both heads"
    );
    assert!(
        governed.project.amendment_bytes(&run, 4).is_none(),
        "WRS-004: the stale-tip refusal wrote nothing"
    );

    // A policy-replacing recovery still verifies after a content recovery:
    // the prior trusted policy is the manifest-bound enrolled line the
    // revert restored, and every signer row carries its label's real key.
    let replacement_b = correction(&governed.runbook_with(&body, &REPLACEMENT_B, false));
    governed.project.rewrite_runbook(&replacement_b);
    governed.amend(&run, 4, "w5-fourth");
    let member: toml::Value =
        String::from_utf8_lossy(&governed.project.amendment_bytes(&run, 4).unwrap())
            .parse()
            .expect("the fourth member is TOML");
    let rows = member["signature-identities"]
        .as_array()
        .expect("signature-identities is an array");
    assert_eq!(rows.len(), 1, "the approver is the one signer");
    assert_eq!(rows[0]["label"].as_str(), Some("lead"));
    for row in rows {
        let label = row["label"].as_str().unwrap_or_default();
        assert_eq!(
            row["key"].as_str(),
            Some(governed.keys.public(label)),
            "WRS-004: the signer row for {label} carries its real key"
        );
        assert!(
            row["signature-sha256"]
                .as_str()
                .is_some_and(|sha| !sha.is_empty()),
            "WRS-004: the signature digest for {label} is present"
        );
    }
    let read = rtm(governed.project.root(), &["status", "--run", &run]);
    assert_eq!(
        read.code, 0,
        "WRS-004: every reader accepts the committed head: {read}"
    );
}

// --- WRSV-004-06 ---------------------------------------------------------------

/// WRSV-004-06: a legacy Run lacking enrolled recovery authority refuses despite builder-writable manifests and generated signatures, and independent external enrollment supplies the prerequisite without recovery ever enrolling its own approver.
#[test]
fn wrsv_004_06() {
    let _lane = support::hook_lane();
    let body = machine_body(None, false, false, "build");
    let keys = KeySet::new("wrsv00406", &["a", "b", "lead", "outsider"]);
    let project = Project::new("wrsv00406", &body);
    let run = project.start();
    assert_eq!(project.state(&run), "intake");

    // The aged rule: the legacy Run refuses motion against the drift.
    project.rewrite_runbook(&correction(&project.runbook_text()));
    let aged = rtm(project.root(), &["step", "--run", &run]);
    assert_eq!(aged.code, 1, "WRS-004: {aged}");
    assert!(
        aged.message.contains("runbook pin mismatch"),
        "WRS-004: the legacy Run reproduces the aged-record refusal: {aged}"
    );

    // Without enrolled recovery trust the render and the verb both name
    // the missing prerequisite.
    let legacy_out = keys.dir.join("w6-legacy.bin");
    let bare_intent = rtm(
        project.root(),
        &[
            "intent",
            "--run",
            &run,
            "--recover",
            "--out",
            &shown(&legacy_out),
        ],
    );
    assert_eq!(bare_intent.code, 1, "WRS-004: {bare_intent}");
    assert_eq!(
        bare_intent.message,
        format!(
            "intent refused; run {run} lacks previously enrolled recovery trust; an \
             independently enrolled external authority must supply it (--enrollment <path>): \
             the builder may neither generate it nor substitute task authorization for it"
        ),
        "WRS-004: the ungoverned render names the eligibility prerequisite"
    );
    let generated_anywhere = keys.dir.join("any-decision.sig");
    fs::write(&generated_anywhere, b"a generated signature\n").expect("write the stub decision");
    let bare_recover = rtm(
        project.root(),
        &[
            "recover",
            "--run",
            &run,
            "--decision",
            &shown(&generated_anywhere),
            "--confirm",
            &format!("recover {run}"),
        ],
    );
    assert_eq!(bare_recover.code, 1, "WRS-004: {bare_recover}");
    assert_eq!(
        bare_recover.message,
        format!(
            "recover refused; run {run} lacks previously enrolled recovery trust; an \
             independently enrolled external authority must supply it (--enrollment <path>): \
             the builder may neither generate it nor substitute task authorization for it"
        ),
        "WRS-004: the legacy Run is ineligible without external enrollment"
    );

    // A caller-supplied manifest the builder itself can write refuses
    // through the landed protected-path detail.
    let scope = shown(&fs::canonicalize(resolved_primary(project.root())).unwrap());
    let manifest = build_manifest(&keys, &ENROLLED, &scope);
    let writable_out = keys.dir.join("w6-writable.bin");
    let writable_intent = rtm(
        project.root(),
        &[
            "intent",
            "--run",
            &run,
            "--recover",
            "--enrollment",
            &shown(&manifest.path),
            "--out",
            &shown(&writable_out),
        ],
    );
    assert_eq!(writable_intent.code, 1, "WRS-004: {writable_intent}");
    assert!(
        writable_intent.message.contains(&format!(
            "the enrollment manifest {} is not protected against the builder actor {}",
            shown(&manifest.path),
            builder_actor()
        )),
        "WRS-004: a builder-writable manifest refuses the landed detail: {writable_intent}"
    );

    // The verb refuses the same builder-writable manifest through the
    // landed protected-path detail: the enrollment is read from the path
    // the caller named, before the decision is ever examined, so the stub
    // decision file serves. This probe runs before the seam below, because
    // an installed trust reader is the one route that does not read the
    // caller's path at all.
    let before_protected = project.snapshot();
    let writable_recover = rtm(
        project.root(),
        &[
            "recover",
            "--run",
            &run,
            "--enrollment",
            &shown(&manifest.path),
            "--decision",
            &shown(&generated_anywhere),
            "--confirm",
            &format!("recover {run}"),
        ],
    );
    assert_eq!(writable_recover.code, 1, "WRS-004: {writable_recover}");
    assert!(
        writable_recover.message.contains(&format!(
            "the enrollment manifest {} is not protected against the builder actor {}",
            shown(&manifest.path),
            builder_actor()
        )),
        "WRS-004: ordinary builder access cannot replace the enrollment: {writable_recover}"
    );
    project.assert_unchanged(&before_protected, "builder-writable enrollment recovery");

    // Independent external enrollment supplies the prerequisite: the same
    // manifest bytes through the trust seam render, sign, and recover.
    let _trust = install(
        &project.root().join(".ratmac"),
        TestTrust {
            manifest_bytes: manifest.bytes.clone(),
            signers_path: manifest.signers_path.clone(),
            verifier_path: PathBuf::from(KEYGEN),
            verifier_sha256: sha256_hex(&fs::read(KEYGEN).unwrap()),
        },
    );
    let out = keys.dir.join("w6-recover.bin");
    let rendered = rtm(
        project.root(),
        &[
            "intent",
            "--run",
            &run,
            "--recover",
            "--enrollment",
            &shown(&manifest.path),
            "--out",
            &shown(&out),
        ],
    );
    assert_eq!(
        rendered.code, 0,
        "WRS-004: the externally enrolled render succeeds: {}{}",
        rendered.stdout, rendered.message
    );
    let bytes = fs::read(&out).unwrap();
    assert_eq!(
        rendered.stdout,
        format!(
            "rtm: canonical recover intent for run {run} written to {} (sha256 {})\n",
            shown(&out),
            sha256_hex(&bytes)
        ),
        "WRS-004: the externally enrolled render reports its line"
    );
    let decision = keys.sign("lead", RECOVER_NAMESPACE, &out);

    // The seam's enrolled trust - the same bytes at their real host path -
    // admits the recovery, and the archive records its origin.
    let before = project.snapshot();
    let recovered = rtm(
        project.root(),
        &[
            "recover",
            "--run",
            &run,
            "--enrollment",
            &shown(&manifest.path),
            "--decision",
            &shown(&decision),
            "--confirm",
            &format!("recover {run}"),
        ],
    );
    assert_eq!(
        recovered.code, 0,
        "WRS-004: the enrolled recovery succeeds: {recovered}"
    );
    assert_eq!(recovered.stdout, success_lines(&run, 1));
    assert!(
        project.amendment_bytes(&run, 1).is_some(),
        "WRS-004: the enrolled recovery publishes its amendment"
    );
    let records = project.authorizations(&run);
    let record = records
        .values()
        .find(|text| text.contains("ratmac-recover-v1"))
        .expect("the recovery archive record exists");
    let archive: toml::Value = record.parse().expect("the archive record is TOML");
    assert_eq!(archive["origin"].as_str(), Some("test-injected"));

    // Recovery never enrolls its own approver: the only bytes that changed
    // are the amendment, the archive, and the history log.
    let after = project.snapshot();
    let changed: Vec<&String> = after
        .keys()
        .filter(|path| before.get(*path) != after.get(*path))
        .collect();
    assert!(
        changed.iter().all(|path| {
            path.starts_with(&format!(".ratmac/runs/{run}/")) || *path == ".ratmac/log.md"
        }),
        "WRS-004: the recovery writes no enrollment manifest of its own: {changed:?}"
    );
    assert!(
        !changed
            .iter()
            .any(|path| path.contains("manifest") || path.contains("signers")),
        "WRS-004: no enrollment artifact appeared under the Engine root: {changed:?}"
    );
}

// --- WRSV-004-07 ---------------------------------------------------------------

/// WRSV-004-07: a fully authorized amendment preserves blocked status, hold history, and original pins, consumes no verdict, launders no receipt, and the resume and transition authorization stay separately required.
#[test]
fn wrsv_004_07() {
    let _lane = support::hook_lane();
    let body = machine_body(Some("t-100.md"), false, false, "build");
    let governed = Governed::new("wrsv00407", &body, false);
    let run = governed.admit();

    // The hold and its durable facts.
    let held = governed.hold_governed(&run, "w7-hold");
    assert_eq!(
        held.code, 0,
        "WRS-004: the governed hold succeeds: {}{}",
        held.stdout, held.message
    );
    assert_eq!(governed.project.status(&run), "blocked");
    let pause = governed.project.run_dir(&run).join("pauses/1.toml");
    let pause_before = fs::read(&pause).expect("read the pause record");
    let evidence_before = governed.project.evidence_bytes(&run);
    let record_before = governed.project.record_text(&run);

    // Old receipts for the gate the corrected runbook changes; the
    // amendment must not launder them into passing proof.
    governed.project.write_receipts(&run, "t-100", None);
    let receipts_before = governed.project.receipt_files(&run);

    // The authorized amendment of the held Run.
    governed
        .project
        .rewrite_runbook(&correction(&governed.project.runbook_text()));
    governed.amend(&run, 1, "w7");
    let head = sha256_hex(&governed.project.amendment_bytes(&run, 1).unwrap());

    // The amendment preserves the blocked status, the hold history, the
    // seven fields, every original pin, and every receipt byte.
    assert_eq!(governed.project.status(&run), "blocked");
    assert_eq!(governed.project.state(&run), "intake");
    assert_eq!(
        governed.project.record_value(&run, "blocker"),
        BLOCKER,
        "WRS-004: the amendment drops no blocker"
    );
    governed.project.assert_seven_fields(&run);
    assert_eq!(
        fs::read(&pause).unwrap(),
        pause_before,
        "WRS-004: the amendment preserves the hold history byte-for-byte"
    );
    assert_eq!(
        governed.project.evidence_bytes(&run),
        evidence_before,
        "WRS-004: the amendment keeps the original pins byte-identical"
    );
    assert_eq!(
        governed.project.record_text(&run),
        record_before,
        "WRS-004: the amendment changes no Run Record byte"
    );
    assert_eq!(
        governed.project.receipt_files(&run),
        receipts_before,
        "WRS-004: the amendment creates no passing receipt and touches none"
    );

    // The amendment consumed no verdict.
    assert!(
        !governed
            .project
            .run_dir(&run)
            .join("verdict.toml")
            .is_file(),
        "WRS-004: the amendment consumes no verdict"
    );

    // A separately authorized resume is still required.
    let stepped = rtm(governed.project.root(), &["step", "--run", &run]);
    assert_eq!(stepped.code, 0, "WRS-004: {stepped}");
    assert!(
        stepped.stdout.contains("step refused")
            && stepped
                .stdout
                .contains(&format!("run {run} is paused against")),
        "WRS-004: the amendment resumed nothing: {}",
        stepped.stdout
    );
    assert!(
        stepped
            .stdout
            .contains(&format!("next: rtm step --run {run}")),
        "WRS-004: the held refusal keeps its next line: {}",
        stepped.stdout
    );
    let unverified = governed.resume_governed(&run, None);
    assert_eq!(unverified.code, 1, "WRS-004: {unverified}");
    assert_eq!(
        unverified.message,
        format!(
            "resume refused; authorization: resolution: observed none; expected one \
             ratmac-resume-v1 signature by the designated approver over the canonical intent \
             (rtm intent --run {run} --resume)"
        ),
        "WRS-004: the resume authorization stays separately required"
    );
    let resume_intent = governed.resume_intent(&run, "w7-resume");
    let resolution = governed.keys.sign("lead", RESUME_NAMESPACE, &resume_intent);
    let resumed = governed.resume_governed(&run, Some(&resolution));
    assert_eq!(
        resumed.code, 0,
        "WRS-004: the authorized resume succeeds: {}{}",
        resumed.stdout, resumed.message
    );
    assert_eq!(governed.project.status(&run), "executing");
    assert_eq!(governed.project.state(&run), "intake");

    // Fresh checks under the corrected authority are the only path to new
    // passing receipts: the stale proof of the changed guard refuses.
    governed.step_ok(&run, "w7-enter-build");
    assert_eq!(governed.project.state(&run), "build");
    let stale = governed.step_governed(&run, "w7-stale");
    assert_eq!(stale.code, 0, "WRS-004: {stale}");
    assert!(
        stale.stdout.contains("receipt is stale"),
        "WRS-004: stale proof of a changed guard never counts: {}",
        stale.stdout
    );

    // The corrected rule moves the guard's ticket: fresh proof for the new
    // checks passes under the corrected authority.
    let changed_gate = correction(
        &governed
            .project
            .runbook_text()
            .replace("ticket = \"t-100.md\"", "ticket = \"t-101.md\""),
    );
    governed.project.rewrite_runbook(&changed_gate);
    let gate_intent = governed.recover_intent(&run, None, "w7-gate");
    let gate_decision = governed.decision_over(&gate_intent, "w7-gate");
    let gate_recovered = governed.recover(
        &run,
        Some(&gate_decision),
        None,
        Some(&format!("recover {run}")),
    );
    assert_eq!(
        gate_recovered.code, 0,
        "WRS-004: the second amendment succeeds: {gate_recovered}"
    );
    assert_eq!(gate_recovered.stdout, success_lines(&run, 2));
    let missing = governed.step_governed(&run, "w7-missing");
    assert_eq!(missing.code, 0, "WRS-004: {missing}");
    assert!(
        missing.stdout.contains("no completion receipt for this"),
        "WRS-004: the removed guard's checks count for nothing: {}",
        missing.stdout
    );
    let head_two = sha256_hex(&governed.project.amendment_bytes(&run, 2).unwrap());
    assert_ne!(head_two, head);
    governed
        .project
        .write_receipts(&run, "t-101", Some(&head_two));
    governed.step_ok(&run, "w7-fresh");
    assert_eq!(governed.project.state(&run), "done");
    assert_eq!(governed.project.status(&run), "passed");
}

// --- WRSV-004-08 ---------------------------------------------------------------

/// WRSV-004-08: a two-entry chain parses and each malformed member - missing, unknown, or duplicate fields, missing sequence, a noncanonical filename, a broken predecessor, a changed original-pin digest, mismatched signed bytes, or an unenumerated change - refuses while a preparation file is ignored and motion uses the latest head.
#[test]
fn wrsv_004_08() {
    let _lane = support::hook_lane();
    let body = machine_body(None, false, false, "build");
    let governed = Governed::new("wrsv00408", &body, false);
    let run = governed.admit();

    // The fixture control: governed motion works under the enrolled rule.
    governed.step_ok(&run, "w8-control");
    assert_eq!(governed.project.state(&run), "build");

    // A two-entry chain through two authorized recoveries.
    governed.rewrite(&body, &ENROLLED, false);
    governed
        .project
        .rewrite_runbook(&correction(&governed.project.runbook_text()));
    governed.amend(&run, 1, "w8-first");
    governed.project.rewrite_runbook(&format!(
        "{}# a second archived-record correction\n",
        governed.runbook_with(&body, &ENROLLED, false)
    ));
    let second_intent = governed.recover_intent(&run, None, "w8-second-intent");
    let second_decision = governed.decision_over(&second_intent, "w8-second");
    let second = governed.recover(
        &run,
        Some(&second_decision),
        None,
        Some(&format!("recover {run}")),
    );
    assert_eq!(
        second.code, 0,
        "WRS-004: the second recovery succeeds: {second}"
    );
    assert_eq!(second.stdout, success_lines(&run, 2));
    let evidence_before = governed.project.evidence_bytes(&run);

    let dir = governed.project.amendments_dir(&run);
    let one = dir.join("1.toml");
    let two = dir.join("2.toml");
    let one_bytes = fs::read(&one).unwrap();
    let two_bytes = fs::read(&two).unwrap();
    let head_one = sha256_hex(&one_bytes);
    let two_text = String::from_utf8_lossy(&two_bytes).into_owned();
    let one_text = String::from_utf8_lossy(&one_bytes).into_owned();

    // The chain's own shape: contiguous sequences, each record's sequence
    // equals its filename, and the predecessor digests bind.
    let one_parsed: toml::Value = one_text.parse().expect("member 1 is TOML");
    let two_parsed: toml::Value = two_text.parse().expect("member 2 is TOML");
    assert_eq!(one_parsed["sequence"].as_integer(), Some(1));
    assert_eq!(two_parsed["sequence"].as_integer(), Some(2));
    assert_eq!(
        two_parsed["previous-amendment-digest"].as_str(),
        Some(head_one.as_str()),
        "WRS-004: each later member binds its predecessor's exact bytes"
    );
    assert_eq!(one_parsed["run"].as_str(), Some(run.as_str()));

    // The digests a member binds: recomputed independently from the
    // ticket's own bytes over the record's own fields, the authority
    // digest of the new table and the change-set digest equal the frames
    // its signed intent carries, so a different domain prefix or identity
    // order in the Engine could not hide here.
    {
        let old_digest = ticket_authority_digest(&one_parsed["old-authorities"]);
        let new_digest = ticket_authority_digest(&one_parsed["new-authorities"]);
        let changes: Vec<&str> = one_parsed["changes"]
            .as_array()
            .expect("changes is an array")
            .iter()
            .map(|change| change.as_str().expect("each change is a string"))
            .collect();
        let change_set = ticket_change_set_digest(&changes, &old_digest, &new_digest);
        let fields =
            signed_intent_frames(one_parsed["signed-intent"].as_str().expect("signed-intent"));
        assert_eq!(fields.len(), 24, "the canonical intent frames 24 fields");
        assert_eq!(
            fields[21],
            new_digest.as_bytes(),
            "WRS-004: the intent binds the ticket-recomputed new-authority digest"
        );
        assert_eq!(
            fields[22],
            change_set.as_bytes(),
            "WRS-004: the intent binds the ticket-recomputed change-set digest"
        );
    }

    // The reader baseline: an intact chain validates and the missing
    // decision is all that refuses.
    let intact = governed.recover(&run, None, None, Some(&format!("recover {run}")));
    assert_eq!(intact.code, 1, "WRS-004: {intact}");
    assert!(
        intact.message.starts_with(
            "recover refused; authorization: decision: observed none; expected one \
             ratmac-recover-v1"
        ),
        "WRS-004: the intact chain validates before the authorization boundary: {intact}"
    );

    // One mutation at a time, each restored exactly.
    let restore = |renamed: Option<&str>| {
        if let Some(name) = renamed {
            let _ = fs::remove_file(dir.join(name));
        }
        fs::write(&one, &one_bytes).expect("restore member 1 bytes");
        fs::write(&two, &two_bytes).expect("restore member 2 bytes");
    };
    let reader = |project: &Project, id: &str| {
        let mut args = vec!["recover".to_owned(), "--run".to_owned(), id.to_owned()];
        args.push("--confirm".to_owned());
        args.push(format!("recover {id}"));
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        rtm(project.root(), &refs)
    };

    // A noncanonical member name.
    fs::rename(&two, dir.join("02.toml")).expect("rename member 2 noncanonically");
    let noncanonical = reader(&governed.project, &run);
    assert_eq!(noncanonical.code, 1, "WRS-004: {noncanonical}");
    assert_eq!(
        noncanonical.message,
        "recover refused; amendment sequence 02 is not a positive decimal integer without \
         leading zeroes",
        "WRS-004: a noncanonical member name refuses mirroring the pause family"
    );
    let status_read = rtm(governed.project.root(), &["status", "--run", &run]);
    assert_eq!(status_read.code, 1, "WRS-004: {status_read}");
    assert!(
        status_read.message.contains(
            "amendment sequence 02 is not a positive decimal integer without leading zeroes"
        ),
        "WRS-004: a reader refuses the malformed chain through its own rendering: {status_read}"
    );
    // Doctor reports the malformed chain as an error finding, writing
    // nothing.
    let before_doctor = governed.project.snapshot();
    let doctor_chain = rtm(governed.project.root(), &["doctor"]);
    assert_eq!(
        doctor_chain.code, 2,
        "WRS-004: an error finding exits 2: {doctor_chain}"
    );
    assert!(
        doctor_chain.stdout.contains(&format!(
            "RB903 error: run {run:?}: amendment sequence 02 is not a positive decimal \
             integer without leading zeroes"
        )),
        "WRS-004: doctor's error finding carries the pinned RB903 code, run \
         location, and strict-chain text: {}",
        doctor_chain.stdout
    );
    governed
        .project
        .assert_unchanged(&before_doctor, "doctor read of the malformed chain");
    restore(Some("02.toml"));

    // A stray non-member refuses the same text.
    fs::write(dir.join("notes.txt"), b"stray\n").expect("plant the stray member");
    let stray = reader(&governed.project, &run);
    assert_eq!(stray.code, 1, "WRS-004: {stray}");
    assert_eq!(
        stray.message,
        "recover refused; amendment sequence notes.txt is not a positive decimal integer \
         without leading zeroes",
        "WRS-004: any other stray member refuses the noncanonical text"
    );
    fs::remove_file(dir.join("notes.txt")).expect("remove the stray member");

    // A missing field.
    fs::write(&two, drop_key_line(&two_text, "changes")).expect("drop the changes field");
    let missing_field = reader(&governed.project, &run);
    assert_eq!(missing_field.code, 1, "WRS-004: {missing_field}");
    assert!(
        missing_field.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a missing field refuses the strict family: {missing_field}"
    );
    restore(None);

    // An unknown field.
    fs::write(&two, format!("bogus = 1\n{two_text}")).expect("add the unknown field");
    let unknown_field = reader(&governed.project, &run);
    assert_eq!(unknown_field.code, 1, "WRS-004: {unknown_field}");
    assert!(
        unknown_field.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: an unknown field refuses the strict family: {unknown_field}"
    );
    restore(None);

    // A duplicate field.
    fs::write(&two, format!("version = 1\n{two_text}")).expect("duplicate a field");
    let duplicate_field = reader(&governed.project, &run);
    assert_eq!(duplicate_field.code, 1, "WRS-004: {duplicate_field}");
    assert!(
        duplicate_field.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a duplicate field refuses the strict family: {duplicate_field}"
    );
    restore(None);

    // A missing sequence.
    fs::write(&two, drop_key_line(&two_text, "sequence")).expect("drop the sequence field");
    let no_sequence = reader(&governed.project, &run);
    assert_eq!(no_sequence.code, 1, "WRS-004: {no_sequence}");
    assert!(
        no_sequence.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a missing sequence refuses the strict family: {no_sequence}"
    );
    restore(None);

    // A sequence that differs from its filename.
    fs::write(
        &two,
        replace_key_line(&two_text, "sequence", "sequence = 5"),
    )
    .expect("plant the mismatched sequence");
    let mismatched_sequence = reader(&governed.project, &run);
    assert_eq!(
        mismatched_sequence.code, 1,
        "WRS-004: {mismatched_sequence}"
    );
    assert!(
        mismatched_sequence.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a sequence that differs from its filename refuses: {mismatched_sequence}"
    );
    restore(None);

    // A broken predecessor digest: member 2 still binds member 1's original
    // bytes - record and signed intent agree - but member 1's own bytes no
    // longer hash to that value.
    let drifted_one = format!("{one_text}# drifted\n");
    fs::write(&one, &drifted_one).expect("drift member 1 bytes");
    let broken = reader(&governed.project, &run);
    assert_eq!(broken.code, 1, "WRS-004: {broken}");
    assert_eq!(
        broken.message,
        format!(
            "recover refused; amendment 2 records previous-amendment-digest {head_one}, \
             but sequence 1 hashes {}",
            sha256_hex(drifted_one.as_bytes())
        ),
        "WRS-004: a broken predecessor refuses naming both digests"
    );
    restore(None);

    // A changed original-pin digest: member 1 still records the pin digest
    // it was signed against, but the pin record's own inputs drifted.
    let evidence_path = governed.project.run_dir(&run).join("evidence.toml");
    let mut drifted_pins: toml::Value = String::from_utf8_lossy(&evidence_before)
        .parse()
        .expect("evidence is TOML");
    drifted_pins["engine"]["sha256"] = toml::Value::String("2".repeat(64));
    fs::write(&evidence_path, toml::to_string(&drifted_pins).unwrap())
        .expect("drift the pin record");
    let re_pinned = reader(&governed.project, &run);
    assert_eq!(re_pinned.code, 1, "WRS-004: {re_pinned}");
    assert!(
        re_pinned
            .message
            .starts_with("recover refused; amendment 1 records previous-amendment-digest ")
            && re_pinned
                .message
                .contains(", but the original pin record hashes "),
        "WRS-004: a changed original-pin digest refuses its family: {re_pinned}"
    );
    fs::write(&evidence_path, &evidence_before).expect("restore the pin record");
    restore(None);

    // A member naming another Run.
    fs::write(
        &two,
        replace_key_line(&two_text, "run", "run = \"run-999999\""),
    )
    .expect("plant the foreign run");
    let foreign = reader(&governed.project, &run);
    assert_eq!(foreign.code, 1, "WRS-004: {foreign}");
    assert_eq!(
        foreign.message,
        format!(
            "recover refused; amendment record {} names run run-999999; this chain belongs \
             to run {run}",
            shown(&two)
        ),
        "WRS-004: a member naming another Run refuses"
    );
    restore(None);

    // A signed intent that disagrees with the record's own fields: valid
    // lowercase hex of different bytes.
    let hex = |bytes: &[u8]| -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() };
    let other_intent = hex(b"ratmac-authorization-v1 and other canonical bytes");
    let mismatched_intent_lines: Vec<String> = two_text
        .lines()
        .map(|line| {
            if line.starts_with("signed-intent") {
                format!("signed-intent = \"{other_intent}\"")
            } else {
                line.to_owned()
            }
        })
        .collect();
    fs::write(&two, format!("{}\n", mismatched_intent_lines.join("\n")))
        .expect("plant the mismatched signed intent");
    let mismatched = reader(&governed.project, &run);
    assert_eq!(mismatched.code, 1, "WRS-004: {mismatched}");
    assert!(
        mismatched.message.starts_with(
            "recover refused; amendment 2 binds a signed intent that disagrees with its \
             record fields: "
        ),
        "WRS-004: valid hex of different bytes refuses the disagree family: {mismatched}"
    );
    restore(None);

    // A signed-intent in a spelling other than lowercase hex is a wrong
    // type.
    fs::write(
        &two,
        two_text.replacen("signed-intent = \"", "signed-intent = \"bogus", 1),
    )
    .expect("plant the non-hex spelling");
    let non_hex = reader(&governed.project, &run);
    assert_eq!(non_hex.code, 1, "WRS-004: {non_hex}");
    assert!(
        non_hex.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a non-hex spelling refuses the strict family: {non_hex}"
    );
    restore(None);

    // A non-string changes entry is a wrong type.
    fs::write(
        &two,
        replace_key_line(&two_text, "changes", "changes = [1]"),
    )
    .expect("plant the non-string changes entry");
    let bad_change = reader(&governed.project, &run);
    assert_eq!(bad_change.code, 1, "WRS-004: {bad_change}");
    assert!(
        bad_change.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a non-string changes entry refuses the strict family: {bad_change}"
    );
    restore(None);

    // A non-string signer label is a wrong type.
    fs::write(&two, replace_key_line(&two_text, "label", "label = 7"))
        .expect("plant the non-string signer label");
    let bad_label = reader(&governed.project, &run);
    assert_eq!(bad_label.code, 1, "WRS-004: {bad_label}");
    assert!(
        bad_label.message.starts_with(&format!(
            "recover refused; amendment record {} is not a strict version-1 record: ",
            shown(&two)
        )),
        "WRS-004: a non-string signer label refuses the strict family: {bad_label}"
    );
    restore(None);

    // A record field that disagrees with its own cleanly framed signed
    // intent: the canonical bytes parse, and the kept State the frames
    // carry refuses the record's different spelling.
    fs::write(
        &two,
        replace_key_line(&two_text, "state", "state = \"done\""),
    )
    .expect("plant the disagreeing kept State");
    let kept_state = reader(&governed.project, &run);
    assert_eq!(kept_state.code, 1, "WRS-004: {kept_state}");
    assert_eq!(
        kept_state.message,
        "recover refused; amendment 2 binds a signed intent that disagrees with its record \
         fields: it does not carry the kept State",
        "WRS-004: a record field disagreeing with its frames refuses: {kept_state}"
    );
    restore(None);

    // An authority difference the changes list omits.
    let unenumerated = replace_in_table(&two_text, "new-authorities", "policy", &"0".repeat(64));
    fs::write(&two, &unenumerated).expect("plant the unenumerated difference");
    let beyond = reader(&governed.project, &run);
    assert_eq!(beyond.code, 1, "WRS-004: {beyond}");
    assert_eq!(
        beyond.message,
        "recover refused; amendment 2 changes policy beyond its enumerated change set",
        "WRS-004: a recorded difference the changes list omits refuses"
    );
    restore(None);

    // A listed change whose old and new identities agree.
    let old_runbook = two_parsed["old-authorities"]["runbook"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let agreeing = replace_in_table(&two_text, "new-authorities", "runbook", &old_runbook);
    fs::write(&two, &agreeing).expect("plant the agreeing change");
    let agreed = reader(&governed.project, &run);
    assert_eq!(agreed.code, 1, "WRS-004: {agreed}");
    assert_eq!(
        agreed.message,
        "recover refused; amendment 2 enumerates a change to runbook whose old and new \
         identities agree",
        "WRS-004: a listed change that agrees refuses"
    );
    restore(None);

    // A gap refuses contiguity.
    fs::rename(&two, dir.join("3.toml")).expect("rename member 2 into a gap");
    let gapped = reader(&governed.project, &run);
    assert_eq!(gapped.code, 1, "WRS-004: {gapped}");
    assert!(
        gapped.message.starts_with(&format!(
            "recover refused; the amendment chain for run {run} is not contiguous: "
        )),
        "WRS-004: a gap refuses the contiguity family: {gapped}"
    );
    restore(Some("3.toml"));

    // A preparation file is ignored, never a member.
    fs::write(dir.join("3.prepare"), b"partial publication\n").expect("plant a preparation file");
    let ignored = reader(&governed.project, &run);
    assert_eq!(ignored.code, 1, "WRS-004: {ignored}");
    assert!(
        ignored.message.starts_with(
            "recover refused; authorization: decision: observed none; expected one \
             ratmac-recover-v1"
        ),
        "WRS-004: the preparation file is ignored by every reader: {ignored}"
    );
    let clean_status = rtm(governed.project.root(), &["status", "--run", &run]);
    assert_eq!(clean_status.code, 0, "WRS-004: {clean_status}");

    // Each successful motion uses only the complete latest verified head.
    assert_eq!(
        governed.project.runbook_pin(&run),
        one_parsed["old-authorities"]["runbook"]
            .as_str()
            .unwrap_or_default(),
        "the original pin stays recorded"
    );
    governed.step_ok(&run, "w8-motion");
    assert_eq!(
        governed.project.state(&run),
        "done",
        "WRS-004: build's live edge leads to done"
    );
    assert_eq!(
        governed.project.evidence_bytes(&run),
        evidence_before,
        "WRS-004: motion under the latest head rewrites no pin"
    );
}

// --- WRSV-004-09 ---------------------------------------------------------------

/// WRSV-004-09: source bytes stay identical while an authorized amendment moves the authority head, receipts and approvals bound to the previous head refuse, fresh proof and separately authorized motion bound to the new head pass, and the older evidence keeps its bytes.
#[test]
fn wrsv_004_09() {
    let _lane = support::hook_lane();
    let body = machine_body(Some("t-100.md"), false, true, "build");
    let governed = Governed::new("wrsv00409", &body, false);

    // Run X: an authority amendment that never touches the source bytes the
    // receipts digest.
    let x = governed.admit();
    governed.step_ok(&x, "w9-x-build");
    assert_eq!(governed.project.state(&x), "build");
    governed
        .project
        .rewrite_runbook(&correction(&governed.runbook_with(&body, &ENROLLED, false)));
    governed.amend(&x, 1, "w9-x-first");
    let head_one_x = sha256_hex(&governed.project.amendment_bytes(&x, 1).unwrap());

    // Fresh proof bound to the new head, recorded while the head is 1.
    governed
        .project
        .write_receipts(&x, "t-100", Some(&head_one_x));

    // The authority amendment: the goal revision moves, the source bytes
    // the receipts digest do not.
    let authorization_before = governed.authorization_args(&x, None, "w9-x-old-auth");
    governed.project.write(
        "goal/spec.md",
        "# Goal\n\nThe frozen goal baseline.\nA corrected goal requirement.\n",
    );
    governed.amend(&x, 2, "w9-x-second");
    let head_two_x = sha256_hex(&governed.project.amendment_bytes(&x, 2).unwrap());

    // An approval bound to the previous effective head cannot authorize
    // motion against the newer one.
    let mut aged_args = vec!["step".to_owned(), "--run".to_owned(), x.to_owned()];
    aged_args.extend(authorization_before);
    let aged_refs: Vec<&str> = aged_args.iter().map(String::as_str).collect();
    let aged_approval = rtm(governed.project.root(), &aged_refs);
    assert_eq!(aged_approval.code, 0, "WRS-004: {aged_approval}");
    assert!(
        aged_approval
            .stdout
            .contains("authorization: review: observed a signature at label \"a\""),
        "WRS-004: the older approval observes no verifying signature: {}",
        aged_approval.stdout
    );
    assert!(
        aged_approval
            .stdout
            .contains(&format!("next: rtm step --run {x}")),
        "WRS-004: the refused step keeps its next line: {}",
        aged_approval.stdout
    );

    // A receipt bound to the previous effective head still refuses.
    let stale = governed.step_governed(&x, "w9-x-stale");
    assert_eq!(stale.code, 0, "WRS-004: {stale}");
    assert!(
        stale.stdout.contains(&format!(
            "receipt is stale: it is bound to authority head {head_one_x}, but this Run's \
             effective head is {head_two_x}"
        )),
        "WRS-004: the stale refusal names both heads: {}",
        stale.stdout
    );

    // A malformed authority-head spelling refuses the receipt parse
    // itself: one digit short, and one uppercase letter among sixty-four.
    {
        let focused = governed.project.root().join(format!(
            ".ratmac/evidence/{x}/completion/{}.toml",
            ratmac::completion::check_slug("PT-100-01")
        ));
        let focused_text = fs::read_to_string(&focused).expect("read the focused receipt");
        let replant = |value: &str| {
            let lines: Vec<String> = focused_text
                .lines()
                .map(|line| {
                    if line.starts_with("authority-head-sha256") {
                        format!("authority-head-sha256 = \"{value}\"")
                    } else {
                        line.to_owned()
                    }
                })
                .collect();
            fs::write(&focused, format!("{}\n", lines.join("\n")))
                .expect("replant the focused receipt");
        };
        let short_head = "a".repeat(63);
        replant(&short_head);
        let before_short = governed.project.snapshot();
        let short_head_step = governed.step_governed(&x, "w9-x-short-head");
        assert!(
            short_head_step.stdout.contains(&format!(
                "receipt field \"authority-head-sha256\" must be 64 lowercase hexadecimal \
                 digits when present, found \"{short_head}\""
            )),
            "WRS-004: a 63-digit head refuses the parse: {}",
            short_head_step.stdout
        );
        governed
            .project
            .assert_unchanged(&before_short, "a short head spelling writes nothing");

        let upper_head = format!("{}{}", "A", "a".repeat(63));
        replant(&upper_head);
        let before_upper = governed.project.snapshot();
        let upper_head_step = governed.step_governed(&x, "w9-x-upper-head");
        assert!(
            upper_head_step.stdout.contains(&format!(
                "receipt field \"authority-head-sha256\" must be 64 lowercase hexadecimal \
                 digits when present, found \"{upper_head}\""
            )),
            "WRS-004: an uppercase head letter refuses the parse: {}",
            upper_head_step.stdout
        );
        governed
            .project
            .assert_unchanged(&before_upper, "an uppercase head spelling writes nothing");
    }

    // Fresh proof and separately authorized motion bound to the new head
    // pass.
    governed
        .project
        .write_receipts(&x, "t-100", Some(&head_two_x));
    governed.step_ok(&x, "w9-x-fresh");
    assert_eq!(governed.project.state(&x), "done");
    assert_eq!(governed.project.status(&x), "passed");

    // Run Y: the corrected rule changes the guard's ticket, so the older
    // receipts remain on disk as unchanged historical evidence that no
    // longer counts.
    governed
        .project
        .rewrite_runbook(&governed.runbook_with(&body, &ENROLLED, false));
    let y = governed.admit();
    governed.step_ok(&y, "w9-y-build");
    assert_eq!(governed.project.state(&y), "build");
    governed
        .project
        .rewrite_runbook(&correction(&governed.runbook_with(&body, &ENROLLED, false)));
    governed.amend(&y, 1, "w9-y-first");
    let head_one_y = sha256_hex(&governed.project.amendment_bytes(&y, 1).unwrap());
    let first_amendment_y = governed.project.amendment_bytes(&y, 1).unwrap();
    governed
        .project
        .write_receipts(&y, "t-100", Some(&head_one_y));
    let older = governed.project.receipt_files(&y);

    let changed_gate = correction(
        &governed
            .project
            .runbook_text()
            .replace("ticket = \"t-100.md\"", "ticket = \"t-101.md\""),
    );
    governed.project.rewrite_runbook(&changed_gate);
    governed.amend(&y, 2, "w9-y-second");
    let head_two_y = sha256_hex(&governed.project.amendment_bytes(&y, 2).unwrap());
    assert_ne!(head_one_y, head_two_y);

    let missing = governed.step_governed(&y, "w9-y-missing");
    assert_eq!(missing.code, 0, "WRS-004: {missing}");
    assert!(
        missing.stdout.contains("no completion receipt for this"),
        "WRS-004: the older receipts no longer count: {}",
        missing.stdout
    );

    governed
        .project
        .write_receipts(&y, "t-101", Some(&head_two_y));
    governed.step_ok(&y, "w9-y-fresh");
    assert_eq!(governed.project.status(&y), "passed");

    // The older receipts, original pin bytes, and earlier amendments keep
    // their bytes as unchanged historical evidence.
    let now = governed.project.receipt_files(&y);
    for (name, bytes) in &older {
        if name.starts_with("cargo") {
            continue;
        }
        assert_eq!(
            now.get(name),
            Some(bytes),
            "WRS-004: the older receipt {name} keeps its bytes"
        );
    }
    assert_eq!(
        governed.project.amendment_bytes(&y, 1).unwrap(),
        first_amendment_y,
        "WRS-004: the earlier amendment keeps its bytes"
    );
    assert_eq!(
        governed.project.runbook_pin(&y),
        governed.project.runbook_pin(&x),
        "both Runs keep their recorded original pins"
    );
}
