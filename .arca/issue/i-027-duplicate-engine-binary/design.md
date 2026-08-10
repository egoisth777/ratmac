# Issue design

## Proposed mechanics

### 1. Give the test copy its own name

`test/qa/Cargo.toml` declares a target named `rtm` pointing at `../../src/bin/rtm.rs`,
the same file the root package ships as `rtm`. Renaming the test package's target — the
name only, not the source path — removes the collision, and the tests then name that
target when they launch the engine. The shipped command is unaffected: it is still built
by the root package and still called `rtm`.

The rename touches every test that starts the engine, but mechanically and in one way,
so the change is large in line count and small in meaning.

### 2. Refuse a future collision

Two candidates, and the ruling belongs to whoever takes the ticket:

- A check that reads every package manifest in the repository and fails by name when two
  targets resolve to one output path. Cheap, and it reads the declaration rather than the
  build.
- Treating cargo's own collision warning as an error in the shop's build entry point. Cheaper
  still, but it depends on a warning the toolchain may reword.

The first is recommended: it states the rule in the shop's own words and does not move when
the toolchain does.

### Rejected: turn the pause points on everywhere

Enabling the test-only build option in the shipping package would make both copies identical
and the collision harmless. It is refused: the pause points read environment variables and
stop the engine mid-write. That belongs in a test build and nowhere near a shipped command.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted
forward authority.
