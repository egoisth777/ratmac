//! ETB-001: the pinned gate execution boundary.
//!
//! Guard evaluation must not compile, fetch, or rebuild project source. A
//! guard command that runs project-derived logic runs a *pinned gate
//! artifact*: its resolved path and SHA-256 are recorded in Run evidence
//! (`.ratmac/runs/<id>/evidence.toml`) no later than first guard use, and
//! re-verified at every later evaluation. A command that reads no project
//! state (a toolchain probe) is exempt, but only if the Runbook marks it
//! `exempt = true`.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
/// Run-evidence file name, relative to the run's `.ratmac/runs/<id>/` directory.
pub const EVIDENCE_FILE: &str = "evidence.toml";

/// The recorded identity of one executable: where it resolved and what it is.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Identity {
    pub resolved: String,
    pub sha256: String,
    /// Provenance: the commit this binary was built from (ECP-001); absent
    /// on identities recorded before the channel pin existed.
    pub source_commit: Option<String>,
    /// Provenance: `stable` or `nightly` (ECP-001); absent likewise.
    pub channel: Option<String>,
}

impl fmt::Display for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "path={} sha256={}",
            crate::root::displayed(&self.resolved),
            self.sha256
        )
    }
}

/// One recorded gate-artifact pin, keyed by the program the Runbook declares.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GatePin {
    pub program: String,
    pub identity: Identity,
}

/// Run evidence: the Stable Engine pin plus every gate-artifact pin.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Evidence {
    pub engine: Option<Identity>,
    pub gates: Vec<GatePin>,
    /// Goal revision observed at Run start (ETB-003).
    pub goal_baseline: Option<String>,
    /// Goal revision frozen at the intake-completion boundary (ETB-003).
    pub goal_frozen: Option<String>,
    /// FDC-005: SHA-256 of the invoking checkout's `.ratmac/ratmac.toml`
    /// recorded at Run start — the runbook pin is this hash and nothing more,
    /// never a copy.
    pub runbook_sha256: Option<String>,
}

