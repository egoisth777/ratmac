//! Engine-root resolution.
//!
//! The checkout that invokes `rtm` supplies tracked, workflow-authored files;
//! Git worktree metadata supplies the repository-wide runtime root.  A missing
//! or unusable Git executable deliberately falls back to a checkout-local
//! Engine root, so resolution is offline and dependency-free.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The Engine-owned directory at either the primary or invoking checkout root.
pub const ENGINE_DIR: &str = ".ratmac";

/// The Machine Class file name is owned by `MachineClass`, the runbook's one
/// reader; this module only addresses it inside the invoking checkout.
use crate::machine::MachineClass;

/// The context of one resolved project: the two roots relevant to an Engine
/// invocation, and the repository they belong to.
///
/// WEB-005: a resolution is the one discovery of these roots. Its constructor
/// is private to this module, so only the entry points defined here resolve;
/// every other part of the Engine is handed the context and cannot make a
/// second one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Roots {
    invoking_checkout_root: PathBuf,
    /// The invoking checkout exactly as the caller spelled it. Abandonment
    /// names the project and its confirmation phrase from this spelling, as
    /// it did before resolution was centralized; nothing else reads it.
    named_checkout: PathBuf,
    engine_root: PathBuf,
    /// The canonical Git common directory every checkout of this repository
    /// shares; `None` when resolution fell back to the checkout-local Engine
    /// root, which makes the invoking checkout the repository's only member.
    repository: Option<PathBuf>,
}

impl Roots {
    /// Resolve the invoking checkout and its Engine runtime root.
    ///
    /// In a Git worktree, the first non-bare worktree in Git's porcelain
    /// listing supplies the shared Engine root. Any failed or unsuitable Git
    /// query intentionally uses the invoking checkout's own `.ratmac/`
    /// directory instead.
    fn resolve(invoking_checkout_root: impl AsRef<Path>) -> Self {
        let named_checkout = invoking_checkout_root.as_ref().to_path_buf();
        let invoking_checkout_root = absolute(invoking_checkout_root.as_ref());
        #[cfg(feature = "test-fault-injection")]
        let repeated = fault::observe(&invoking_checkout_root);
        let repository = git_repository(&invoking_checkout_root);
        let engine_root = repository
            .as_ref()
            .map_or(&invoking_checkout_root, |(primary, _)| primary)
            .join(ENGINE_DIR);
        #[cfg(feature = "test-fault-injection")]
        let engine_root = repeated.unwrap_or(engine_root);
        Self {
            invoking_checkout_root,
            named_checkout,
            engine_root,
            repository: repository.map(|(_, common)| common),
        }
    }

    /// Whether a canonical workspace is a checkout of the repository these
    /// roots were resolved in. The workspace is judged against that
    /// repository - Git names the repository the workspace belongs to - and
    /// is never resolved itself. Without Git the invoking checkout is the
    /// repository's only member.
    pub(crate) fn holds_workspace(&self, workspace: &Path) -> bool {
        match &self.repository {
            Some(repository) => {
                git_common_dir(workspace).is_some_and(|common| &common == repository)
            }
            None => canonical(workspace) == canonical(&self.invoking_checkout_root),
        }
    }

    /// The checkout from which the command was invoked.
    pub fn invoking_checkout_root(&self) -> &Path {
        &self.invoking_checkout_root
    }

    /// The invoking checkout in the caller's own spelling, used only to name
    /// the project in abandonment phrases and messages.
    pub(crate) fn named_checkout(&self) -> &Path {
        &self.named_checkout
    }

    /// The resolved, potentially primary-checkout, Engine runtime root.
    pub fn engine_root(&self) -> &Path {
        &self.engine_root
    }

    /// The Machine Class path, which is always read from the invoking checkout.
    pub fn machine_class_path(&self) -> PathBuf {
        checkout_machine_class_path(&self.invoking_checkout_root)
    }
}

/// The Machine Class path of a checkout, which needs no resolution: the class
/// is always read from the checkout that carries it.
pub(crate) fn checkout_machine_class_path(checkout: &Path) -> PathBuf {
    absolute(checkout)
        .join(ENGINE_DIR)
        .join(MachineClass::FILE_NAME)
}

