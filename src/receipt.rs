//! PGE-003: sensitivity receipts and the P4 gate predicate.
//!
//! Test-first completion is evidenced by executable receipts, never by prose.
//! `.arca/evidence/` is agent-writable: agents record one structured receipt
//! per executed check, and the gate resolves every planned test the ticket
//! declares to a receipt that proves the test can fail.
//!
//! Receipt file: `.arca/evidence/<ticket-id>/<planned-test-id>.toml`
//!
//! ```toml
//! planned-test-id = "PT-045-01"
//! ticket-id = "t-045"
//! kind = "baseline-failure"       # or "mutation-kill"
//! command = "cargo test -p ratmac-qa --test t047_receipts sensitivity"
//! working-dir = "."
//! test-file = "test/qa/tests/t047_receipts.rs"
//! test-name = "sensitivity_receipt_required"
//! exit-status = 101
//! output-sha256 = "<sha256 of output>"
//! output = """
//! test sensitivity_receipt_required ... FAILED
//! """
//! ```
//!
//! Every field is checked: a receipt whose digest does not re-derive from its
//! own recorded output, whose sensitivity run exited zero, or whose test
//! target does not exist as a runnable test is not proof.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Agent-writable evidence directory, relative to the project root.
pub const EVIDENCE_DIR: &str = ".arca/evidence";

/// What a receipt proves about a planned test.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sensitivity {
    /// The test failed before the implementation existed.
    BaselineFailure,
    /// A controlled mutation flipped the test from pass to fail.
    MutationKill,
}

impl Sensitivity {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "baseline-failure" => Some(Self::BaselineFailure),
            "mutation-kill" => Some(Self::MutationKill),
            _ => None,
        }
    }
}

impl fmt::Display for Sensitivity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::BaselineFailure => "baseline-failure",
            Self::MutationKill => "mutation-kill",
        };
        formatter.write_str(text)
    }
}

/// One executed check, recorded so a reviewer can re-derive it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub planned_test: String,
    pub ticket: String,
    pub kind: Sensitivity,
    pub command: String,
    pub working_dir: String,
    pub test_file: String,
    pub test_name: String,
    pub exit_status: i64,
    pub output_sha256: String,
    pub output: String,
}

/// Why a receipt, or the absence of one, is not proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptDefect {
    /// The planned test this defect is about, when one can be named.
    pub planned_test: String,
    pub reason: String,
}

impl fmt::Display for ReceiptDefect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.planned_test.is_empty() {
            formatter.write_str(&self.reason)
        } else {
            write!(formatter, "{}: {}", self.planned_test, self.reason)
        }
    }
}

/// SHA-256 of a string's bytes, lowercase hex.
pub fn sha256_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// The directory holding one ticket's receipts.
pub fn ticket_evidence_dir(root: &Path, ticket: &str) -> PathBuf {
    root.join(EVIDENCE_DIR).join(ticket)
}

/// Read one receipt file, refusing anything that is not self-consistent.
pub fn load_receipt(path: &Path) -> Result<Receipt, ReceiptDefect> {
    let named = |planned_test: &str, reason: String| ReceiptDefect {
        planned_test: planned_test.to_owned(),
        reason,
    };
    let anonymous = |reason: String| named("", reason);

    let source = fs::read_to_string(path)
        .map_err(|error| anonymous(format!("unreadable receipt {}: {error}", shown(path))))?;
    let document: toml::Value = source.parse().map_err(|error| {
        anonymous(format!(
            "receipt {} is not valid TOML: {error}",
            shown(path)
        ))
    })?;

    let text = |key: &str| {
        document
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let planned_test = text("planned-test-id")
        .ok_or_else(|| anonymous(format!("receipt {} names no planned test", shown(path))))?;
    let field = |key: &str| {
        text(key).ok_or_else(|| named(&planned_test, format!("receipt is missing {key:?}")))
    };

    let kind_text = field("kind")?;
    let kind = Sensitivity::parse(&kind_text).ok_or_else(|| {
        named(
            &planned_test,
            format!(
                "unknown sensitivity kind {kind_text:?}; expected baseline-failure or mutation-kill"
            ),
        )
    })?;
    let exit_status = document
        .get("exit-status")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| {
            named(
                &planned_test,
                "receipt is missing \"exit-status\"".to_owned(),
            )
        })?;
    let output = field("output")?;
    let output_sha256 = field("output-sha256")?;
    let command = field("command")?;
    let working_dir = field("working-dir")?;
    let test_file = field("test-file")?;
    let test_name = field("test-name")?;

    let ticket = field("ticket-id")?;
    Ok(Receipt {
        planned_test,
        ticket,
        kind,
        command,
        working_dir,
        test_file,
        test_name,
        exit_status,
        output_sha256,
        output,
    })
}

