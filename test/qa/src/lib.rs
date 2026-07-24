#[cfg(test)]
mod tests {
    use ratmac::graph::{MachineGraph, MachineState, Phase, Transition};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TestLifecycleStatus {
        Planned,
        Executing,
        Blocked,
        Passed,
        Failed,
    }

    #[test]
    fn test_machine_state_is_phase_only() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/phase-only-class/ratmac.toml"
        ));
        let document: toml::Value = fixture.parse().expect("phase-only fixture is valid TOML");
        let phases = document
            .get("phases")
            .and_then(toml::Value::as_table)
            .expect("fixture declares phases");
        let transitions = document
            .get("transitions")
            .and_then(toml::Value::as_array)
            .expect("fixture declares transitions");

        let declared_phases: Vec<_> = phases.keys().map(Phase::new).collect();
        let declared_transitions: Vec<_> = transitions
            .iter()
            .map(|transition| {
                let table = transition.as_table().expect("transition is a table");
                Transition::new(
                    table
                        .get("from")
                        .and_then(toml::Value::as_str)
                        .expect("transition has a source phase"),
                    table
                        .get("to")
                        .and_then(toml::Value::as_str)
                        .expect("transition has a target phase"),
                )
            })
            .collect();

        let graph = MachineGraph::new(declared_phases, declared_transitions);
        assert_eq!(graph.phases().count(), 3);
        assert_eq!(graph.transitions().count(), 2);

        let prepare = Phase::new("prepare");
        let review = Phase::new("review");
        let finish = Phase::new("finish");
        let state = MachineState::new(prepare.clone());
        assert_eq!(state.phase(), &prepare);
        assert_eq!(graph.next_phase(state.phase()), Some(&review));
        assert_eq!(graph.next_phase(review.clone()), Some(&finish));
        assert_eq!(graph.next_phase(finish), None);

        // Status is intentionally test-local: it cannot alter state identity or lookup.
        for status in [
            TestLifecycleStatus::Planned,
            TestLifecycleStatus::Executing,
            TestLifecycleStatus::Blocked,
            TestLifecycleStatus::Passed,
            TestLifecycleStatus::Failed,
        ] {
            let _status = status;
            assert_eq!(state.phase(), &prepare);
            assert_eq!(graph.next_phase(state.phase()), Some(&review));
        }
    }

    use std::str::FromStr;

    use ratmac::model::{Run, Status};

    const STATUS_VALUES: [&str; 5] = ["planned", "executing", "blocked", "passed", "failed"];

    #[test]
    fn test_status_is_phase_local_lifecycle_enum() {
        let fixture = include_str!("../../fixtures/status-lifecycle/state.toml");
        let phase_name = fixture
            .lines()
            .find_map(|line| {
                line.strip_prefix("phase = ")
                    .map(|value| value.trim_matches('"'))
            })
            .expect("status fixture must declare a phase");
        let configured_status = fixture
            .lines()
            .find_map(|line| {
                line.strip_prefix("status = ")
                    .map(|value| value.trim_matches('"'))
            })
            .expect("status fixture must declare a status");

        let graph = MachineGraph::new(
            vec![Phase::new("build"), Phase::new("verify")],
            vec![Transition::new("build", "verify")],
        );
        let expected_transition = graph.next_phase(Phase::new(phase_name));
        assert_eq!(configured_status, "planned");

        for raw_status in STATUS_VALUES {
            let status = Status::from_str(raw_status).unwrap_or_else(|error| {
                panic!("{raw_status:?} must be a valid lifecycle status: {error}")
            });
            let run = Run::new(Phase::new(phase_name), status);

            assert_eq!(run.phase(), &Phase::new(phase_name));
            assert_eq!(run.status().to_string(), raw_status);
            assert_eq!(
                graph.next_phase(run.phase().clone()),
                expected_transition,
                "status {raw_status:?} must not affect transition lookup"
            );
        }

        for invalid_status in ["", "Planned", "EXECUTING", "unknown", "blocked "] {
            assert!(
                Status::from_str(invalid_status).is_err(),
                "invalid lifecycle status {invalid_status:?} must be rejected"
            );
        }
    }
    use ratmac::machine::MachineClass;

    const STATUS_DIMENSION_FIXTURE: &str =
        include_str!("../../fixtures/machine-class/status-dimension.toml");

    /// PT-003-01 / T-13: Machine Class declarations have phases and transitions,
    /// but lifecycle status is not a graph dimension.
    #[test]
    fn test_machine_class_rejects_status_dimension() {
        let error = match MachineClass::from_toml(STATUS_DIMENSION_FIXTURE) {
            Ok(_) => panic!("a Machine Class status dimension must be rejected"),
            Err(error) => error,
        };

        let message = error.to_string().to_ascii_lowercase();
        assert!(
            message.contains("status"),
            "status-dimension parse errors must identify the offending key: {message}"
        );

        let phase_transition_class = r#"
            [phases.prepare]
            prompt = "Prepare the inputs."

            [phases.done]
            prompt = "Finish the run."

            [[transitions]]
            from = "prepare"
            to = "done"
        "#;
        let class = MachineClass::from_toml(phase_transition_class)
            .expect("phase/transition-only Machine Classes must be accepted");

        assert_eq!(class.phases().len(), 2);
        assert_eq!(class.transitions().len(), 1);
    }

    use std::fs;
    use std::path::{Path, PathBuf};

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/human-authored-class")
    }

    fn sorted_entries(path: &Path) -> Vec<String> {
        let mut entries = fs::read_dir(path)
            .expect("fixture/project .arca directory should be readable")
            .map(|entry| {
                entry
                    .expect("directory entry should be readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    #[test]
    fn test_ratmac_toml_is_human_authored_reviewed_input() {
        let fixture = fixture_root();
        let source_class = fixture.join(".arca/ratmac.toml");
        let source_bytes = fs::read(&source_class).expect("canonical ratmac fixture should exist");

        let project = std::env::temp_dir().join(format!("ratmac-pt-004-01-{}", std::process::id()));
        if project.exists() {
            fs::remove_dir_all(&project).expect("stale PT-004-01 directory should be removable");
        }
        fs::create_dir_all(project.join(".arca")).expect("temporary project should be creatable");
        let class_path = project.join(".arca/ratmac.toml");
        fs::write(&class_path, &source_bytes).expect("fixture class should be copied verbatim");

        let before_entries = sorted_entries(&project.join(".arca"));
        let before_bytes = fs::read(&class_path).expect("copied class should be readable");
        assert_eq!(
            before_bytes, source_bytes,
            "fixture setup must preserve authored bytes"
        );

        let _class = MachineClass::load_from_project_root(&project)
            .expect("scheduler should consume .arca/ratmac.toml as reviewed input");

        let after_entries = sorted_entries(&project.join(".arca"));
        let after_bytes = fs::read(&class_path).expect("canonical class should remain readable");
        assert_eq!(
            after_bytes, source_bytes,
            "scheduler must not rewrite authored ratmac.toml"
        );
        assert_eq!(
            after_entries, before_entries,
            "loading must not generate class files"
        );

        fs::remove_dir_all(project).expect("PT-004 temporary project should be cleaned up");
    }
    fn t009_fixture_class_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("fixtures")
            .join("run")
            .join("class-read-only")
            .join(".arca")
            .join("ratmac.toml")
    }

    fn t009_copy_fixture() -> (PathBuf, PathBuf) {
        let fixture_class = t009_fixture_class_path();
        assert!(fixture_class.is_file(), "PT-009-01 fixture is missing");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_nanos();
        let project =
            std::env::temp_dir().join(format!("ratmac-pt-009-{}-{}", std::process::id(), nonce));
        let arca = project.join(".arca");
        fs::create_dir_all(&arca).expect("create isolated .arca fixture");
        let class_path = arca.join("ratmac.toml");
        fs::copy(fixture_class, &class_path).expect("copy ratmac.toml fixture");
        (project, class_path)
    }

    #[test]
    fn test_start_reads_class_without_modifying_ratmac() {
        let (project, class_path) = t009_copy_fixture();
        let before = fs::read(&class_path).expect("read ratmac.toml before start");
        ratmac::Scheduler::open(&project)
            .expect("open class-read-only fixture")
            .start()
            .expect("start must load the canonical class");
        let after = fs::read(&class_path).expect("read ratmac.toml after start");
        assert_eq!(before, after, "start must preserve class bytes exactly");
        let text = String::from_utf8(after).expect("ratmac.toml remains UTF-8");
        for key in [
            "phase =",
            "status =",
            "goal_revision =",
            "input_revision =",
            "output_revision =",
            "active_refs =",
            "blocker =",
        ] {
            assert!(
                !text.contains(key),
                "runtime key {key} must stay outside class"
            );
        }
        fs::remove_dir_all(project).expect("clean PT-009 temporary project");
    }
}

#[cfg(test)]
mod t006 {
    use std::fs;
    use std::path::PathBuf;

    use ratmac::graph::{MachineGraph, Phase, Transition};
    use ratmac::model::{Run, Status};
    use ratmac::scheduler::{EntryPrerequisites, Scheduler};

    fn missing_input_revision_fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/entry-prerequisites/missing-input-revision")
    }

    fn present_input_revision_fixture() -> PathBuf {
        let root = std::env::temp_dir().join("ratmac-t006-present-input");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create present input fixture");
        let path = root.join("input_revision");
        fs::write(&path, b"input revision").expect("write present input revision");
        path
    }

    #[test]
    fn test_blocked_only_for_missing_entry_prerequisite() {
        let fixture = missing_input_revision_fixture();
        assert!(
            fixture.is_dir(),
            "missing-input-revision fixture must exist"
        );
        assert!(
            !fixture.join("input_revision").exists(),
            "the fixture intentionally has no input revision"
        );

        // `blocked` is produced by the Scheduler's entry check, not by the
        // Phase graph or by constructing an ordinary lifecycle status.
        let machine = MachineGraph::new(
            vec![Phase::new("plan"), Phase::new("execute")],
            vec![Transition::new("plan", "execute")],
        );
        let mut scheduler = Scheduler::new(machine);
        let run = Run::new(Phase::new("plan"), Status::Planned);
        let blocked = scheduler.evaluate_entry_prerequisites(
            run,
            EntryPrerequisites::new(fixture.join("input_revision")),
        );

        assert_eq!(blocked.status(), &Status::Blocked);
        assert_eq!(blocked.blocker(), Some("input_revision"));
        assert_eq!(blocked.phase(), &Phase::new("plan"));
        assert_eq!(scheduler.machine().phases().count(), 2);
        assert_eq!(scheduler.machine().transitions().count(), 1);
        assert!(
            !scheduler
                .machine()
                .phases()
                .any(|phase| phase == &Phase::new("blocked")),
            "blocked must not become a machine-graph node"
        );

        let present = present_input_revision_fixture();
        // Complete entry input leaves every ordinary lifecycle value alone. This
        // deliberately does not exercise Exit Guard refusal; that behavior belongs
        // to the later guard tickets.
        for status in [
            Status::Planned,
            Status::Executing,
            Status::Passed,
            Status::Failed,
        ] {
            let mut scheduler = Scheduler::new(MachineGraph::new(
                vec![Phase::new("plan"), Phase::new("execute")],
                vec![Transition::new("plan", "execute")],
            ));
            let state = scheduler.evaluate_entry_prerequisites(
                Run::new(Phase::new("plan"), status),
                EntryPrerequisites::new(present.clone()),
            );
            assert_eq!(state.status(), &status);
            assert_eq!(state.blocker(), None);
            assert_eq!(state.phase(), &Phase::new("plan"));
            assert_eq!(scheduler.machine().phases().count(), 2);
        }
        fs::remove_dir_all(present.parent().expect("present fixture has a parent"))
            .expect("remove present input fixture");
    }
}

#[cfg(test)]
mod t007_state_file {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use ratmac::model::{RunState, Status};
    use ratmac::scheduler::Scheduler;

    const VALID_STATE: &str = include_str!("../../fixtures/state/valid-state.toml");
    const BLOCKED_STATE: &str = include_str!("../../fixtures/state/blocked-with-blocker.toml");
    const STATUS_FIXTURE: &str = "../fixtures/state/status-with-pending-guards";

    struct IsolatedProject(PathBuf);

    impl IsolatedProject {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock is before unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("ratmac-t007-{nonce}"));
            fs::create_dir_all(&root).expect("create isolated project root");
            Self(root)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn state_bytes(&self) -> Vec<u8> {
            fs::read(self.0.join(".arca/state.toml")).expect("read Scheduler-owned state")
        }
    }

    impl Drop for IsolatedProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture_state(toml: &str) -> RunState {
        toml::from_str(toml).expect("fixture is a valid State File TOML")
    }

    fn scheduler(project: &IsolatedProject) -> Scheduler {
        Scheduler::open(project.root()).expect("open isolated Scheduler project")
    }

    fn install_machine_class(project: &IsolatedProject) {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(STATUS_FIXTURE);
        fs::create_dir_all(project.root().join(".arca")).expect("create class directory");
        fs::copy(
            fixture_root.join(".arca/ratmac.toml"),
            project.root().join(".arca/ratmac.toml"),
        )
        .expect("copy human-authored Machine Class fixture");
    }

    #[test]
    fn test_state_file_records_blocked_and_blocker_fields() {
        let project = IsolatedProject::new();
        fs::create_dir_all(project.root().join(".arca")).expect("create state directory");
        let mut scheduler = scheduler(&project);
        let expected = fixture_state(BLOCKED_STATE);

        scheduler
            .record_missing_prerequisite(expected.clone(), "input_revision")
            .expect("Scheduler records a missing entry prerequisite");

        let actual = scheduler
            .load_state()
            .expect("load Scheduler-owned State File");
        assert_eq!(actual.phase, expected.phase);
        assert_eq!(actual.goal_revision, expected.goal_revision);
        assert_eq!(actual.input_revision, expected.input_revision);
        assert_eq!(actual.output_revision, expected.output_revision);
        assert_eq!(actual.active_refs, expected.active_refs);
        assert_eq!(actual.status, Status::Blocked);
        assert_eq!(actual.blocker, expected.blocker);
        assert!(actual.blocker.contains("input_revision"));
    }

    #[test]
    fn test_state_file_shape_and_status_are_reportable() {
        let project = IsolatedProject::new();
        install_machine_class(&project);
        let mut scheduler = scheduler(&project);
        let expected = fixture_state(VALID_STATE);

        scheduler
            .initialize_state(expected.clone())
            .expect("Scheduler creates the State File");
        let before = project.state_bytes();
        let report = scheduler.status().expect("read-only status succeeds");
        let after = project.state_bytes();
        let loaded = scheduler
            .load_state()
            .expect("reload Scheduler-owned State File");

        assert_eq!(before, after, "status must not rewrite state.toml");
        assert_eq!(loaded, expected, "serialized state must round-trip");
        assert_eq!(report.state.phase, expected.phase);
        assert_eq!(report.state.status, expected.status);
        assert_eq!(report.state.goal_revision, expected.goal_revision);
        assert_eq!(report.state.input_revision, expected.input_revision);
        assert_eq!(report.state.output_revision, expected.output_revision);
        assert_eq!(report.state.active_refs, expected.active_refs);
        assert_eq!(report.state.blocker, expected.blocker);
        let text = String::from_utf8(before).expect("State File is UTF-8 TOML");
        for field in [
            "phase",
            "status",
            "goal_revision",
            "input_revision",
            "output_revision",
            "active_refs",
            "blocker",
        ] {
            assert!(
                text.contains(&format!("{field} =")),
                "missing serialized field {field}"
            );
        }
    }

    #[test]
    fn test_status_prints_phase_status_blocker_and_pending_guards() {
        let project = IsolatedProject::new();
        install_machine_class(&project);
        let mut scheduler = scheduler(&project);
        let expected = fixture_state(BLOCKED_STATE);

        scheduler
            .record_missing_prerequisite(expected.clone(), "input_revision")
            .expect("Scheduler records a missing entry prerequisite");
        let before = project.state_bytes();
        let report = scheduler.status().expect("read-only status succeeds");
        let after = project.state_bytes();
        let output = report.to_string();

        assert_eq!(before, after, "status must not mutate state.toml");
        assert!(output.contains("Phase: P4"));
        assert!(output.contains("Status: blocked"));
        assert!(output.contains("Blocker:"));
        assert!(output.contains("input_revision"));
        assert!(output.contains("pending guard: files_exact"));
        assert!(output.contains("pending guard: cmd"));
    }
}