/// The context of one CLI invocation. Each distinct project the invocation
/// addresses - its own checkout, a runbook's project, a scaffold's project -
/// is resolved the first time it is needed and never again, so a command that
/// never needs a project resolves nothing.
pub(crate) struct Invocation {
    checkout: PathBuf,
    resolved: std::cell::RefCell<Vec<(String, Roots)>>,
}

impl Invocation {
    fn new(checkout: &Path) -> Self {
        Self {
            checkout: checkout.to_path_buf(),
            resolved: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// The invoking checkout's context.
    pub(crate) fn checkout(&self) -> Roots {
        self.project(&self.checkout.clone())
    }

    /// The context of a project this invocation addresses by path.
    pub(crate) fn project(&self, project: &Path) -> Roots {
        let key = project_key(&absolute(project));
        if let Some((_, roots)) = self.resolved.borrow().iter().find(|(held, _)| *held == key) {
            return roots.clone();
        }
        let roots = Roots::resolve(project);
        self.resolved.borrow_mut().push((key, roots.clone()));
        roots
    }
}

/// A project's identity for one invocation: its canonical path, or the
/// absolute path when it cannot be canonicalized, in the one spelling.
fn project_key(project: &Path) -> String {
    displayed(canonical(project))
}

/// Builds with `test-fault-injection` observe every resolution: the harness
/// counts resolutions per project and can make a second one of the same
/// project answer differently, so a route that resolves twice is caught
/// either by its count or by the divergent answer.
#[cfg(feature = "test-fault-injection")]
mod fault {
    use std::io::Write;
    use std::path::{Path, PathBuf};

    /// Append `project`'s line to `RATMAC_TEST_ROOT_LOG`, and return the
    /// planted Engine root of `RATMAC_TEST_ROOT_REPEAT` when the log already
    /// held that line.
    pub(super) fn observe(project: &Path) -> Option<PathBuf> {
        let log = std::env::var_os("RATMAC_TEST_ROOT_LOG").filter(|log| !log.is_empty())?;
        let line = super::project_key(project);
        let held = std::fs::read_to_string(&log)
            .map(|text| text.lines().any(|held| held == line))
            .unwrap_or(false);
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()))
            .unwrap_or_else(|error| panic!("RATMAC_TEST_ROOT_LOG cannot record {line}: {error}"));
        if !held {
            return None;
        }
        let answer =
            std::env::var_os("RATMAC_TEST_ROOT_REPEAT").filter(|answer| !answer.is_empty())?;
        if answer == "fail" {
            panic!("second resolution of project {line}");
        }
        Some(PathBuf::from(answer))
    }
}

/// The public path-taking library entries (WEB-005). Each keeps its existing
/// path through a re-export from its own module, establishes one context
/// here, and hands it to the context-taking internals.
pub(crate) mod entries {
    use super::{addressed_project_root, displayed, Invocation, Roots};
    use std::io::Write;
    use std::path::Path;

    /// WEB-004: one contract defect carrying a residue refusal. The artifact
    /// is the inspected project's checkout path and the reason is the
    /// refusal itself, so a contract entry answers residue before it reads
    /// any workflow record or judges any request value.
    fn contract_residue_defect(
        project: &Roots,
        error: &crate::state::StateError,
    ) -> crate::contract::ContractDefect {
        crate::contract::ContractDefect {
            artifact: displayed(project.invoking_checkout_root()),
            reason: error.to_string(),
        }
    }

    /// Run the CLI from supplied arguments without spawning a process.
    ///
    /// FDC-004: `status` and `step` act on an existing Run, so `--run <id>`
    /// is always required; a missing value refuses and prints the roster (the
    /// listing of `.ratmac/runs/`) without touching any Run. `start` takes no
    /// run-id: it mints one.
    pub fn run_from<I, S, W>(
        args: I,
        project_root: impl AsRef<Path>,
        writer: &mut W,
    ) -> Result<i32, crate::cli::CliError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
        W: Write,
    {
        let invocation = Invocation::new(project_root.as_ref());
        crate::cli::run(args, &invocation, writer)
    }

    /// Resolve what an abandon request addresses (WRS-007).
    pub fn resolve_target(
        root: &Path,
        run: Option<&str>,
    ) -> Result<crate::abandon::Target, crate::abandon::AbandonRefusal> {
        crate::abandon::resolve_target_in(&Roots::resolve(root), run)
    }

