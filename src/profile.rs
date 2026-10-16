//! WCP-007: the distributable Plan-Build profile, shipped as data.
//!
//! `rtm profile <name> <path>` exports one packaged profile as a single
//! folder at a path that does not exist yet. The profile's content lives as
//! tracked files under `profiles/plan-build/` and is embedded into the
//! Engine binary at build time - the way the operator skill's content lives
//! in `src/skill.rs` - so export is offline and deterministic and the
//! tracked files are the one source. The Engine gains no project-specific
//! code from the profile: every cycle rule ships inside the bundle, and
//! this module is generic export plumbing only.
//!
//! Before the first byte the Engine validates the embedded manifest against
//! the embedded file table - format supported, dialect matched, inventory
//! bidirectionally complete, every digest equal - and any disagreement
//! refuses as an engine-content defect: the tracked bytes and the manifest
//! have drifted, which is never a caller's error to repair.
//!
//! The write is atomic in placement, byte-for-byte the skill's discipline:
//! checks before the first byte, the whole bundle built at a hidden
//! `.<component>.rtm-profile-partial` sibling of the target, then one
//! rename into place - so an interrupted write never leaves a half folder
//! at the requested path, only a named leftover beside it, which the next
//! attempt refuses and names for the operator to remove. The bundle carries
//! no Engine stamp, deliberately unlike the skill folder: two exports of
//! the same version contain the same bytes.

use std::path::{Path, PathBuf};

pub use crate::root::entries::write_profile;

/// The suffix of the hidden sibling an in-progress export builds in before
/// the one rename moves it to the caller's path. Fixed per target so an
/// interrupted write is detectable by name (the skill's naming rule).
const PARTIAL_SUFFIX: &str = "rtm-profile-partial";

/// The profile format this Engine exports, and the runbook dialect it
/// reads; both must equal what the embedded manifest declares.
const SUPPORTED_FORMAT: i64 = 1;
const DIALECT: &str = "states-v1";

/// Why a profile bundle was not written. Every variant leaves the tree
/// untouched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileRefusal {
    /// Something already lives at the requested path.
    Occupied(String),
    /// The requested path's parent directory does not exist. The export
    /// creates exactly one folder and no directories, so the parent is the
    /// caller's call to make, not the Engine's.
    NoParent(String),
    /// An interrupted earlier export left its partial sibling behind. The
    /// Engine never deletes what it cannot vouch finished, so the operator
    /// removes the named leftover and runs the command again.
    Leftover(String),
    /// The write itself failed; any partial sibling was removed.
    Unwritable(String, String),
}

impl std::fmt::Display for ProfileRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Occupied(path) => write!(
                formatter,
                "profile: {path} already exists; the profile export never overwrites - choose a new path or delete that one yourself"
            ),
            Self::NoParent(path) => write!(
                formatter,
                "profile: the directory for {path} does not exist; the profile export creates exactly one folder and no directories"
            ),
            Self::Leftover(path) => write!(
                formatter,
                "profile: an interrupted write left {path} behind; no partial folder ever sits at the requested path - remove the leftover yourself and run the command again"
            ),
            Self::Unwritable(path, error) => {
                write!(formatter, "profile: cannot write {path}: {error}")
            }
        }
    }
}

/// How a profile export failed outside the write discipline: the name is
/// not packaged (the caller's error, the handler's exit-2 row), or the
/// embedded manifest disagrees with the embedded bytes (an engine-content
/// defect carrying the operational exit, not the usage exit).
#[derive(Debug)]
pub(crate) enum ExportFault {
    UnknownName(String),
    Drift(String),
    Refusal(ProfileRefusal),
}

/// One packaged profile: its embedded manifest and its artifact table, in
/// the manifest's declared order. The tracked files under
/// `profiles/plan-build/` are the one source; the digests the manifest
/// records are validated over exactly these bytes before any export.
pub(crate) struct Bundle {
    pub(crate) name: &'static str,
    /// The manifest's declared format, validated equal to the supported one
    /// before any export returns.
    pub(crate) format: i64,
    pub(crate) manifest: &'static str,
    /// `(path, role, bytes)` per artifact, in the declared order.
    pub(crate) artifacts: &'static [(&'static str, &'static str, &'static str)],
}

