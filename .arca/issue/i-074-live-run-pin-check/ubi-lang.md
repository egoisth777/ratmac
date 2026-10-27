# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Live Run | A Run whose record (`run.toml` in its folder under the shared Engine root's `runs/`) has a line that, trimmed, reads `status = "planned"` or `status = "executing"`, as the doctor reads it. A record that cannot be read is not live. |
| Pinned file | The Engine file a live Run's pin names: the `resolved` path in the `[engine]` table of that Run's `evidence.toml`, written by the Engine that started the Run. The resolved Engine is the pinned file when both paths, made absolute and written with `/`, are equal under the platform's path rules (letter case ignored on Windows). A pinned path that no longer exists is never the resolved Engine. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
