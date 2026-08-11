//! t-084 / SVC-008: the audit carries history by name, and catches a slip.
//!
//! SVCV-009 `the_live_surface_audit_is_enumerated_and_sharp`
//!
//! History is preserved, not skipped: archived bundles, archived tickets,
//! archived gap records, and the append-only history file are named in an
//! enumerated allowlist, each row with a reason. The list cannot rot, and it
//! cannot hide a slip on a live surface.

use ratmac_qa::rebrand::{
    self, Rule, ALLOWLIST, LEGACY_COMMAND, LEGACY_PRODUCT, PRE_CUTOVER_POSITION,
};
use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway copy of the tracked tree, so the negative probes never touch
/// the repository they are auditing.
struct Copy {
    base: PathBuf,
    root: PathBuf,
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

impl Copy {
    /// Copy every audited file, preserving relative paths.
    fn of(source: &Path, label: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "ratmac-t084-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("tree");
        for path in rebrand::collect_files(source) {
            let relative = path
                .strip_prefix(source)
                .expect("audited file lives under the root");
            let destination = root.join(relative);
            let parent = destination.parent().expect("a copied file has a parent");
            fs::create_dir_all(parent).expect("create the copied parent");
            fs::copy(&path, &destination).expect("copy the audited file");
        }
        Copy { base, root }
    }

    fn rules(&self) -> Vec<Rule> {
        rebrand::load_allowlist(&rebrand::allowlist_path(&self.root))
            .expect("the copied allowlist loads")
    }

    fn write_allowlist(&self, rules: &[String]) {
        let path = rebrand::allowlist_path(&self.root);
        let body = rules.join("\n");
        fs::write(path, format!("{body}\n")).expect("rewrite the copied allowlist");
    }

    fn allowlist_lines(&self) -> Vec<String> {
        fs::read_to_string(rebrand::allowlist_path(&self.root))
            .expect("read the copied allowlist")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

#[test]
fn the_live_surface_audit_is_enumerated_and_sharp() {
    let root = rebrand::repo_root();
    let rules = rebrand::load_allowlist(&rebrand::allowlist_path(&root))
        .expect("SVCV-009: the allowlist must load");

    // Every row carries a reason and a token this audit knows.
    assert!(
        !rules.is_empty(),
        "SVCV-009: the allowlist must not be empty"
    );
    for rule in &rules {
        assert!(
            !rule.reason.trim().is_empty(),
            "SVCV-009: every allowlist row states why its file may carry the token: {rule:?}"
        );
        assert!(
            rule.token == LEGACY_PRODUCT
                || rule.token == LEGACY_COMMAND
                || rule.token == PRE_CUTOVER_POSITION
                || rule.token == "both",
            "SVCV-009: every allowlist row names a known token: {rule:?}"
        );
    }

    // History is enumerated, not skipped: each carrier class is named.
    for carrier in [
        ".arca/log.md",
        ".arca/ticket/archive/**",
        ".arca/issue/archive/**",
        ".arca/residual/archive/**",
    ] {
        let listed = rules.iter().any(|rule| {
            rule.pattern == carrier && (rule.token == PRE_CUTOVER_POSITION || rule.token == "both")
        });
        assert!(
            listed,
            "SVCV-009: {carrier} must be an enumerated carrier of the pre-cutover spelling"
        );
    }

    // The tracked tree passes: no live surface carries a retired spelling,
    // and no row has rotted.
    let report = rebrand::audit(&root, &rules);
    assert!(
        report.violations.is_empty(),
        "SVCV-009: unallowlisted retired spellings on live surfaces:\n{}",
        report.violations.join("\n")
    );
    assert!(
        report.stale.is_empty(),
        "SVCV-009: allowlist rows that match nothing:\n{}",
        report.stale.join("\n")
    );

    // Negative probe one: drop the row that carries the history file, and the
    // audit fails naming that file.
    let dropped = Copy::of(&root, "dropped-row");
    let kept: Vec<String> = dropped
        .allowlist_lines()
        .into_iter()
        .filter(|line| !line.starts_with(".arca/log.md\t"))
        .collect();
    dropped.write_allowlist(&kept);
    let after_drop = rebrand::audit(&dropped.root, &dropped.rules());
    assert!(
        after_drop
            .violations
            .iter()
            .any(|violation| violation.starts_with(".arca/log.md:")),
        "SVCV-009: removing the history row must fail the audit naming that file: {:?}",
        after_drop.violations.iter().take(5).collect::<Vec<_>>()
    );

    // Negative probe two: plant the pre-cutover spelling on a live surface,
    // and the audit fails naming that file and line.
    let planted = Copy::of(&root, "planted-slip");
    let live = planted.root.join("README.md");
    let original = fs::read_to_string(&live).expect("the live surface is readable");
    fs::write(
        &live,
        format!("{original}\nThe Run sits in a Phase, not a State.\n"),
    )
    .expect("plant the slip");
    let after_plant = rebrand::audit(&planted.root, &planted.rules());
    let expected_line = original.lines().count() + 2;
    assert!(
        after_plant
            .violations
            .iter()
            .any(|violation| violation.starts_with(&format!("README.md:{expected_line}:"))),
        "SVCV-009: a planted slip must fail the audit naming file and line: {:?}",
        after_plant.violations
    );

    // Negative probe three: a slip can hide in a name as well as in a line,
    // so renaming a live file to spell the retired word fails the audit too.
    let renamed = Copy::of(&root, "renamed-path");
    let carrier = renamed.root.join("src").join("phase_notes.rs");
    fs::write(&carrier, "// nothing incriminating inside\n").expect("plant the named slip");
    let after_rename = rebrand::audit(&renamed.root, &renamed.rules());
    assert!(
        after_rename
            .violations
            .iter()
            .any(|violation| violation.starts_with("src/phase_notes.rs:")),
        "SVCV-009: a path that names the retired spelling must fail the audit: {:?}",
        after_rename.violations
    );

    // The probes never touched the repository they copied.
    assert_eq!(
        rebrand::audit(&root, &rules),
        report,
        "SVCV-009: auditing must leave the tracked tree exactly as it found it"
    );
    assert!(
        rebrand::allowlist_path(&root).ends_with(ALLOWLIST),
        "SVCV-009: one list serves every suite"
    );
}