    /// Verify an abandon request without writing anything.
    pub fn plan_abandon(
        root: &Path,
        request: &crate::abandon::AbandonRequest,
    ) -> Result<crate::abandon::AbandonPlan, crate::abandon::AbandonRefusal> {
        crate::abandon::plan_abandon_in(&Roots::resolve(root), request)
    }

    /// Apply a verified abandon plan.
    pub fn apply_abandon(
        root: &Path,
        plan: &crate::abandon::AbandonPlan,
    ) -> Result<(), crate::abandon::AbandonRefusal> {
        crate::abandon::apply_abandon_in(&Roots::resolve(root), plan)
    }

    /// The `p5-blocked` route predicate: verify a hold without writing anything.
    pub fn plan_hold(
        root: &Path,
        request: &crate::blocked::HoldRequest,
    ) -> Result<crate::blocked::HoldPlan, crate::blocked::HoldRefusal> {
        crate::blocked::plan_hold_in(&Roots::resolve(root), request)
    }

    /// Apply a verified hold plan.
    pub fn apply_hold(
        root: &Path,
        plan: &crate::blocked::HoldPlan,
    ) -> Result<(), crate::blocked::HoldRefusal> {
        crate::blocked::apply_hold_in(&Roots::resolve(root), plan)
    }

    /// Diagnose a runbook through the roots selected for its addressed project.
    pub fn diagnose(path: &Path) -> Vec<crate::doctor::Finding> {
        let project = Roots::resolve(addressed_project_root(path));
        crate::doctor::diagnose_in(path, &project)
    }

    /// Write the scaffold at `path`, or refuse.
    pub fn write_scaffold(path: &Path) -> Result<(), crate::scaffold::ScaffoldRefusal> {
        crate::scaffold::write_scaffold_in(path, &Roots::resolve(addressed_project_root(path)))
    }

    /// Write the skill folder at `path`, or refuse.
    pub fn write_skill(path: &Path) -> Result<(), crate::skill::SkillRefusal> {
        crate::skill::write_skill_in(path, &Roots::resolve(addressed_project_root(path)))
    }

    /// PGE-001: verify intake completion over the working tree.
    pub fn gate_intake(workspace: &Path) -> Result<(), Vec<crate::contract::ContractDefect>> {
        let project = Roots::resolve(workspace);
        if let Err(error) = crate::Scheduler::refuse_flat_residue_with_roots(&project) {
            return Err(vec![contract_residue_defect(&project, &error)]);
        }
        crate::contract::gate_intake_in(&project)
    }

    /// PCR-003: classify every work item from the tree alone.
    pub fn work_items(
        workspace: &Path,
    ) -> Result<Vec<crate::contract::WorkItem>, Vec<crate::contract::ContractDefect>> {
        let project = Roots::resolve(workspace);
        if let Err(error) = crate::Scheduler::refuse_flat_residue_with_roots(&project) {
            return Err(vec![contract_residue_defect(&project, &error)]);
        }
        crate::contract::work_items_in(&project)
    }

    /// PGE-002: verify residual and ticket record contracts.
    pub fn gate_records(
        workspace: &Path,
        engine_root: &Path,
        run_id: &str,
    ) -> Result<(), Vec<crate::contract::ContractDefect>> {
        let project = Roots::resolve(workspace);
        if let Err(error) = crate::Scheduler::refuse_flat_residue_with_roots(&project) {
            return Err(vec![contract_residue_defect(&project, &error)]);
        }
        crate::contract::gate_records_in(&project, engine_root, run_id)
    }

    /// Requirements whose mechanizing gate the project's Runbook does not
    /// declare.
    pub fn unproven_mechanization(root: &Path) -> Vec<crate::contract::ContractDefect> {
        let project = Roots::resolve(root);
        if let Err(error) = crate::Scheduler::refuse_flat_residue_with_roots(&project) {
            return vec![contract_residue_defect(&project, &error)];
        }
        crate::contract::unproven_mechanization_in(project.invoking_checkout_root())
    }
}