/// The one profile packaged today: the Plan-Build profile (WCP-007).
const PLAN_BUILD: Bundle = Bundle {
    name: "plan-build",
    format: SUPPORTED_FORMAT,
    manifest: include_str!("../profiles/plan-build/manifest.toml"),
    artifacts: &[
        (
            "OPERATING.md",
            "instructions",
            include_str!("../profiles/plan-build/OPERATING.md"),
        ),
        (
            "terms.md",
            "terms",
            include_str!("../profiles/plan-build/terms.md"),
        ),
        (
            "machine-class.toml",
            "machine-class",
            include_str!("../profiles/plan-build/machine-class.toml"),
        ),
        (
            "schemas/issue.toml",
            "schema",
            include_str!("../profiles/plan-build/schemas/issue.toml"),
        ),
        (
            "schemas/residual.toml",
            "schema",
            include_str!("../profiles/plan-build/schemas/residual.toml"),
        ),
        (
            "schemas/ticket.toml",
            "schema",
            include_str!("../profiles/plan-build/schemas/ticket.toml"),
        ),
        (
            "schemas/goal.toml",
            "schema",
            include_str!("../profiles/plan-build/schemas/goal.toml"),
        ),
        (
            "checkers/verify_profile.py",
            "checker",
            include_str!("../profiles/plan-build/checkers/verify_profile.py"),
        ),
        (
            "checkers/verify_issue_binding.py",
            "checker",
            include_str!("../profiles/plan-build/checkers/verify_issue_binding.py"),
        ),
        (
            "blanks/issue/index.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/issue/index.md"),
        ),
        (
            "blanks/issue/spec.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/issue/spec.md"),
        ),
        (
            "blanks/issue/design.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/issue/design.md"),
        ),
        (
            "blanks/issue/test-plan.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/issue/test-plan.md"),
        ),
        (
            "blanks/issue/ubi-lang.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/issue/ubi-lang.md"),
        ),
        (
            "blanks/residual.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/residual.md"),
        ),
        (
            "blanks/ticket.md",
            "blank",
            include_str!("../profiles/plan-build/blanks/ticket.md"),
        ),
    ],
};

/// The names of every packaged profile, in packaging order: the list the
/// unknown-name refusal renders.
pub(crate) const PACKAGED: [&str; 1] = ["plan-build"];

/// The packaged profile `name` names, or none.
pub(crate) fn packaged(name: &str) -> Option<&'static Bundle> {
    (name == PLAN_BUILD.name).then_some(&PLAN_BUILD)
}

/// Validate one manifest text against one artifact table: the check the
/// export runs over the embedded pair before the first byte, and the one
/// entry a doctored pair is observed through. The detail names the artifact
/// and the observed-versus-recorded fact.
pub(crate) fn validate(manifest: &str, artifacts: &[(&str, &str, &str)]) -> Result<(), String> {
    let value: toml::Value = manifest
        .parse()
        .map_err(|error| format!("the manifest does not parse as TOML: {error}"))?;
    let format = value
        .get("profile")
        .and_then(|profile| profile.get("format"))
        .and_then(toml::Value::as_integer);
    if format != Some(SUPPORTED_FORMAT) {
        let shown = format
            .map(|number| number.to_string())
            .unwrap_or_else(|| "none".to_owned());
        return Err(format!(
            "the manifest declares format {shown}, not {SUPPORTED_FORMAT}"
        ));
    }
    let dialect = value
        .get("compatibility")
        .and_then(|table| table.get("runbook"))
        .and_then(toml::Value::as_str);
    if dialect != Some(DIALECT) {
        let shown = dialect.unwrap_or("none");
        return Err(format!(
            "the manifest declares runbook dialect \"{shown}\", not \"{DIALECT}\""
        ));
    }
    let Some(rows) = value.get("artifact").and_then(toml::Value::as_array) else {
        return Err("the manifest lists no artifact rows".to_owned());
    };
    for (index, (row, (path, role, bytes))) in rows.iter().zip(artifacts.iter()).enumerate() {
        let listed = row.get("path").and_then(toml::Value::as_str);
        if listed != Some(*path) {
            let shown = listed.unwrap_or("none");
            return Err(format!(
                "the manifest lists {shown} (row {index}), but the embedded table holds {path}"
            ));
        }
        let recorded_role = row.get("role").and_then(toml::Value::as_str);
        if recorded_role != Some(*role) {
            return Err(format!(
                "the manifest lists {path} with role {}, but the embedded table declares {role}",
                recorded_role.unwrap_or("none")
            ));
        }
        let recorded = row.get("sha256").and_then(toml::Value::as_str);
        let observed = crate::receipt::sha256_text(bytes);
        if recorded != Some(observed.as_str()) {
            return Err(format!(
                "the artifact {path} hashes to {observed}, but the manifest records {}",
                recorded.unwrap_or("none")
            ));
        }
    }
    if rows.len() != artifacts.len() {
        return Err(format!(
            "the manifest rows {} artifacts, but the embedded table holds {}",
            rows.len(),
            artifacts.len()
        ));
    }
    Ok(())
}