/// Check one receipt against the repository it claims to be about.
pub fn verify_receipt(root: &Path, receipt: &Receipt) -> Result<(), ReceiptDefect> {
    let defect = |reason: String| ReceiptDefect {
        planned_test: receipt.planned_test.clone(),
        reason,
    };

    let recomputed = sha256_text(&receipt.output);
    if recomputed != receipt.output_sha256 {
        return Err(defect(format!(
            "digest does not re-derive from the recorded output: observed {recomputed}, recorded {}",
            receipt.output_sha256
        )));
    }
    if receipt.exit_status == 0 {
        return Err(defect(format!(
            "{} receipt records a passing run (exit 0), which proves no sensitivity",
            receipt.kind
        )));
    }
    if receipt.command.trim().is_empty() {
        return Err(defect("receipt records no command".to_owned()));
    }

    let test_path = root.join(&receipt.test_file);
    let source = fs::read_to_string(&test_path).map_err(|error| {
        defect(format!(
            "test file {} is unreadable, so the planned test is not runnable: {error}",
            receipt.test_file
        ))
    })?;
    if !source.contains(&format!("fn {}(", receipt.test_name)) {
        return Err(defect(format!(
            "test {} does not exist in {}",
            receipt.test_name, receipt.test_file
        )));
    }
    Ok(())
}

/// The planned tests a ticket declares, in declaration order.
pub fn planned_tests(ticket_source: &str) -> Vec<String> {
    let mut tests = Vec::new();
    let mut inside = false;
    for line in ticket_source.lines() {
        if line.starts_with("planned-test-refs:") {
            inside = true;
            continue;
        }
        if inside {
            let trimmed = line.trim();
            if let Some(entry) = trimmed.strip_prefix("- ") {
                tests.push(entry.trim().trim_matches('"').to_owned());
            } else {
                break;
            }
        }
    }
    tests
}

/// The P4 gate predicate: every planned test the ticket declares resolves to a
/// receipt that proves it can fail, and every receipt filed under the ticket
/// belongs to a planned test the ticket declares.
///
/// Prose satisfies nothing: only `.toml` receipts are read, and a planned test
/// with no receipt is refused by name.
pub fn gate_sensitivity(root: &Path, ticket_relative: &str) -> Result<(), Vec<ReceiptDefect>> {
    let ticket_path = root.join(ticket_relative);
    let Ok(ticket_source) = fs::read_to_string(&ticket_path) else {
        return Err(vec![ReceiptDefect {
            planned_test: String::new(),
            reason: format!("ticket {ticket_relative} is unreadable"),
        }]);
    };
    let ticket_id = Path::new(ticket_relative)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let declared = planned_tests(&ticket_source);
    let mut defects = Vec::new();
    if declared.is_empty() {
        defects.push(ReceiptDefect {
            planned_test: String::new(),
            reason: format!("ticket {ticket_id} declares no planned tests"),
        });
    }

    let dir = ticket_evidence_dir(root, &ticket_id);
    let mut receipts = Vec::new();
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for file in &files {
        match load_receipt(file) {
            Ok(receipt) => receipts.push(receipt),
            Err(defect) => defects.push(defect),
        }
    }

    for planned in &declared {
        match receipts
            .iter()
            .find(|receipt| &receipt.planned_test == planned)
        {
            None => defects.push(ReceiptDefect {
                planned_test: planned.clone(),
                reason: format!(
                    "no sensitivity receipt in {}/{ticket_id}; prose and file names are not evidence",
                    EVIDENCE_DIR
                ),
            }),
            Some(receipt) => {
                if let Err(defect) = verify_receipt(root, receipt) {
                    defects.push(defect);
                }
            }
        }
    }

    for receipt in &receipts {
        if !declared.contains(&receipt.planned_test) {
            defects.push(ReceiptDefect {
                planned_test: receipt.planned_test.clone(),
                reason: format!(
                    "receipt names a planned test that ticket {ticket_id} does not declare"
                ),
            });
        }
        if receipt.ticket != ticket_id {
            defects.push(ReceiptDefect {
                planned_test: receipt.planned_test.clone(),
                reason: format!(
                    "receipt is filed under {ticket_id} but claims ticket {}",
                    receipt.ticket
                ),
            });
        }
    }

    if defects.is_empty() {
        Ok(())
    } else {
        Err(defects)
    }
}

fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