impl crate::machine::MachineClass {
    /// Load the reviewed Machine Class from the invoking checkout.
    ///
    /// Loading is deliberately read-only: it only reads
    /// `.ratmac/ratmac.toml` from that checkout and never creates or replaces
    /// a class file.
    pub fn load_from_project_root(
        project_root: impl AsRef<Path>,
    ) -> Result<Self, crate::machine::MachineClassParseError> {
        let project = Roots::resolve(project_root);
        // WEB-004: retired-layout residue in the project refuses before the
        // runbook is read, with the loader's own coded refusal shape.
        if let Err(error) = crate::Scheduler::refuse_flat_residue_with_roots(&project) {
            let runbook = checkout_machine_class_path(project.invoking_checkout_root());
            return Err(crate::machine::MachineClassParseError::at(
                error.code().unwrap_or("RB101"),
                runbook.displayed().to_string(),
                error.to_string(),
            ));
        }
        Self::load_from_checkout(project.invoking_checkout_root())
    }
}

impl crate::Scheduler {
    /// Open a project without creating or modifying any scheduler-owned file.
    ///
    /// No run is addressed yet: `start` mints one, and `open_run` binds to an
    /// existing one. State operations refuse until a run is addressed.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, crate::state::StateError> {
        Self::open_with_roots(&Roots::resolve(root))
    }

    /// Open a project addressed at one canonical, minted roster member under
    /// `.ratmac/runs/<run_id>/`.
    pub fn open_run(
        root: impl AsRef<Path>,
        run_id: impl AsRef<str>,
    ) -> Result<Self, crate::state::StateError> {
        Self::open_run_with_roots(&Roots::resolve(root), run_id)
    }

    /// The plural runs directory for an invoking checkout.
    pub fn runs_dir(root: impl AsRef<Path>) -> PathBuf {
        Self::runs_dir_at(Roots::resolve(root).engine_root())
    }

    /// Listing the resolved `.ratmac/runs/` is the roster: direct
    /// run-directory artifacts, sorted. Symlinks are not Run directories and
    /// cannot put a roster member outside the plural residency path.
    pub fn run_roster(root: impl AsRef<Path>) -> Result<Vec<String>, crate::state::StateError> {
        Self::run_roster_with_roots(&Roots::resolve(root))
    }

    /// FDC-003: spawn a child of an addressed parent under the parent's
    /// workspace.
    pub fn spawn_to(
        root: impl AsRef<Path>,
        parent_id: &str,
        spawn_name: &str,
        bindings: &std::collections::BTreeMap<String, String>,
    ) -> Result<String, crate::state::StateError> {
        Self::spawn_to_with_roots(&Roots::resolve(root), parent_id, spawn_name, bindings, None)
    }

    /// Spawn with an optional workspace spelling from the invocation. Keeping
    /// the original `spawn_to` entry point preserves callers that inherit the
    /// parent workspace by default.
    pub fn spawn_to_with_workspace(
        root: impl AsRef<Path>,
        parent_id: &str,
        spawn_name: &str,
        bindings: &std::collections::BTreeMap<String, String>,
        workspace: Option<&Path>,
    ) -> Result<String, crate::state::StateError> {
        Self::spawn_to_with_roots(
            &Roots::resolve(root),
            parent_id,
            spawn_name,
            bindings,
            workspace,
        )
    }

    /// FDC-007/FDC-006: human-confirmed supersession.
    pub fn respawn(
        root: impl AsRef<Path>,
        request: &crate::RespawnRequest,
    ) -> Result<String, crate::state::StateError> {
        Self::respawn_with_roots(&Roots::resolve(root), request)
    }
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Render an Engine path for a report in the one spelling the Engine shows.
///
/// Resolution mixes sources: Git prints checkout paths with forward slashes
/// while `Path::join` and the no-Git fallback use the platform separator, so
/// the same root reaches a report spelled two ways.  Reports are read, diffed,
/// and parsed as JSON, so they carry one spelling; comparison and filesystem
/// access keep using the `Path` itself, never this string.
pub(crate) fn displayed(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().replace('\\', "/")
}

/// One path component - a file name, a stem, an extension - as text.
///
/// A component holds no separator, so there is nothing to normalize; it is
/// named here so that `to_string_lossy` has exactly one home in the Engine and
/// a scan can say so without reading identifiers at call sites.
pub(crate) fn component(component: impl AsRef<std::ffi::OsStr>) -> String {
    component.as_ref().to_string_lossy().into_owned()
}

