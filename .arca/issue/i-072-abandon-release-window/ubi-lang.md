# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Release window | The moment while another command releases a lock. On Windows the lock path stays delete-pending until that command's handle closes; opening it then fails with `Access is denied. (os error 5)`, and so can reading its attributes at the moment the handle closes. |
| Lock-free lock check | Abandon's read of whether the root lock and the addressed Run's lock paths exist, taken while holding no lock. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
