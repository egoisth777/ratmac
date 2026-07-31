//! AAL-002: authoring starts from something that already works.
//!
//! `rtm scaffold <path>` writes the smallest runbook the doctor accepts with
//! zero findings, and refuses rather than overwriting anything. It is not a
//! template engine: one file, one shape, no options - the author edits from
//! there, guided by `.arca/runbook-authoring.md`.

use std::path::Path;

/// The smallest runbook that is both doctor-clean and runnable: one initial
/// Phase, one terminal Phase, one edge between them, and no guards to argue
/// with. Every schema fact it needs is a pointer, never a restatement.
pub const SCAFFOLD: &str = r#"# A runbook: the Machine Class this project runs.
#
# What may appear here is defined once, in .arca/runbook-spec.md.
# How to grow it - edit, `rtm doctor --json <path>`, repair by code, repeat -
# is in .arca/runbook-authoring.md.
# A branch adds `inputs = ["approve", "rework"]` to its Phase and one matching
# `input = "approve"` or `input = "rework"` to each ordinary transition; the
# exact contract and repairs remain in the two documents above.

[phases.build]
prompt = "Do the work, then report what you produced."

[phases.review]
prompt = "Review the work against the ticket and report the verdict."

[[transitions]]
from = "build"
to = "review"
"#;

/// Why a scaffold was not written. Every variant leaves the tree untouched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScaffoldRefusal {
    /// Something already lives at the requested path.
    Occupied(String),
    /// The requested path's parent directory does not exist. Scaffolding
    /// writes one file and creates no directories, so this is the author's
    /// call to make, not the Engine's.
    NoParent(String),
    /// The write itself failed.
    Unwritable(String, String),
}

impl std::fmt::Display for ScaffoldRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Occupied(path) => write!(
                formatter,
                "scaffold: {path} already exists; scaffolding never overwrites - choose a new path or delete that one yourself"
            ),
            Self::NoParent(path) => write!(
                formatter,
                "scaffold: the directory for {path} does not exist; scaffolding creates exactly one file and no directories"
            ),
            Self::Unwritable(path, error) => {
                write!(formatter, "scaffold: cannot write {path}: {error}")
            }
        }
    }
}

/// Write the scaffold at `path`, or refuse.
///
/// The checks come before the write, so a refusal is a refusal: no partial
/// file, no created directory, nothing to clean up.
pub fn write_scaffold(path: &Path) -> Result<(), ScaffoldRefusal> {
    let shown = path.to_string_lossy().replace('\\', "/");
    if path.exists() {
        return Err(ScaffoldRefusal::Occupied(shown));
    }
    match path.parent() {
        // A bare file name is written into the current directory.
        Some(parent) if parent.as_os_str().is_empty() => {}
        Some(parent) if parent.is_dir() => {}
        Some(_) => return Err(ScaffoldRefusal::NoParent(shown)),
        None => return Err(ScaffoldRefusal::NoParent(shown)),
    }
    std::fs::write(path, SCAFFOLD)
        .map_err(|error| ScaffoldRefusal::Unwritable(shown, error.to_string()))
}