/// `path.displayed()` at a call site that would otherwise call the standard
/// `Path::display`.
///
/// The Engine names paths in messages everywhere, not only in reports, and a
/// message is read by the same eyes as a report.  A method keeps the one
/// renderer as convenient as the standard one it replaces, so a new call site
/// has no reason to hand-roll a second spelling.
pub(crate) trait Displayed {
    /// This path in the one spelling the Engine shows.
    fn displayed(&self) -> String;
}

impl<T: AsRef<Path> + ?Sized> Displayed for T {
    fn displayed(&self) -> String {
        displayed(self)
    }
}

/// The project that owns a runbook addressed by path.
pub(crate) fn addressed_project_root(path: &Path) -> PathBuf {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let legacy_workflow_dir = crate::scheduler::legacy_workflow_dir();
    if parent.file_name().is_some_and(|name| {
        name == std::ffi::OsStr::new(ENGINE_DIR)
            || name == std::ffi::OsStr::new(legacy_workflow_dir)
    }) {
        parent
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        parent.to_path_buf()
    }
}

pub(crate) fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// The primary checkout and the canonical common Git directory of the
/// repository `invoking_checkout_root` belongs to.
fn git_repository(invoking_checkout_root: &Path) -> Option<(PathBuf, PathBuf)> {
    let inside_work_tree = git_command(invoking_checkout_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .ok()?;
    if !inside_work_tree.status.success()
        || String::from_utf8(inside_work_tree.stdout).ok()?.trim() != "true"
    {
        return None;
    }

    let worktrees = git_command(invoking_checkout_root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .ok()?;
    if !worktrees.status.success() {
        return None;
    }
    let worktrees = String::from_utf8(worktrees.stdout).ok()?;
    // ENS-003 requires one roster and id namespace per repository, so all
    // worktrees share one Engine root. A bare store has no worktree and can
    // never be that root.
    let primary_checkout = worktrees
        .split("\n\n")
        .find(|record| !record.lines().any(|line| line == "bare"))?
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))?;
    let primary_checkout = PathBuf::from(primary_checkout);
    if !primary_checkout.is_absolute() {
        return None;
    }

    let git_dir = git_command(invoking_checkout_root)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !git_dir.status.success() {
        return None;
    }
    let git_dirs = String::from_utf8(git_dir.stdout).ok()?;
    let mut git_dirs = git_dirs.lines();
    let git_dir = PathBuf::from(git_dirs.next()?);
    let common_dir = PathBuf::from(git_dirs.next()?);
    // Git storage is not a worktree. In particular, a separate Git directory
    // can appear in this position, so reject it rather than infer a root.
    if !git_dir.is_absolute() || !common_dir.is_absolute() || primary_checkout == git_dir {
        return None;
    }

    Some((primary_checkout, canonical(&common_dir)))
}

/// The canonical common Git directory of the repository whose work tree holds
/// `directory`. Git storage (inside `.git`) is not a work tree and names no
/// repository here, just as root discovery rejects it. Asking Git which
/// repository holds a directory is not a resolution: no Engine root is chosen
/// and nothing is recorded.
fn git_common_dir(directory: &Path) -> Option<PathBuf> {
    let output = git_command(directory)
        .args([
            "rev-parse",
            "--is-inside-work-tree",
            "--path-format=absolute",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines();
    if lines.next()?.trim() != "true" {
        return None;
    }
    let common_dir = PathBuf::from(lines.next()?);
    common_dir.is_absolute().then(|| canonical(&common_dir))
}

fn git_command(invoking_checkout_root: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(invoking_checkout_root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_WORK_TREE");
    command
}

#[cfg(test)]
mod tests {
    use super::addressed_project_root;
    use std::path::{Path, PathBuf};

    #[test]
    fn addressed_project_root_hoists_engine_directory() {
        assert_eq!(
            addressed_project_root(Path::new("P/.ratmac/ratmac.toml")),
            PathBuf::from("P")
        );
    }

    #[test]
    fn addressed_project_root_uses_runbook_parent() {
        assert_eq!(
            addressed_project_root(Path::new("P/ratmac.toml")),
            PathBuf::from("P")
        );
    }

    #[test]
    fn addressed_project_root_uses_current_directory_for_bare_runbook() {
        assert_eq!(
            addressed_project_root(Path::new("ratmac.toml")),
            PathBuf::from(".")
        );
    }
}