impl Evidence {
    /// Read a run's evidence from its directory, or an empty record when none
    /// exists yet.
    pub fn load(run_dir: &Path) -> Self {
        let path = evidence_path(run_dir);
        let Ok(source) = fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(value) = source.parse::<toml::Value>() else {
            return Self::default();
        };

        let engine = value.get("engine").and_then(identity_from);
        let gates = value
            .get("gate")
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| {
                        let program = entry.get("program")?.as_str()?.to_owned();
                        Some(GatePin {
                            program,
                            identity: identity_from(entry)?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let goal = value.get("goal");
        let goal_field = |name: &str| {
            goal.and_then(|table| table.get(name))
                .and_then(toml::Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
        };
        let runbook_sha256 = value
            .get("runbook")
            .and_then(|table| table.get("sha256"))
            .and_then(toml::Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned);
        Self {
            engine,
            gates,
            goal_baseline: goal_field("baseline"),
            goal_frozen: goal_field("frozen"),
            runbook_sha256,
        }
    }

    /// The pin recorded for `program`, if any.
    pub fn gate(&self, program: &str) -> Option<&Identity> {
        self.gates
            .iter()
            .find(|pin| pin.program == program)
            .map(|pin| &pin.identity)
    }

    /// Record or replace the Stable Engine pin.
    pub fn set_engine(&mut self, identity: Identity) {
        self.engine = Some(identity);
    }

    /// The engine pin's refusal rule (ECP-001): an absent pin records the
    /// observed identity; an equal pin confirms; any difference - path, hash,
    /// or provenance - refuses exactly as an identity mismatch does, and
    /// writes nothing.
    pub fn confirm_engine(&mut self, observed: Identity) -> Result<(), String> {
        match &self.engine {
            None => {
                self.engine = Some(observed);
                Ok(())
            }
            Some(recorded) if *recorded == observed => Ok(()),
            Some(recorded) => Err(format!(
                "engine pin mismatch: recorded {recorded} channel={} source-commit={}; \
                 observed {observed} channel={} source-commit={}",
                recorded.channel.as_deref().unwrap_or("-"),
                recorded.source_commit.as_deref().unwrap_or("-"),
                observed.channel.as_deref().unwrap_or("-"),
                observed.source_commit.as_deref().unwrap_or("-"),
            )),
        }
    }

    /// Record a gate pin for `program`; existing pins are never overwritten
    /// silently, because a differing identity is a refusal, not an update.
    pub fn record_gate(&mut self, program: &str, identity: Identity) {
        if self.gate(program).is_none() {
            self.gates.push(GatePin {
                program: program.to_owned(),
                identity,
            });
        }
    }

    /// Render Run evidence as deterministic TOML.
    pub fn render(&self) -> String {
        let mut text = String::new();
        if let Some(engine) = &self.engine {
            text.push_str("[engine]\n");
            text.push_str(&format!("resolved = {}\n", quote(&engine.resolved)));
            text.push_str(&format!("sha256 = {}\n", quote(&engine.sha256)));
            if let Some(source_commit) = &engine.source_commit {
                text.push_str(&format!("source-commit = {}\n", quote(source_commit)));
            }
            if let Some(channel) = &engine.channel {
                text.push_str(&format!("channel = {}\n", quote(channel)));
            }
        }
        if self.goal_baseline.is_some() || self.goal_frozen.is_some() {
            text.push_str("\n[goal]\n");
            text.push_str(&format!(
                "baseline = {}\n",
                quote(self.goal_baseline.as_deref().unwrap_or_default())
            ));
            text.push_str(&format!(
                "frozen = {}\n",
                quote(self.goal_frozen.as_deref().unwrap_or_default())
            ));
        }
        if let Some(runbook) = &self.runbook_sha256 {
            text.push_str("\n[runbook]\n");
            text.push_str(&format!("sha256 = {}\n", quote(runbook)));
        }
        for pin in &self.gates {
            text.push_str("\n[[gate]]\n");
            text.push_str(&format!("program = {}\n", quote(&pin.program)));
            text.push_str(&format!("resolved = {}\n", quote(&pin.identity.resolved)));
            text.push_str(&format!("sha256 = {}\n", quote(&pin.identity.sha256)));
        }
        text
    }

    /// Persist Run evidence beside the Run Record, inside the run directory.
    pub fn write(&self, run_dir: &Path) -> std::io::Result<()> {
        let path = evidence_path(run_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.render())
    }
}

/// Path of the Run-evidence file inside a run's directory.
pub fn evidence_path(run_dir: &Path) -> PathBuf {
    run_dir.join(EVIDENCE_FILE)
}

/// SHA-256 of a file's bytes, lowercase hex.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// The identity of the running Engine binary.
pub fn engine_identity() -> Option<Identity> {
    let path = std::env::current_exe().ok()?;
    let sha256 = sha256_file(&path).ok()?;
    Some(Identity {
        resolved: crate::root::displayed(&path),
        sha256,
        // Provenance is stamped at build time by the channel-aware bootstrap
        // (ECP-002); a binary built without it honestly reports none.
        source_commit: option_env!("RTM_SOURCE_COMMIT").map(str::to_owned),
        channel: option_env!("RTM_CHANNEL").map(str::to_owned),
    })
}

/// Resolve a declared guard program to a single, unambiguous executable file.
///
/// Relative paths resolve against the project root; a bare name resolves
/// through `PATH`. A directory or a symlink is refused rather than pinned:
/// neither has a stable identity a reviewer can verify.
pub fn resolve_program(root: &Path, program: &str) -> Result<PathBuf, String> {
    let path = locate_program(root, program)?;

    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        format!(
            "gate artifact is unreadable: {} ({error})",
            crate::root::displayed(&path)
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "gate artifact is a symlink, which has no stable identity: {}",
            crate::root::displayed(&path)
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "gate artifact is not a file: {}",
            crate::root::displayed(&path)
        ));
    }
    Ok(path)
}

/// Locate a declared program without demanding a pinnable identity.
///
/// Resolves the path only: a bare name through `PATH`, anything else against
/// the project root. Callers decide what kind of file they accept - pinning
/// demands a stable identity, while PGE-005 only needs to know that a recorded
/// command names something that exists.
pub fn locate_program(root: &Path, program: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(program);
    let path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else if program.contains('/') || program.contains('\\') {
        root.join(candidate)
    } else {
        search_path(program).ok_or_else(|| format!("program not found on PATH: {program}"))?
    };
    Ok(path)
}

/// Locate `program` on `PATH`, honouring Windows executable extensions.
fn search_path(program: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_owned())
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(|extension| extension.to_ascii_lowercase())
            .collect()
    } else {
        Vec::new()
    };