/// Export the packaged profile `name` at `path`, or refuse. The manifest
/// self-validation precedes the target judgment and the first byte; the
/// target operation is recorded before the write it permits, so every
/// path-family refusal is observed too.
pub(crate) fn export(name: &str, path: &Path) -> Result<&'static Bundle, ExportFault> {
    let Some(bundle) = packaged(name) else {
        return Err(ExportFault::UnknownName(name.to_owned()));
    };
    validate(bundle.manifest, bundle.artifacts).map_err(ExportFault::Drift)?;
    let shown = crate::root::displayed(path);
    crate::observe::operation("target");
    if path.exists() {
        return Err(ExportFault::Refusal(ProfileRefusal::Occupied(shown)));
    }
    let parent = match path.parent() {
        // A bare folder name is written into the current directory.
        Some(parent) if parent.as_os_str().is_empty() => Path::new("."),
        Some(parent) => parent,
        None => return Err(ExportFault::Refusal(ProfileRefusal::NoParent(shown))),
    };
    if !parent.is_dir() {
        return Err(ExportFault::Refusal(ProfileRefusal::NoParent(shown)));
    }
    let Some(component) = path.file_name() else {
        return Err(ExportFault::Refusal(ProfileRefusal::NoParent(shown)));
    };
    let partial: PathBuf = parent.join(format!(
        ".{}.{}",
        crate::root::component(component),
        PARTIAL_SUFFIX
    ));
    if partial.exists() {
        return Err(ExportFault::Refusal(ProfileRefusal::Leftover(
            crate::root::displayed(&partial),
        )));
    }
    if let Err(error) = write_partial(&partial, bundle) {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(ExportFault::Refusal(ProfileRefusal::Unwritable(
            shown,
            error.to_string(),
        )));
    }
    match std::fs::rename(&partial, path) {
        Ok(()) => Ok(bundle),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&partial);
            Err(ExportFault::Refusal(ProfileRefusal::Unwritable(
                shown,
                error.to_string(),
            )))
        }
    }
}

/// Die mid-write, exactly as an interrupted process dies, when the QA-only
/// fault seam asks for it: after `manifest.toml` lands inside the partial
/// sibling, before any other artifact. Death runs no cleanup, so the
/// leftover the durability lanes judge stays where it was being built.
/// Compiled only with `test-fault-injection`, which the QA harnesses
/// enable; production builds use the no-op stub below.
#[cfg(feature = "test-fault-injection")]
fn abort_mid_write_if_requested(name: &str) {
    if name == "manifest.toml"
        && std::env::var("RATMAC_TEST_PROFILE_FAULT").ok().as_deref() == Some("after-manifest")
    {
        std::process::abort();
    }
}

#[cfg(not(feature = "test-fault-injection"))]
fn abort_mid_write_if_requested(_name: &str) {}

/// Build the complete bundle at `partial`: the manifest first, then every
/// artifact at its final name, so the rename into place exposes the whole
/// bundle at once. The bundle's own subfolders travel inside it; nothing
/// outside the partial sibling is created.
fn write_partial(partial: &Path, bundle: &Bundle) -> std::io::Result<()> {
    std::fs::create_dir(partial)?;
    std::fs::write(partial.join("manifest.toml"), bundle.manifest)?;
    abort_mid_write_if_requested("manifest.toml");
    for (path, _, bytes) in bundle.artifacts {
        let target = partial.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, bytes)?;
        abort_mid_write_if_requested(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate;

    /// A doctored manifest against the real embedded table refuses: the
    /// drift the export guards against names the artifact and both facts.
    #[test]
    fn a_drifted_digest_refuses_naming_the_artifact() {
        let bundle = super::packaged("plan-build").expect("plan-build is packaged");
        let doctored = bundle.manifest.replacen(
            &crate::receipt::sha256_text(bundle.artifacts[0].2),
            "0".repeat(64).as_str(),
            1,
        );
        let error = validate(&doctored, bundle.artifacts).expect_err("the digest drifted");
        assert!(error.contains(bundle.artifacts[0].0), "{error}");
        assert!(error.contains("hashes to"), "{error}");
    }

    /// A doctored pair - the artifact table and the manifest both changed
    /// consistently - still verifies: the check is agreement between the
    /// two embedded sources, never a separately invented truth.
    #[test]
    fn a_consistently_doctored_pair_verifies() {
        let bundle = super::packaged("plan-build").expect("plan-build is packaged");
        let (path, _, bytes) = bundle.artifacts[1];
        let doctored_bytes = format!("{bytes}# one doctored byte\n");
        let doctored_digest = crate::receipt::sha256_text(&doctored_bytes);
        let doctored_table: Vec<(&str, &str, &str)> = bundle
            .artifacts
            .iter()
            .map(|(listed, listed_role, body)| {
                (
                    *listed,
                    *listed_role,
                    if *listed == path {
                        doctored_bytes.as_str()
                    } else {
                        *body
                    },
                )
            })
            .collect();
        let doctored_manifest =
            bundle
                .manifest
                .replacen(&crate::receipt::sha256_text(bytes), &doctored_digest, 1);
        assert!(validate(&doctored_manifest, &doctored_table).is_ok());
        assert!(validate(bundle.manifest, &doctored_table).is_err());
    }

    /// The shipped pair - the embedded manifest over the embedded table -
    /// verifies: the tracked bytes and the manifest agree.
    #[test]
    fn the_shipped_pair_verifies() {
        let bundle = super::packaged("plan-build").expect("plan-build is packaged");
        assert!(validate(bundle.manifest, bundle.artifacts).is_ok());
    }
}
