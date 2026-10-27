# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Release window | The moment while another command releases a lock. On Windows the lock path stays delete-pending until that command's handle closes; opening it then fails with `Access is denied. (os error 5)`, and so can reading its attributes at the moment the handle closes. |
| Resumed claim | The child Engine endpoint's claim on an already admitted child Run's lock, made when the parent continues that child instead of admitting a new one. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