    for directory in std::env::split_paths(&path_var) {
        let direct = directory.join(program);
        if direct.is_file() {
            return Some(direct);
        }
        for extension in &extensions {
            let candidate = directory.join(format!("{program}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Build drivers: a subcommand tells them to produce artifacts from source.
const BUILD_DRIVERS: [&str; 7] = ["cargo", "make", "cmake", "ninja", "msbuild", "go", "npm"];

/// Compilers: an input source file or an output flag means they compile.
const COMPILERS: [&str; 5] = ["rustc", "cc", "gcc", "clang", "cl"];

/// Driver subcommands that produce artifacts or fetch dependencies.
const BUILDING_SUBCOMMANDS: [&str; 9] = [
    "build", "run", "test", "check", "install", "bench", "fetch", "update", "publish",
];

/// Compiler flags that request output rather than information.
const OUTPUT_FLAGS: [&str; 6] = [
    "-o",
    "-c",
    "--emit",
    "--out-dir",
    "--crate-type",
    "--output",
];

/// Source-file extensions a compiler invocation would build.
const SOURCE_EXTENSIONS: [&str; 7] = ["rs", "c", "cc", "cpp", "cxx", "go", "m"];

/// `Some(reason)` when this command would rebuild project source at evaluation
/// time: a build driver told to build, or a compiler given input or output.
pub fn build_invocation_reason(program: &str, args: &[&str]) -> Option<String> {
    let name = Path::new(program)
        .file_stem()
        .map(|stem| crate::root::component(stem).to_ascii_lowercase())
        .unwrap_or_default();
    let lowered: Vec<String> = args.iter().map(|arg| arg.to_ascii_lowercase()).collect();

    let building = if BUILD_DRIVERS.contains(&name.as_str()) {
        lowered.is_empty()
            || lowered
                .iter()
                .any(|arg| BUILDING_SUBCOMMANDS.contains(&arg.as_str()))
    } else if COMPILERS.contains(&name.as_str()) {
        lowered.iter().any(|arg| {
            OUTPUT_FLAGS.contains(&arg.as_str())
                || Path::new(arg)
                    .extension()
                    .map(|extension| {
                        SOURCE_EXTENSIONS.contains(&crate::root::component(extension).as_str())
                    })
                    .unwrap_or(false)
        })
    } else {
        false
    };

    building.then(|| {
        format!(
            "guard would rebuild project source at evaluation time: {name} {}",
            args.join(" ")
        )
    })
}

/// Read one `Identity` from a TOML table.
fn identity_from(value: &toml::Value) -> Option<Identity> {
    let optional = |name: &str| {
        value
            .get(name)
            .and_then(toml::Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    Some(Identity {
        resolved: value.get("resolved")?.as_str()?.to_owned(),
        sha256: value.get("sha256")?.as_str()?.to_owned(),
        source_commit: optional("source-commit"),
        channel: optional("channel"),
    })
}

/// Quote a value as a TOML basic string.
fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
