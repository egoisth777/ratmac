# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Test Engine | The Engine binary `rtm-qa` that tests run: built from the same source as the shipped `rtm`, with test pause points compiled in. |
| Build report | What Cargo prints on standard output about one build when asked with `--message-format=json`: one record per line, where the `compiler-artifact` record of a binary names the file it built in its `executable` field and a `compiler-message` record carries a compiler error's text in its `rendered` field. |
| Redirected build output | Build output that Cargo puts somewhere other than the repository's `target/` folder because `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR`, a `build.target-dir` setting in a Cargo configuration file, or a build target named by `CARGO_BUILD_TARGET` says so. |
| Fixed-folder path | A binary path a test makes by joining a folder name such as `target/debug` instead of asking Cargo where the build put the binary. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