#[cfg(test)]
mod t008_state_writer_tests {
    use ratmac::Scheduler;
    use std::collections::hash_map::DefaultHasher;
    use std::fs;
    use std::hash::{Hash, Hasher};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const RATMAC: &str = r#"[phases.p4]
    prompt = "Produce the P4 output."
    "#;

    fn temp_project(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before the Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("ratmac-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join(".arca")).expect("create isolated project");
        root
    }

    fn fixture_bytes() -> Vec<u8> {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/state/immutable-sentinel.toml");
        fs::read(fixture).expect("read PT-008-01 immutable sentinel fixture")
    }

    fn bytes_hash(bytes: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
    }

    fn setup_project(root: &Path, sentinel: &[u8]) -> PathBuf {
        let arca = root.join(".arca");
        fs::write(arca.join("ratmac.toml"), RATMAC).expect("write isolated Machine Class");
        let state = arca.join("state.toml");
        fs::write(&state, sentinel).expect("install sentinel state in isolated project");
        state
    }

    #[test]
    fn test_only_rtm_writes_state_file() {
        let sentinel = fixture_bytes();

        let read_project = temp_project("state-read");
        let read_state = setup_project(&read_project, &sentinel);
        let before_read = fs::read(&read_state).expect("read sentinel before status");
        let before_hash = bytes_hash(&before_read);
        assert_eq!(
            before_read, sentinel,
            "test setup must preserve sentinel bytes"
        );

        // The Scheduler exposes status as a read-only operation. This test intentionally
        // uses no StateStore/raw writer; that writer must remain private to Scheduler code.
        let scheduler = Scheduler::open(&read_project).expect("open isolated Scheduler");
        scheduler.status().expect("read Scheduler status");

        let after_status = fs::read(&read_state).expect("read sentinel after status");
        assert_eq!(bytes_hash(&after_status), before_hash);
        assert_eq!(
            after_status, before_read,
            "read-only Scheduler::status must not rewrite state.toml"
        );
        fs::remove_dir_all(&read_project).expect("remove isolated read project");

        let write_project = temp_project("state-write");
        let write_state = setup_project(&write_project, &sentinel);
        let before_write = fs::read(&write_state).expect("read sentinel before Scheduler mutation");
        let before_write_hash = bytes_hash(&before_write);
        let mut scheduler = Scheduler::open(&write_project).expect("open isolated Scheduler");
        let state = scheduler.load_state().expect("load complete State File");
        scheduler
            .record_missing_prerequisite(state, "sentinel-prerequisite")
            .expect("Scheduler state-changing operation");

        let after_write = fs::read(&write_state).expect("read state after Scheduler mutation");
        assert_ne!(bytes_hash(&after_write), before_write_hash);
        assert_ne!(
            after_write, before_write,
            "only the Scheduler state-changing operation may change state.toml"
        );
        fs::remove_dir_all(&write_project).expect("remove isolated write project");

        // v1 caller identity is policy, not authentication: this test proves the filesystem
        // boundary and does not claim that rtm can authenticate Main-Agent versus subagent.
    }
}

#[cfg(test)]
mod t010 {
    use ratmac::Scheduler;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_project() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("ratmac-pt010-{}-{stamp}", std::process::id()));
        let arca = root.join(".arca");
        fs::create_dir_all(&arca).expect("create isolated PT-010 project");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/run/empty-project/.arca/ratmac.toml");
        fs::copy(fixture, arca.join("ratmac.toml")).expect("copy PT-010 class fixture");
        root
    }

    #[test]
    fn test_start_instantiates_run_owned_state_log_and_lock() {
        let project = setup_project();
        let arca = project.join(".arca");
        let class_path = arca.join("ratmac.toml");
        let before = fs::read(&class_path).expect("read class before start");
        let run = Scheduler::open(&project)
            .expect("open project")
            .start()
            .expect("start must instantiate a Run");
        assert_eq!(
            run.phase().as_str(),
            "prepare",
            "start must choose the unique Phase with no incoming transition"
        );

        assert_eq!(
            run.lock_path().expect("started Run owns lock path"),
            arca.join("schd.lock"),
            "Run must own the flat lock path"
        );
        for artifact in ["state.toml", "log.md"] {
            assert!(
                arca.join(artifact).is_file(),
                "start must create flat .arca/{artifact}"
            );
        }
        assert!(
            !arca.join("schd.lock").exists(),
            "start must release the invocation lock before returning"
        );
        assert_eq!(
            before,
            fs::read(&class_path).expect("read class after start"),
            "start must not modify ratmac.toml"
        );
        fs::remove_dir_all(project).expect("remove isolated PT-010 project");
    }
}

