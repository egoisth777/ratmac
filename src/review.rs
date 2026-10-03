//! Authorized review transitions (WRA-001).
//!
//! The red landing carries only the declared test-seam surface: no review
//! policy is parsed, verified, or enforced, and the installed trust is never
//! consulted.

/// Test-only trust injection, compiled only with `test-fault-injection`.
#[cfg(feature = "test-fault-injection")]
pub mod test {
    use std::path::{Path, PathBuf};

    /// Fixture trust facts that stand in for an operator-protected enrollment.
    pub struct TestTrust {
        pub manifest_bytes: Vec<u8>,
        pub signers_path: PathBuf,
        pub verifier_path: PathBuf,
        pub verifier_sha256: String,
    }

    /// Uninstalls its trust reader when dropped.
    pub struct TrustGuard {
        _private: (),
    }

    /// Install a trust reader for one Engine root.
    #[must_use]
    pub fn install(engine_root: &Path, trust: TestTrust) -> TrustGuard {
        let _ = (engine_root, trust);
        TrustGuard { _private: () }
    }
}
