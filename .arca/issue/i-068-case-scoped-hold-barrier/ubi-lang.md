# Ubiquitous language
## Terms
| Term | Meaning |
| :--- | :--- |
| Test hook | A test-only behavior that an Engine or tool built for testing switches on when a `RATMAC_TEST_*` setting names it, such as a pause point or an injected fault. |
| Hold test barrier | The test hook that stops `rtm hold`, `rtm resume`, or a command finishing a pending pause (such as `rtm step` or `rtm abandon`) at a named boundary until its release file appears or its timeout passes, set by the `RATMAC_TEST_HOLD_BARRIER` settings. |
| Arming case | The test case that set a test hook for its own commands. |

Existing project terms follow [the dictionary](../../dict.md) and [goal language](../../goal/ubi-lang.md).