#[cfg(test)]
mod t005 {
    //! Contract tests for strict `ratmac.toml` Machine Class parsing.
    //!
    //! The parser API exercised here is intentionally small: `MachineClass::from_toml`
    //! returns a `Result`, while `phases()` and `transitions()` expose the accepted
    //! graph. Product code is owned by a later build phase; these tests are the P4
    //! proof boundary for R-011.

    use ratmac::machine::MachineClass;
    use ratmac::{Scheduler, StepOutcome, StepRequest};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const UNKNOWN_KEY_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/machine-class/unknown-key.toml"
    ));
    const UNKNOWN_STATUS_KEY_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/machine-class/unknown-status-key.toml"
    ));

    const KNOWN_GUARD_FIELDS_FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/machine-class/known-guard-fields.toml"
    ));

    fn without_top_level_key(source: &str, key: &str) -> String {
        let prefix = format!("{key} =");
        source
            .lines()
            .filter(|line| !line.trim_start().starts_with(&prefix))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn assert_valid_positive_control(source: &str, fixture_key: &str) {
        let valid_source = without_top_level_key(source, fixture_key);
        let class = MachineClass::from_toml(&valid_source).unwrap_or_else(|error| {
            panic!(
                "positive control must parse before testing rejection of `{fixture_key}`: {error}"
            )
        });

        assert_eq!(
            class.phases().len(),
            2,
            "positive control must contain both declared phases"
        );
        assert_eq!(
            class.transitions().len(),
            1,
            "positive control must contain the declared transition"
        );
    }

    #[test]
    fn test_ratmac_unknown_key_is_hard_error() {
        assert_valid_positive_control(UNKNOWN_KEY_FIXTURE, "unexpected_key");

        let error = match MachineClass::from_toml(UNKNOWN_KEY_FIXTURE) {
            Ok(_) => {
                panic!(
                    "a syntactically valid class with an unknown key must be rejected, not ignored"
                )
            }
            Err(error) => error,
        };
        let message = error.to_string();
        let lower = message.to_ascii_lowercase();

        assert!(
            lower.contains("unexpected_key"),
            "unknown-key error must identify `unexpected_key`; got: {message}"
        );
        assert!(
            lower.contains("unknown") || lower.contains("field") || lower.contains("key"),
            "unknown-key error must explain the actionable failure; got: {message}"
        );
    }

    #[test]
    fn test_ratmac_status_dimension_is_unknown_key_for_r011() {
        assert_valid_positive_control(UNKNOWN_STATUS_KEY_FIXTURE, "status");

        let error = match MachineClass::from_toml(UNKNOWN_STATUS_KEY_FIXTURE) {
            Ok(_) => {
                panic!("the status dimension must be rejected rather than becoming graph state")
            }
            Err(error) => error,
        };
        let message = error.to_string();
        let lower = message.to_ascii_lowercase();

        assert!(
            lower.contains("status"),
            "status-dimension error must identify the forbidden `status` key; got: {message}"
        );
        assert!(
            lower.contains("unknown")
                || lower.contains("forbidden")
                || lower.contains("field")
                || lower.contains("key"),
            "status-dimension error must explain the actionable strictness failure; got: {message}"
        );
    }
    #[test]
    fn test_machine_class_accepts_standard_guard_fields() {
        let class = MachineClass::from_toml(KNOWN_GUARD_FIELDS_FIXTURE)
            .expect("standard files_exact/file_contains/command_exit fields must parse");
        assert_eq!(class.phases().len(), 2);
        assert_eq!(class.transitions().len(), 1);
    }
    #[test]
    fn test_machine_class_rejects_empty_or_undeclared_transition_endpoints() {
        let empty = r#"
[phases.prepare]
prompt = "Prepare"

[[transitions]]
from = ""
to = "prepare"
"#;
        let undeclared = r#"
[phases.prepare]
prompt = "Prepare"

[[transitions]]
from = "prepare"
to = "missing"
"#;
        let empty_phase = "[phases.\"\"]\nprompt = \"Empty\"\n";
        let whitespace_phase = "[phases.\"   \"]\nprompt = \"Whitespace\"\n";
        assert!(MachineClass::from_toml(empty).is_err());
        assert!(MachineClass::from_toml(undeclared)
            .expect_err("undeclared transition endpoint must fail")
            .to_string()
            .contains("undeclared"));
        assert!(MachineClass::from_toml(empty_phase).is_err());
        assert!(MachineClass::from_toml(whitespace_phase).is_err());
    }

    fn revalidation_project() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("arca-t005-review-{nonce}"));
        fs::create_dir_all(root.join(".arca")).unwrap();
        fs::write(
            root.join(".arca/ratmac.toml"),
            "[phases.prepare]\nprompt = \"Prepare\"\n",
        )
        .unwrap();
        root
    }

    #[test]
    fn test_open_scheduler_revalidates_class_before_status_and_step() {
        let root = revalidation_project();
        let mut scheduler = Scheduler::open(&root).unwrap();
        scheduler.start().unwrap();
        fs::write(
            root.join(".arca/ratmac.toml"),
            "unknown = true\n[phases.prepare]\nprompt = \"Prepare\"\n",
        )
        .unwrap();
        assert!(scheduler.status().is_err());
        assert!(scheduler.step(StepRequest::new("claim")).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_guard_paths_cannot_escape_project_root() {
        let root = revalidation_project();
        let outside = root.parent().unwrap().join("arca-t005-outside.txt");
        fs::write(&outside, "SECRET").unwrap();
        fs::write(
            root.join(".arca/ratmac.toml"),
            "[phases.prepare]\nprompt = \"Prepare\"\nguards = [{ kind = \"file_contains\", path = \"../arca-t005-outside.txt\", contains = \"SECRET\" }]\n",
        )
        .unwrap();
        let mut scheduler = Scheduler::open(&root).unwrap();
        scheduler.start().unwrap();
        let outcome = scheduler.step(StepRequest::new("claim")).unwrap();
        let StepOutcome::Refused { failures } = outcome else {
            panic!("escaping guard path must be refused")
        };
        assert!(failures[0].observed().contains("escapes"));
        assert!(!failures[0].observed().contains("SECRET"));

        let absolute_path = format!("{outside:?}");
        fs::write(
            root.join(".arca/ratmac.toml"),
            format!(
                "[phases.prepare]\nprompt = \"Prepare\"\nguards = [{{ kind = \"file_contains\", path = {absolute_path}, contains = \"SECRET\" }}]\n"
            ),
        )
        .unwrap();
        let absolute_outcome = scheduler.step(StepRequest::new("claim")).unwrap();
        let StepOutcome::Refused { failures } = absolute_outcome else {
            panic!("absolute guard path must be refused")
        };
        assert!(failures[0].observed().contains("escapes"));
        assert!(!failures[0].observed().contains("SECRET"));
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn test_status_with_active_state_but_missing_class_is_hard_error() {
        let root = revalidation_project();
        fs::remove_file(root.join(".arca/ratmac.toml")).unwrap();
        fs::write(
            root.join(".arca/state.toml"),
            "phase = \"prepare\"\nstatus = \"executing\"\ngoal_revision = \"\"\ninput_revision = \"\"\noutput_revision = \"\"\nactive_refs = []\nblocker = \"\"\n",
        )
        .unwrap();
        let scheduler = Scheduler::open(&root).unwrap();
        assert!(scheduler.status().is_err());
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn test_start_with_missing_class_is_actionable_error() {
        let root = revalidation_project();
        fs::remove_file(root.join(".arca/ratmac.toml")).unwrap();
        let mut scheduler = Scheduler::open(&root).unwrap();
        let error = scheduler
            .start()
            .expect_err("start must reject missing ratmac");
        assert!(error.to_string().to_ascii_lowercase().contains("ratmac"));
        let _ = fs::remove_dir_all(root);
    }
}
