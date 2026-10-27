# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Build root | The checkout the bootstrap builds in: the project root with no channel or the nightly channel, or the separate checkout standing at the tagged commit (the stable build checkout) with the stable channel. |
| Cargo-resolved output folder | The folder Cargo itself reports as a build root's output (`target_directory` in `cargo metadata`, asked inside the build root) after every redirect has been applied: `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`, a `build.target-dir` setting in a Cargo configuration file, or, for a stable build, the build checkout's own `target` that the bootstrap names. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
