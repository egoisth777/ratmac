# Issue test plan

## Verification

| Check | Requirement Refs | Expected evidence |
| :--- | :--- | :--- |
| `DEBV-001` | `DEB-001` | A clean build of the whole repository prints no output filename collision, and no two build targets resolve to one output path. |
| `DEBV-002` | `DEB-001` | The binary the tests launch carries the pause-point wiring: the blocked-route check that holds the engine before it writes Run state reaches its barrier and passes, from a whole-repository test run, not only from a test-package run. |
| `DEBV-003` | `DEB-001` | The shipped command keeps its name and its identity: the bootstrap entry point still resolves a command called `rtm`, and the pin check still reports one path and one hash. |
| `DEBV-004` | `DEB-002` | Declaring a second target that would write an existing output path fails by name, and the failure names both declarations. Removing the duplicate makes it pass again. |

## Integration traces

| Trace | Where it lands |
| :--- | :--- |
| The one-command evidence rule | The working rules' statement of how a turn's suite is run — today the gap record [res-122](../../residual/res-122.md) has to name a package-scoped command instead. |
| The pause-point boundary | The existing barrier checks in `test/qa/tests/t050_blocked_route.rs`, which are the direct witnesses of the defect. |
