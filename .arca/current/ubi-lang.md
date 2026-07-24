# ubi-lang — ratmac

Glossary of ubiquitous language. One term, one meaning. Terms not listed here must not be used in docs, code, or CLI output.

| Term | Definition |
|---|---|
| Machine | The state machine as a whole — Phases, transitions, Exit Guards — pure data declared by a Machine Class, never run by agents; an agent sees only its Phase Prompt, never the graph (ADR-0009). |
| Machine Class | The state-machine definition in `ratmac.toml`. Data, not code. A template: declares Phases, transitions, Exit Guards. Human-written and reviewed, never agent-authored. |
| `ratmac.toml` | The Machine Class file, TOML (ADR-0004), at `.arca/ratmac.toml` (ADR-0008). One per project. |
| Run | A live instance of a Machine Class, created by `rtm start` (class vs instantiation, ADR-0005). Each Run owns its State File and Transition Log. |
| Scheduler | The generic engine — Rust CLI, binary `rtm`. Sole writer of State Files. Holds no project-specific knowledge. |
| Phase | A node of the Machine where agent work happens. The ONLY dimension of machine state (ADR-0001). Always say "Phase", never "state", for machine nodes. |
| Status | Phase-local lifecycle (`planned\|executing\|blocked\|passed\|failed`) recorded by the Scheduler. Not part of the Machine graph (ADR-0001). |
| Exit Guard | A predicate over the working tree, evaluated by the Scheduler at `rtm step`. Checks artifacts — filesystem shape (`files_exact`), file content (`yaml_field`), or a command's exit code (`cmd`) — never agent claims. Passing ALL of a Phase's Exit Guards is the only way to leave it. |
| State File | `.arca/state.toml` (ADR-0008). Per-Run machine-readable current state. Written ONLY by the Scheduler; all agents read, never write (ADR-0003). |
| Transition Log | `.arca/log.md` (ADR-0008). Per-Run append-only record of every transition the Scheduler performs. |
| Phase Prompt | What an agent receives for a Phase: inline prose from `ratmac.toml` + the Scheduler-generated Exit Guard list (ADR-0009). The ONLY machine information ever shown to an agent. |
| Main-Agent | The orchestrating agent in the main checkout. May invoke `rtm step`; changes state only through the Scheduler (ADR-0003). |
| Subagent | A worker agent in a ticket worktree. Reads state; never invokes `rtm` (ADR-0003). |
| `rtm start` | Instantiate a Run. User-only; loop entry is never agent-initiated or suggested. |
| `rtm step` | Transition request for a Run (replaces the handoff's `next`). Requesting is not deciding (ADR-0002); Exit Guards decide. A refused `step` changes nothing (ADR-0006). |
| `rtm status` | Read-only report of a Run's Phase, Status, and pending guards. |
| Legacy identity | The superseded spellings `arca-scheduler` and `schd`, retained only in the historical allowlist or explicit migration records. |
| Clean cutover | `ratmac` and `rtm` are the only active product and command spellings; no compatibility alias or package fallback is shipped. |
| External repository identity | The GitHub slug, canonical `origin` URL, checkout directory, `.git` metadata, and active repository-facing links that identify this project outside its Rust code. |
| Canonical repository | The GitHub repository `egoisth777/ratmac`; the superseded `egoisth777/arca-scheduler` slug is not canonical after cutover. |
| Canonical origin | The exact SSH URL `git@github.com:egoisth777/ratmac.git` recorded for the local `origin` remote. |
| Checkout basename | The final local directory name `ratmac`, verified from the checkout's actual path and Git top-level rather than from a display-only label. |
| Historical allowlist | Append-only `.arca/log.md`, archived issue/ticket records, and explicit migration evidence where legacy external identity may remain unchanged as provenance. |
| Safe external cutover | A preflighted, ordered repository rename, origin update, and checkout move with checkpoints and reversible recovery that never discards work, force-pushes, or bypasses Git arbitration. |
