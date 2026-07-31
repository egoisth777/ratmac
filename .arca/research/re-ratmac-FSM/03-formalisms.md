# Formal Models for Per-Subtask Machines

**Date:** 2026-07-28
**Method:** Theory survey. Direct reads of primary and reference sources on statecharts, State Chart Extensible Markup Language (SCXML), the actor model and Open Telecom Platform supervision, pushdown and recursive state machines, Petri nets and workflow nets, durable execution engines, and process calculi — including van der Aalst's own workflow-net paper and the 2022 result that settles soundness complexity. Read against ratmac's `.arca/steering.md`, `.arca/runbook-spec.md`, and the engine source in `src/`.
**Scope:** Formalisms and their trade-offs, not products. The question is which model gives each subtask its own machine instance, plus real fan-out and fan-in, at the least cost to offline analysis.
**Confidence note:** Three widely repeated claims turned out to be wrong and are corrected in place. (1) Workflow-net soundness is EXPSPACE-*complete*, proved in 2022 — not "EXPSPACE-hard", which no one proved and which is usually mis-cited to a paper about 1-safe nets (section 4). (2) Unrestricted propositional hierarchical task network planning is *undecidable*, not EXPSPACE-complete; that bound belongs to a restricted row of the table (section 3). (3) The guarantees of multiparty session types are communication safety, progress, and session fidelity — "deadlock freedom" is a narrower paraphrase the authors do not use (section 6).

Two searches returned nothing and are reported as *unverified absence* rather than negative results: no dedicated paper on statically verifying Open Telecom Platform supervision trees, and no dedicated SCXML model checker. Where a claim rests on a secondary source rather than the paper itself, the source list says so.

---

## VERDICT: block-structured parallelism over an artifact-derived instance set

Take the **workflow-net** answer to fan-out and fan-in, restricted to **well-handled blocks** so that soundness is a parsing result rather than a search. Take from the **actor model** exactly one relation — a parent owns its children's lifecycle — and refuse its mailbox. Take from **durable execution** exactly one idea — position is derived from an append-only journal rather than stored as a mutable field — and refuse its server. Refuse **statechart** orthogonal regions with cross-region transitions, refuse **history states**, refuse a general **stack**, and refuse **dynamic spawn**.

The concrete shape: one new phase kind that fans out over a finite instance set read from disk, runs one declared sub-machine per instance, and joins when every branch reaches its own terminal phase. The branch is sealed — no edge crosses its boundary in either direction. This is Amazon States Language's `Parallel`/`Map` shape, and it is a Well-handled net with Regular Iteration, which is sound by construction.

Position stops being one field. It becomes a bounded tree: the top-level phase name, plus one phase name per open branch, keyed by the branch's identity. That is the price, and it is unavoidable — a join means several places are occupied at once, and no formalism sells you a real barrier while keeping a single position. What survives is the property that actually pays for `rtm doctor`: **verifying a position stays a membership test with no search**, and every check the doctor runs today stays decidable in time linear in the runbook.

Two published results carry the whole design, and both are about *interface width rather than expressive power*. A single-entry component keeps reachability, cycle detection, and linear-time properties **linear even under unbounded recursion**; and restricting a workflow net to the well-structured class drops soundness from EXPSPACE-complete to **polynomial**. Meanwhile the one thing that must not happen — concurrency combined with unbounded recursion — is outright **undecidable**, which is why nesting depth is bounded by declaration rather than left open. The recommendation is therefore not "use a weaker formalism to be safe"; it is "narrow two interfaces and collect a complexity-class refund".

The strongest argument against is at the end, and it is serious: ratmac may already have all of this for free, because a git worktree is its own project root, and every path in the engine is root-relative.

---

## What ratmac is actually protecting

Three properties are load-bearing. Every formalism below is scored against them, because "does it fit ratmac" means "which of these three does it break".

**P-1 — Position is cheap to verify.** Today, runtime position is one string in `.arca/state.toml`. Answering "is this a legal position" is one set-membership test against the runbook's declared phase names. No search, no reachability computation, no history.

**P-2 — Offline analysis is total.** `rtm doctor` decides *every* property it reports without running anything: it parses through the one reader in `machine.rs`, then checks entry uniqueness, reachability, duplicate and self edges, guard field validity per kind, and prompt ownership. All of it is a walk over a finite graph, linear or near-linear in the runbook's size, and it always terminates with a verdict. That totality is what lets a finding be a *stable code with a repair* rather than a "could not determine". `.arca/steering.md` names this: refusals are branchable, and an agent repairs by diagnostic code.

**P-3 — The process is data with no expression language.** A runbook declares phases and transitions and nothing else. Guards are a closed vocabulary of kinds with typed fields (`files_exact`, `file_contains`, `command_exit`, `sensitivity_receipts`, `completion_gate`, `intake_contract`, `record_contract`). No edge carries a condition. There is no interpolation. This is why `RB106` and `RB107` — unknown guard kind, wrong field for the kind — are decidable at all: the vocabulary is finite, so the parser can be exhaustive.

Two stated non-goals also bind: ratmac is **not a scheduler or task queue**, and it is **deterministic and offline** — no network, no installs, no hidden global state.

### The repo already has two graphs, and only one of them is the runbook

This is the most important thing found in the source, and it reframes the whole question.

1. **The runbook graph** — phases and transitions in `.arca/ratmac.toml`. Small, fixed, human-authored, statically linted. `MachineGraph::transition_for` takes the first ordinary edge out of the current phase, so a phase has exactly one automatic destination.

2. **The work graph** — tickets on disk. `src/contract.rs` already reads every `.arca/ticket/*.md`, pulls a `dependencies:` list out of each ticket's front matter, builds a directed graph, and **rejects cycles** (`find_cycles`). It also enforces that every gap record is owned by exactly one ticket.

So ratmac already computes and validates a directed acyclic work graph. It just never *executes* it. The runbook walks phases; the work graph is only a contract the record gate checks.

The owner's pressure — each subtask gets its own machine instance, composable into a graph with parallel execution — is a request about the **work graph**, not the runbook graph. Recognising that is what makes the recommendation cheap: the runbook graph can stay exactly as flat and as lintable as it is today.

### The ask decomposes into three separable pieces

They are routinely treated as one. They are not, and they have very different formal costs.

| Ask | What it really demands | Formal cost |
| :--- | :--- | :--- |
| **(a) Per-subtask isolation** | Each subtask has private state nobody else writes | Near zero. This is a naming and file-layout question, not a semantics question. |
| **(b) Fan-out and fan-in** | Several branches active at once; a barrier that cannot be crossed until all arrive | Real. This is the one that costs P-1, and no formalism avoids it. |
| **(c) Recursive self-improvement** | The process definition is revised from results | Zero, *if kept outside the machine*. Catastrophic if built into it. |

(a) and (c) are cheap. Only (b) is expensive. A design that pays the (b) price to buy (a) and (c) has overpaid by a wide margin.

---

## 1. Harel statecharts, and SCXML as its concrete spec

David Harel's 1987 paper *Statecharts: A Visual Formalism for Complex Systems* adds three things to a flat state machine: **depth** (a state may contain a sub-machine — an exclusive-or state), **orthogonality** (a state may contain several regions that are all active at once — an and state), and **broadcast communication** between regions. History states were added so a region can be re-entered where it left off.

### What hierarchy buys

The honest, unglamorous answer: **edge factoring**, which the Unified Modeling Language (UML) literature calls *programming by difference*. Nesting is named there as "the most important innovation of UML state machines over the traditional FSMs": when a machine is in a substate it is implicitly in the enclosing superstate too, and an event the substate does not handle is not discarded — it propagates outward and is handled at the enclosing level. So a substate need only declare how it *differs* from its parent.

In a flat machine, "from any of these twelve phases, on failure, go to cleanup" is twelve edges. Wrap the twelve in a superstate and it is one edge from the superstate. The saving is linear in the number of inner states, and it is real — ratmac would feel it immediately, because the blocked route is declared per phase today, and every phase that wants one needs its own `[[transitions]]` entry with `blocked-route = true`. The standard motivating example is exactly this shape: nesting avoids "repeating the transitions Clear and Off in virtually every state".

Crucially, **hierarchy alone changes the file, not the semantics.** A machine with superstates and no orthogonal regions and no history flattens mechanically into an equivalent flat machine. Every check `rtm doctor` runs today survives, because it can run on the flattened form. Depth is the one statechart feature that is nearly free.

The big saving people cite belongs to **orthogonality**, not depth, and the accurate statement of it is *additive declaration against multiplicative configuration space*: with fully independent regions the number of states you must declare is k + l + m + …, while the equivalent flat machine needs the Cartesian product k × l × m × …. That is the compression that makes statecharts worth the trouble in embedded control — and it is also where the costs live.

### What it costs in static analysis — the exact answer

This question has a precise published answer, and it is the single most useful table in this survey. Alur's *Formal Analysis of Hierarchical State Machines* gives the complexity of reachability as a cube over three independent features — hierarchy, concurrency, and finite-domain variables:

| Features present | Reachability |
| :--- | :--- |
| Plain state machines | NLOGSPACE |
| + hierarchy | **PTIME** |
| + concurrency | PSPACE |
| + finite-domain variables | PSPACE |
| hierarchy + variables | PSPACE |
| concurrency + variables | PSPACE |
| **hierarchy + concurrency** | **EXPSPACE** |
| all three | EXPSPACE |

(The bounds assume all variables are global.)

Read the first two rows together and the verdict on depth is settled: **hierarchy alone costs one complexity class, from NLOGSPACE to PTIME, and the concrete algorithms stay linear in the machine's size.** Alur's Theorem 1 gives reachability in `O(|K|)` for a hierarchical machine, Theorem 3 gives cycle detection in `O(|K|)`, and Theorem 7 shows that linear-time-temporal-logic model checking is PSPACE-complete with hardness *inherited from flat* structures — that is, **hierarchy makes linear-temporal-logic checking no harder at all.**

Branching-time logic is the exception and shows where the real cost hides: checking a computation-tree-logic property is exponential in `d`, the maximum number of *exit* nodes of a substructure, and Alur's Theorems 10 and 11 prove that exponent unavoidable. His own explanation is the transferable lesson: linear-time properties need only *local* checking, which "can be solved efficiently by our algorithm that avoids repeated analysis of a shared substructure", whereas branching-time properties need the global variant repeatedly, "which requires splitting of a substructure because the satisfaction of formulas can vary from context to context."

**Everything `rtm doctor` checks today is reachability and cycle detection — the linear column.** So depth is not merely cheap in principle; it is cheap for exactly the checks this project runs.

The last row is the warning. Hierarchy and concurrency *together* cost two exponentials, not one. That is not additive, and it is the reason the recommendation seals its parallel blocks rather than letting hierarchy and concurrency interact freely.

### What it costs in static analysis — the rest

**First, position stops being a label.** With orthogonal regions, runtime position is a *configuration*: a set of active states, one per active region, consistent with the hierarchy (exactly one child of each active exclusive-or state, every child of each active and state). Checking a configuration is well-formed is still cheap — linear in the state tree — so P-1 degrades rather than dies. But "the current state" is no longer a thing you can print.

**Second, history states add memory that the graph does not constrain.** A shallow or deep history pseudostate stores which sub-state a region was in when it was last left. That value is not derivable from the current configuration; it must be persisted separately, per region. A static analyser can check that a stored history value names a state inside the right region, and nothing more — *any* prior state is legal. So history turns "position" into "position plus a side table of remembered positions", and every property that quantifies over positions now quantifies over that side table too. This is the first feature in this survey that genuinely hurts P-1 for no compensating gain in ratmac's setting, since ratmac never re-enters a region it left.

**Third, and worst: statecharts do not have a semantics, they have a family of them.** This is the well-documented central problem of the formalism. The differences are not cosmetic — they change which transition fires. Recurring points of divergence:

- **Transition priority.** When an inner state and an enclosing superstate both have an enabled transition on the same event, which wins? UML resolves innermost-first — "the substate takes precedence over the superstate". The STATEMATE tradition resolves outermost-first. Same diagram, different run.
- **Guard evaluation order.** UML permits several transitions on one trigger provided their guards do not overlap, and then **"intentionally does not stipulate any particular order"** of evaluation, making non-overlap the designer's obligation rather than the tool's check. An unchecked obligation is exactly what ratmac's guard vocabulary exists to eliminate.
- **Run-to-completion versus a synchronous step.** Whether a step processes one event to quiescence or takes a fixed-length macro-step with all enabled transitions firing together. Run-to-completion is near-universal but its cost is stated plainly in the literature: responsiveness "is determined by its longest RTC step".
- **Event lifetime.** Whether a broadcast event is visible in the same step that generated it or only in the next.
- **Inter-level transitions.** Whether an edge may cross a region boundary — jump from inside one orthogonal region to a state inside another. Allowing it is what breaks compositional reasoning outright, because a region is then no longer analysable on its own.

For ratmac, this is disqualifying at the level of the **Ideal shape** property "authored, not imitated": an agent writes a runbook from the written schema. A format whose meaning depends on which of a family of semantics the reader assumes cannot be learned from a specification, because the specification would have to pick one and then defend the pick forever. `.arca/runbook-spec.md` is currently short enough to hold the entire truth. A statechart specification is not.

### SCXML

State Chart Extensible Markup Language is the World Wide Web Consortium's document form of a statechart, and it is the right thing to read as the concrete artefact: `<state>`, `<parallel>`, `<history>`, `<transition>`, `<onentry>`/`<onexit>`, `<datamodel>`, `<send>`, `<invoke>`.

Two of its elements answer the owner's pressure directly, and both cost P-2 and P-3:

- **`<parallel>`** is the orthogonal region. It gives fan-out. It does not give a *barrier*: a `<parallel>` state is left when its transition fires, and expressing "leave only when every region has finished" is done with the `done.state.<id>` event convention, not with a structural join. So the fan-in is by protocol, not by construction — which means a static checker cannot prove the join is correct, only that the event names match.
- **`<invoke>`** starts a child session — genuinely "each subtask gets its own machine instance", including an external SCXML document, with results returned as events. But the invocation is *dynamic*: the child session's identity is created at runtime, so the set of live machines is not in the document. Static topology is gone.

And the deeper problem for ratmac: `<transition cond="...">` holds an expression in a data-model language — the ECMAScript profile is the normative one. **Guards become arbitrary code.** That is P-3 deleted.

This is not an SCXML quirk; it is the family's position. UML itself does not define the syntax of guard or action expressions at all, so practitioners write them in C, C++, or Java, and the notation "depends heavily on the specific programming language". A formalism that leaves its own guard language undefined cannot be the basis of a closed, checkable guard vocabulary — which is the one thing ratmac's `RB106`/`RB107` findings depend on.

**And there is a worse problem, which arrives before any guard is written.** Both UML statecharts and SCXML posit an **unbounded event queue** in their execution semantics. Brand and Zafiropulo proved in 1983 that communicating finite-state machines with unbounded channels are Turing-powerful. So the undecidability is in the *semantics of the formalism itself*, not in some optional feature an author might avoid: a statechart with a completely empty guard language, no variables, and no orthogonal regions still has an unbounded queue underneath it. Nor is a lossy queue an escape — for lossy channel systems, recurrent reachability, liveness, boundedness, and every behavioural equivalence remain undecidable, and what decidability survives is not primitive recursive.

This is the decisive fact for ratmac. Adopting statecharts would mean adopting a formalism whose *base semantics* is Turing-complete, and then hoping that the fragment actually used stays analysable. ratmac's current position is the opposite: the format is a finite graph with a closed guard vocabulary, so analysability is a property of the format rather than a property of restraint. The moment an edge carries an expression, `rtm doctor` cannot decide whether an edge is dead, whether a phase is reachable, or whether two edges out of a phase are mutually exclusive. Reachability becomes reachability-modulo-a-program, which is undecidable in general. Every finding in the `RB2xx` family that is an error today would become a warning saying "may be unreachable".

### What survives

| Feature | Survives ratmac's offline analysis? |
| :--- | :--- |
| Depth / superstates | **Yes.** Flattens mechanically; every current check applies to the flattened form. |
| Orthogonal regions, sealed | **Mostly.** Configuration well-formedness is a linear check; a sealed region is analysable alone. |
| Inter-level transitions | **No.** Destroys compositional analysis; a region can no longer be checked in isolation. |
| History states | **No, for ratmac.** Adds unconstrained side state and buys nothing in a forward-only build loop. |
| Guarded transitions with expressions | **No.** Deletes P-3 and turns every reachability error into a maybe. |

**Verdict on statecharts.** Steal depth as sugar if edge duplication becomes painful — it is free. Steal the *idea* of an orthogonal region only in its sealed, block-structured form, which is what the Petri-net section reaches by a better road. Do not adopt statecharts as the formalism: their semantic variance is a direct attack on "authored from the written schema", and SCXML's conditional transitions are a direct attack on P-3.

---

## 2. The actor model and supervision trees

This is the closest match to what the owner asked for in so many words: *each subtask has its own abstraction*. It deserves the most honest treatment in this document, because the intuition is right and the formalism is still the wrong thing to adopt.

Carl Hewitt's actor model gives each actor three things: private state no other actor can touch, a mailbox that queues incoming messages, and the ability to create more actors. Erlang and its Open Telecom Platform (OTP) libraries turn this into an engineering discipline. A **supervisor** is a process whose only job is "starting, stopping, and monitoring its child processes", and whose purpose "is to keep its child processes alive by restarting them when necessary". Four restart strategies form the whole policy vocabulary:

| Strategy | On a child's death |
| :--- | :--- |
| `one_for_one` | Only that child restarts |
| `one_for_all` | All children stop, then all restart |
| `rest_for_one` | That child and every child started after it stop, then restart |
| `simple_one_for_one` | A degenerate case for many dynamic instances of the *same* child |

Failure that repeats is bounded rather than looped: a supervisor declares an `intensity` and a `period`, and when restarts exceed that rate "the supervisor terminates all the child processes and then itself", handing the decision to *its* parent. Children start in declaration order and shut down in reverse. Akka and its successor Pekko carry the same structure into the Java Virtual Machine with typed behaviors and supervision wrappers.

Two details are worth carrying into any design that borrows this. First, restart budgets **multiply down the tree** — "the total number of restarts will be the product of the intensity values of all the supervisors above the failing child process", so ten at the top over ten below is a hundred, which is a trap worth knowing before copying the mechanism. Second, `simple_one_for_one` is the exact shape a ratmac fan-out needs: many dynamically added instances of one declared child. The formalism has already isolated the case.

### What it buys

**Failure isolation that is structural, not disciplinary.** A crashed child cannot corrupt its parent's state, because there is no shared state to corrupt. In ratmac's terms: a subagent that wedges its ticket cannot leave the parent's position wrong, because the parent's position was never writable from the child. That is a stronger version of the current invariant, which today is a rule the engine keeps rather than a rule it enforces — `src/state.rs` funnels every internal write through one crate-private path, and nothing stops a person or agent editing `.arca/state.toml` by hand.

**A named owner for lifecycle.** The supervision relation answers questions ratmac has no answer for today: who decides a stuck branch is dead, who restarts it, how many restarts before the failure escalates, and what happens to siblings when one dies. The four restart strategies are a small, well-tested vocabulary for exactly the policy the parent of a fan-out needs.

**A precedent for "own its own state file".** The actor discipline says private state plus explicit messages. Applied to ratmac, it says: one state file per branch, each with exactly one writer. That *strengthens* the one-writer invariant rather than weakening it — the invariant generalises from "one writer for the state file" to "one writer for every state file", and each branch's writer is its own engine instance.

### What it costs

**There is no position.** The state of an actor system is the product of every actor's private state *and* the contents of every mailbox. Mailboxes are unbounded queues. So the state space is infinite even with a fixed, finite set of actors — and the actor set is not fixed, because actors create actors at runtime. "Where are we" has no bounded answer. P-1 is not degraded here; it is deleted.

**Static analysis is essentially absent, and this is verifiable rather than merely asserted.** There is no offline check on an OTP supervision tree comparable to `rtm doctor`, and three separate observations converge on that.

*The tree is a runtime value.* Child specifications are returned by the `init/1` callback when the supervisor process starts, "rather than being fixed at compile time". Children may also be added later with `start_child`. So the shape of a supervision tree is computed by a program, not declared in a document a linter can read.

*The standard tool does not attempt it.* Dialyzer, Erlang's static analyser, works from success typings and detects "definite type errors, code that is unreachable because of programming errors, and unnecessary tests", ensuring "sound warnings without false positives". Concurrency and message protocols appear nowhere in that description. Its design point is the opposite of a linter's: it reports only what it can *prove* wrong, and is therefore silent on everything it cannot — the complement of WhatsApp's eqWAlizer, whose authors draw the line explicitly ("eqWAlizer is a type checker and Dialyzer is a static analysis tool"), flagging whatever it cannot prove *correct*.

*The literature is close to empty.* A targeted search of the computer-science bibliography across supervisor, supervision-tree, `gen_server`, and OTP-behaviour-semantics queries surfaced **no dedicated paper on statically verifying OTP supervision trees**, and none formalising `gen_server`/`supervisor` semantics. The nearest work addresses Erlang fault tolerance broadly (Nyström, 2009) or Core Erlang semantics, which only reached the concurrent fragment in 2024. This should be read as "nothing surfaced under the queries tried", not as proven absence — but for a thirty-year-old industrial discipline, an empty shelf is itself the finding.

The tools that do model-check actor programs (the Soter line for Erlang-style actors; Rebeca and its Afra toolset) buy decidability by *abstracting* the mailbox — counter abstractions that forget message order, or bounded exploration — which lands the analysis back in Petri-net territory with Petri-net complexity. Even the classical boundary is sharper than usually quoted: for communicating finite-state machines, Brand and Zafiropulo showed that with a *single* message type between two machines, boundedness, deadlock-freedom, and absence of unspecified receptions are decidable, and at two or more message types they become undecidable. One extra message type is the entire distance between analysable and not.

Verifying communication properties statically therefore requires importing a discipline from *outside* the actor model, which is what behavioural and session types do (section 6). The state of that art is three months old: Fowler and Hu's Maty (OOPSLA, April 2026) extends a statically session-typed actor language with Erlang-style supervision and cascading failure while preserving its metatheory. Against an otherwise empty field, that is the demonstration that supervision *can* be statically verified — and note what it costs: a typed language designed for the purpose, not an annotation added to an existing one.

**The philosophy is the opposite of ratmac's.** "Let it crash" is a *recovery* answer to unreliability: do not try to prove the process correct, make failure cheap and restart from a known-good state. ratmac's thesis is a *verification* answer: trust comes from deterministic guards over artifacts, and a failing guard refuses and leaves state untouched. Both are respectable. They are not the same bet, and importing OTP's structure imports its bet.

**Mailboxes are queues, and ratmac's non-goals forbid a queue.** `.arca/steering.md` says "Not a CI system, task queue, or scheduler-as-a-service." A mailbox is a queue with delivery semantics, ordering guarantees, and backpressure. Adopting the actor model wholesale means building the thing the project has twice declared it will not build.

**Fan-in is not included.** This is the practical gap people miss. Supervision is about *lifecycle*, not *rendezvous*. OTP hands you a tree that starts, restarts, and stops children; it hands you nothing that says "proceed only when all N children have succeeded". You write that yourself: gather N replies, count them, decide what the missing one means, handle the timeout. The barrier — precisely the part ratmac would need to *gate* on — is the part the actor model leaves as an exercise.

### What survives

The supervision *relation* survives and is worth taking: a parent owns its children's lifecycle, failure escalates upward, and restart policy is declared rather than improvised. That relation is a finite, statically declarable annotation on a fan-out phase, and a linter can check it. Everything else — mailboxes, dynamic spawn, message-passing topology — is unanalysable offline and violates stated non-goals.

**Verdict on actors.** Right intuition, wrong formalism to adopt. Take the supervision relation. Leave the mailbox. Branches in ratmac should communicate only through artifacts on disk that guards already read — which is not a message-passing system at all, and that is exactly why it stays checkable.

---

## 3. Pushdown automata, recursive state machines, and hierarchical task networks

### The call/return question

The moment a phase can *call* a sub-machine and *return* to where it was, position stops being a control location and becomes a control location plus a **stack**. This is a pushdown system, and it is worth being precise about what that does and does not cost, because the received wisdom overstates it in one direction and understates it in another.

**What it costs P-1.** "Where are we" is now a sequence, not a name. `rtm status` prints a call stack. Comparing two positions means comparing stacks. The set of reachable positions is infinite if recursion is unbounded, so a position cannot be checked against an enumerated list — only against a grammar.

**What it does *not* cost — and this surprises people.** Reachability does *not* become undecidable. Reachability and model checking for pushdown systems are decidable, and the classical saturation algorithms (the Bouajjani–Esparza–Maler line, implemented in tools such as Moped and generalised in weighted pushdown systems) compute the set of reachable configurations as a finite automaton, in time polynomial in the size of the pushdown system. The recursive state machine formulation (Alur, Benedikt, Etessami, Godefroid, Reps and Yannakakis) gives the same result in the language a state-machine person actually thinks in: a set of component machines, each with entry and exit nodes, where a box in one machine stands for an invocation of another. Reachability there is likewise polynomial in the machine's size.

So the honest statement is: **recursion costs the shape of position, not the decidability of analysis.** That is a much smaller loss than "we can no longer lint it". What it costs *ratmac specifically* is that `rtm doctor` would stop being a graph walk and start being a model checker — a saturation fixpoint over configurations. That is a real jump in implementation weight for a project whose entire engine is under six thousand lines.

**Visibility is the cheap version.** Alur and Madhusudan's visibly pushdown automata — equivalently, nested word automata — partition the alphabet into **call**, **return**, and **internal** symbols, so the stack's behaviour is dictated entirely by which kind of symbol is being read. The machine pushes only on calls, pops only on returns, and "cannot push to and pop from the stack with the same input symbol". Because the two machines in a product construction then have stack actions "synchronized along the input symbols read", the class recovers what deterministic context-free languages lack: visibly pushdown languages are closed under union, intersection, and complement, forming a Boolean algebra, and inclusion and universality are decidable.

The lesson transfers directly: **if a runbook's call and return are explicit, statically paired syntax — a bracket — you keep almost everything.** If they are computed at runtime, you do not. This is the same lesson the Petri-net section reaches from the other side, and it is the single most reusable idea in this survey.

Two honest qualifications. Decidable is not cheap: inclusion for nondeterministic visibly pushdown automata is EXPTIME-complete, and determinisation can take *s* states to 2^(s²). And the class sits properly between the regular and the deterministic context-free languages — so this is a real step up from where ratmac is, not a free one. Visibility rescues *analysability*; it does not restore *linearity*. That is the argument for going one step further and bounding the nesting.

**Single entry is where it becomes genuinely cheap — even with unbounded recursion.** This is the strongest result in the section and it inverts the usual intuition. Alur, Benedikt, Etessami, Godefroid, Reps and Yannakakis establish the correspondence directly: every pushdown system is bisimilar to a recursive state machine and vice versa, and **every context-free system is bisimilar to a single-exit recursive state machine** and vice versa. Multiple-exit machines are strictly more expressive — there are multi-exit machines whose unfolding is bisimilar to no single-exit machine's. The complexity then splits by interface shape:

| Recursive hierarchical machine | Reachability | Cycle detection | Linear-time logic | Branching-time logic |
| :--- | :--- | :--- | :--- | :--- |
| Single-exit (any entries) | Linear | Linear | Linear | Linear |
| **Single-entry** / multi-exit | **Linear** | **Linear** | **Linear** | EXPTIME |
| Multi-entry / multi-exit | Cubic | Cubic | Cubic | EXPTIME |

**Linear, with unbounded recursion, provided one end of the interface is narrow.** The parameter is explicit in the theorem: reachability and cycle detection run in `O(nθ²)` time and `O(nθ)` space, where **`θ` is the maximum over components of the *smaller* of the entry count and the exit count**. So a component with many entries is still cheap if it has few exits, and vice versa. Single entry means θ = 1, which collapses the bound to `O(n)`; a wide interface at both ends lets θ grow with the machine, which is where "cubic" comes from.

That is a direct instruction for the runbook format. A sealed sub-machine with exactly one entry phase — which is what `RB202`/`RB203` already enforce per machine — sits in the linear row. **The cost of recursion was never recursion; it was a wide interface.**

**Bounded nesting is still required, and now for a sharper reason.** Everything above is about *sequential* hierarchy. Ramalingam proved that context-sensitive, synchronisation-sensitive analysis is undecidable — concurrency plus recursion is undecidable "even for the simplest analysis problems". Since the recommendation has concurrency, unbounded recursion is not merely expensive but off the cliff. Declaring nesting statically so the doctor computes a finite maximum depth is what keeps the combination decidable. Position becomes a bounded tree instead of an unbounded stack, and every check stays a finite graph walk.

### Hierarchical task networks

Hierarchical task network planning decomposes a compound task into subtasks by applying *methods*, recursively, until only primitive tasks remain — SHOP2 being the best-known implementation. It is the AI-planning cousin of the same idea.

The complexity results are grim and famous, and worth stating exactly, because the usually-quoted version is garbled. From Erol, Hendler and Nau:

| Restriction | Propositional | With variables |
| :--- | :--- | :--- |
| None, partially ordered | **Undecidable** | **Undecidable** |
| None, totally ordered | in EXPTIME; PSPACE-hard | in double-EXPTIME; EXPSPACE-hard |
| Regular (at most one compound task, last) | PSPACE-complete | EXPSPACE-complete |
| No compound tasks, totally ordered | Polynomial | NP-complete |

Note what is *not* true: the frequently-cited "propositional hierarchical task network planning is EXPSPACE-complete" is wrong — unrestricted propositional planning with partial order is **undecidable**, and EXPSPACE-completeness belongs to the *regular, with-variables* row. The totally-ordered case was only closed later, by Alford, Bercher and Aha, as **EXPTIME-complete** propositionally and double-EXPTIME-complete with variables.

**And here is where this section's two halves meet.** Erol, Hendler and Nau's undecidability proof is a reduction from *emptiness of the intersection of two context-free grammars*, and they say the correspondence outright: "just as restricting context-free grammars to be right linear produces regular sets, restricting HTN methods to be regular produces STRIPS-style planning." Höller and colleagues later sharpened it to an equality — **the languages of totally-ordered hierarchical task networks are exactly the context-free languages** — while partially-ordered networks escape context-freeness strictly, because interleaving is not a stack discipline.

Put that beside the recursive-state-machine result above and the picture closes: **totally-ordered decomposition is a context-free derivation, which is exactly the single-exit regime where reachability and linear-time checking are linear.** Interleaved decomposition leaves that regime, and no visibility trick recovers it, because there is no stack shape left to make visible. Ordering is the property that buys analysability — the same lesson as sealing, in a third vocabulary.

But those numbers are a warning, not a bill ratmac would pay, and it is important to say why: **HTN is a search formalism.** It answers "find *a* decomposition that achieves the goal". ratmac never searches. A runbook is a fixed plan, authored by a human, reviewed before it becomes the project's machine. The expressive power HTN buys — the planner choosing between methods — is power ratmac's Ideal shape explicitly refuses, since routing must be deterministic and a phase has exactly one automatic destination.

What is worth stealing from HTN is one vocabulary item: the **method**, a named, reusable decomposition of a compound task into an ordered set of subtasks. That is the right way to think about "one sub-machine definition, instantiated per ticket". Take the noun. Leave the planner.

### What survives

| Feature | Survives? |
| :--- | :--- |
| Unbounded recursion, wide interface | **Analysis yes, cheaply no.** Cubic at best; `rtm doctor` becomes a model checker. |
| Unbounded recursion, **single entry** | **Yes, linearly.** Reachability, cycle detection, and linear-time properties all stay linear. |
| Visible, syntactically paired call/return | **Yes, but not cheaply.** Boolean-algebra closure and decidable inclusion are recovered; inclusion is EXPTIME-complete. |
| Recursion **combined with concurrency** | **No — undecidable** (Ramalingam), even for the simplest analyses. This is why depth must be bounded. |
| Statically bounded nesting depth | **Yes, fully.** Configuration space stays finite; every current check still applies. |
| HTN method selection / search | **Not applicable.** ratmac does not plan; adopting it would import undecidability for a capability the project refuses. |

---

## 4. Petri nets and workflow nets

This is the formalism that actually contains a fan-in, and it deserves the most careful reading, because it also contains the worst complexity results in this document — and the escape from them.

### Why this is exactly the right generalisation

A Petri net is a bipartite directed graph of **places** and **transitions**, with **tokens** distributed over places. The distribution is called a **marking**, and the marking is the state. A transition may fire when every input place holds enough tokens; firing atomically removes tokens from the inputs and deposits them in the outputs.

The connection to ratmac is exact and worth stating precisely. A Petri net in which **every transition has exactly one input place and exactly one output place, and every marking holds exactly one token**, is called a *state machine net*. It expresses conflict but not concurrency. **That is ratmac today.** Not "like" ratmac — it is the formal definition of the machine ratmac already runs.

So the move the owner is asking for has an exact name: **allow more than one token.** Everything else in this section is the consequence of that one change.

### What it buys: a barrier that is an arc, not a protocol

Fan-out is a transition with one input place and N output places. Fan-in is a transition with N input places and one output place. The join transition **cannot fire** until every one of its input places holds a token. That is worth dwelling on, because it is the property no other formalism in this survey provides as cleanly:

- In SCXML, the join is a convention over `done.state` events.
- In the actor model, the join is code you write — count replies, handle the missing one.
- In durable execution, the join is `await` over a set of promises inside a program.
- In a Petri net, **the join is an arc.** There is nothing to implement and nothing to get wrong, and a static checker sees it as structure rather than having to reason about a program.

For a project whose thesis is "process-as-data beats process-as-prompt", that difference is not aesthetic. A barrier expressed as data is lintable. A barrier expressed as code or convention is not.

### What it costs

**Position becomes a vector.** A marking is a multiset over places. `rtm status` prints a vector of counts rather than a name. Checking a marking is *well-formed* stays cheap — every named place exists, every count is a non-negative integer, linear in the marking's size. Checking a marking is **reachable** is a different matter entirely.

**The general complexity results are catastrophic.** Reachability in Petri nets is decidable (Mayr, 1981), was long known to be EXPSPACE-hard (Lipton), was shown non-elementary in 2018, and was finally settled as **Ackermann-complete** in 2021, proved independently by Leroux and by Czerwiński with Orlikowski. Ackermann-complete means not primitive recursive. For an offline linter that must always terminate with a verdict, that is not a slow algorithm; it is a different category of thing.

**Soundness is the property that actually matters, and it is not free either.** Van der Aalst's **workflow net** is a Petri net with a single source place, a single sink place, and every node on a path from source to sink. Its **soundness** property is the workflow analogue of "the doctor is clean", and it is three conditions: from the initial marking the net can always still reach the terminal marking (option to complete), when the sink is marked nothing is left behind (proper completion), and no transition is dead (every transition can fire in some reachable state). The classical reduction short-circuits the net — an extra transition from sink back to source — and turns soundness into **liveness plus boundedness** of the result.

The complexity here is worth stating carefully, because the number usually quoted is folklore. The often-repeated "soundness is EXPSPACE-hard" is not a theorem anyone proved, and its usual attribution is to a paper that actually established PSPACE-completeness for 1-safe nets. The real result is recent: Blondin, Mazowiecki and Offtermatt (Logic in Computer Science, 2022) settled it as **EXPSPACE-complete** for classical and structural soundness, and **PSPACE-complete** for generalised soundness. So the honest summary is that soundness is decidable and genuinely intractable — but by a proved bound rather than a repeated rumour, and a full exponential better behaved than reachability.

### The escape: block structure buys soundness by construction

This is the finding the recommendation rests on.

You do not have to decide soundness by searching a state space if you can **build only nets that are sound**. The relevant structural notion is **well-handledness**: a net is well-handled when no two fully distinct elementary paths run between a place and a transition — any two such paths share a node. Informally, every split is matched by a join of the same kind, nested like brackets, with no path escaping from inside one branch to somewhere outside it.

The results that matter:

- An **acyclic well-handled workflow net is generally sound** (Ping and colleagues, 2004) — sound for every initial token count, not just one.
- **Well-handled nets with Regular Iteration** are closed under substituting a transition by another such net. Because they are generally sound, **composing only well-structured blocks yields soundness by construction.**

Read that as an engineering statement: if the only ways to combine pieces are *sequence*, *choice*, and *sealed parallel block*, and each piece is itself built that way, then the whole is sound and you have proved it **by parsing**. No search, no fixpoint, no state-space exploration. `rtm doctor` stays a linear walk over a declaration.

The decisive point is that this is not merely a folk heuristic. Van der Aalst's own paper carries the structural notions — well-handled, well-structured, S-coverable — and **two polynomial-time corollaries**: soundness is decidable in polynomial time both for free-choice workflow nets and for well-structured ones. That is the gap the recommendation lives in. General soundness is EXPSPACE-complete; restricted to the structural class, the *same property* drops to polynomial. Restriction is not a consolation prize here — it is a complexity-class jump bought with a syntax rule.

Free-choice nets — where conflict and concurrency never occur at the same place — are the other classical well-behaved subclass with the same polynomial result. But block structure is the better restriction for this purpose, because it can be enforced *syntactically*: a syntactic rule is something an agent authoring a runbook can obey, a diagnostic code can name, and a reviewer can see. "Free-choice" is a property you check; "brackets match" is a property you cannot violate without the parser saying so.

### Why workflow engines chose Petri nets, and what they learned

Van der Aalst's workflow patterns catalogue is the empirical case: a plain sequential state machine cannot express parallel split with synchronisation, multiple instances of an activity, discriminators, or cancellation regions. YAWL — Yet Another Workflow Language — was built directly on Petri-net foundations *and then extended past them*, precisely because plain nets could not handle multiple instances, cancellation, and the general OR-join cleanly. That is an honest warning: Petri nets are the right base and they are not sufficient for every pattern people actually want.

The Business Process Model and Notation carries token semantics inherited from this lineage, and its most famous unresolved wart is the **OR-join**: "proceed when the branches that were going to arrive have arrived" requires knowing whether more tokens *could still* arrive, which is a non-local property of the whole net. Every attempt to give it a local semantics has been unsatisfying.

The lesson for ratmac is direct and cheap to apply: **support the AND-join only.** "Every branch must arrive" is local, structural, and decidable. "Some branches must arrive" is the thing that broke BPMN. A runbook should be unable to express it.

Coloured Petri nets attach typed data to tokens and are what practitioners actually model with (CPN Tools). Colouring buys expressiveness and immediately costs the analysis: with an unbounded colour domain the net is Turing-powerful, and analysis proceeds by unfolding to an ordinary net, which is only possible when the colour sets are finite and small. For ratmac, the equivalent temptation is letting a token carry data that a guard then branches on — which is P-3 deleted by another route.

### What survives

| Feature | Survives ratmac's offline analysis? |
| :--- | :--- |
| Multiple tokens (real concurrency) | **Yes**, at the cost of position being a vector rather than a field. |
| Structural AND-join | **Yes.** The barrier is an arc; a linter reads it directly. |
| General net soundness checking | **No.** Reachability Ackermann-complete; soundness EXPSPACE-complete. |
| Free-choice or well-structured soundness | **Yes.** Polynomial time, by van der Aalst's own corollaries. |
| Block-structured / well-handled composition | **Yes, completely.** Soundness by construction — proved by parsing. |
| OR-join | **No.** Non-local semantics; the known unsolved case. Forbid it. |
| Coloured tokens with data-dependent firing | **No.** Turing-powerful in general; deletes P-3. |

---

## 5. Durable execution

Temporal, Restate, DBOS, and Azure Durable Functions solve a problem adjacent to ratmac's and solve it well. Understanding *how* they get isolation plus composition is instructive, and understanding what they pay for it is decisive.

### How they get it

**Temporal** runs workflow code as ordinary program code and makes it durable by **deterministic replay**. Every step's result is appended to an **event history** — "a complete, ordered log of everything that has already happened in a Workflow", and "the source of truth for everything that happens in the Workflow". Recovery does not restore a memory snapshot: the code "restarts from the top", the history is walked forward, and recorded results are handed back instead of the work being redone, so execution fast-forwards to exactly where it stopped.

The price is that the workflow function must be deterministic — it "has to make the same decisions when given the same history", and "shouldn't depend on any values *not* recorded in the history which would be different between runs". The documentation names the hazards directly: a call to `Date.now()` "could return a different value on replay", "a random number could change", an unwrapped network request "could return something new". Everything touching the outside world must move into an activity, and time and randomness must come from workflow-provided APIs that record what they returned. Changing the code of a running workflow breaks replay against old histories, which is why the versioning and patching APIs exist at all.

Note what kind of guarantee that is. Determinism here is a **property of a program that the runtime polices at replay time**, not a property of a document that a checker decides beforehand.

Composition comes from **child workflows**: a workflow starts another workflow that has its own identity, its own event history, its own retry policy, and its own timeouts, with a declared parent-close policy. Fan-out and fan-in is starting N children or activities and awaiting all of them.

**Restate** takes a different route to the same place: a durable log underneath, with virtual objects providing single-writer, key-addressed concurrency, plus durable promises and journalled side effects. The single-writer-per-key idea is genuinely close to ratmac's one-writer invariant, and closer than Temporal's model.

**Azure Durable Functions** names the same pattern explicitly in its documentation as "fan-out/fan-in", with the same orchestrator-determinism rules.

### What it costs

**There is no graph to lint.** This is the whole point and the whole problem. The process definition is a Turing-complete function. You cannot ask whether a phase is unreachable, because there are no phases. You cannot ask whether an edge is dead, or whether two outgoing routes are mutually exclusive, or whether the machine has one entry point. Every `RB2xx` finding in ratmac's diagnostic table is *inexpressible* in this model, not merely harder.

Determinism is enforced by three runtime mechanisms, none of them offline: a sandbox that stubs out nondeterministic APIs, a replay check that raises a nondeterminism error when the new execution diverges from the recorded history, and the versioning APIs that exist because the first two are brittle under code change.

Static analysers do exist, and their *stated limits* make the point better than any outside commentary could:

- **Temporal's `workflowcheck` (Go)** finds workflow functions by their `workflow.Context` first parameter and searches transitively for known-nondeterministic calls — `time.Now`, `math/rand`, channel operations, `range` over a map. Its own README says, in bold, that it "will not catch all cases of non-determinism such as **global var mutation**", and positions itself as "just a helper" with developers expected to "still review workflow code themselves". Global variable mutation is excluded *deliberately*, because it "cannot be reliably distinguished from deterministic use in common cases" — the distinction is semantic, not syntactic.
- **Temporal's Java analyser** scans bytecode and is labelled "beta quality". Its authors' rejected-alternatives list is itself evidence: Checkstyle, ErrorProne and PMD were "not built for transitive bytecode checking", and **CodeQL was "Too slow"**. They wrote a bespoke scanner instead. Static types also defeat detection — iterating a `TreeMap` is flagged because `entrySet`'s declared type is `Set`.
- **TypeScript and Python have no static analyser at all**, only sandboxes. Temporal's own Python sandbox documentation concedes it "is not completely isolated, and some libraries can internally mutate state, which can result in breaking determinism", and that using it "doesn't completely negate the risk".
- **Restate has no static analyser in any language.** Enforcement is entirely the `RT0016` journal-mismatch error at replay.
- **Azure Durable Functions ships real Roslyn analysers** — the closest thing in this space to `rtm doctor` — and the decisive detail is severity: every orchestrator-determinism rule is a **Warning or Info**, while only *binding* misuse is an Error. **A determinism violation does not fail a default build.** Cross-assembly resolution is unsupported, so a nondeterministic call reachable through a referenced library is invisible; and no analyser exists for JavaScript, Python, PowerShell, or Java — the very languages carrying the sharpest constraints.

Each of these is a lint for *one class of bug*. None is a process-graph checker, and none can be, because there is no process graph. The whole class of **ordering** nondeterminism — you inserted an activity call in the middle of the sequence — is out of static reach entirely; every vendor's answer to it is a runtime conditional (`GetVersion`, `patched`, `DBOS.patch()`, version branching), never an analysis.

What actually enforces determinism is worth naming plainly, because it is the opposite of offline checking. The recommended procedure is: download the event histories of a representative sample of recent workflows, replay them against the new code, and fail continuous integration on any replay error. That is **regression testing against production traces**. It can only find defects your sampled histories happen to exercise.

**It requires a server.** Temporal needs a cluster; Restate needs a runtime service. Both are network services with persistent storage. ratmac is deterministic and offline: no network, no installs, no hidden global state. This is not a preference to be traded off — it is an invariant in `.arca/steering.md`.

**It is a scheduler.** Task queues, workers, timers, retry policies, backpressure. That is the thing ratmac's non-goals name in the first line.

### The two ideas worth stealing

**The journal.** Durable execution's real insight is that **position is derived from an append-only record of completed steps, not stored as a mutable field**. Position is a fold over history. ratmac already has the two halves of this: append-only history in `.arca/log.md`, and structured per-check receipts under `.arca/evidence/<ticket-id>/`. If a branch's position is derivable from its own receipts, then a branch is resumable and auditable without any scheduler — and, importantly, a branch's position becomes *evidence* rather than *narration*, which is precisely the project's thesis applied one level down. Note that `.arca/index.md` already states the same principle for the repository as a whole: "Where are we? Derived from the tree, never declared."

**Child identity.** A child has its own history, its own identity, and its own lifecycle. That is the per-subtask abstraction the owner wants, and it is a naming decision, not a formalism.

### The declarative counter-example: Amazon States Language

The right thing to look at is not Temporal but **Amazon States Language**, the declarative JavaScript Object Notation state machine behind AWS Step Functions. It has exactly the two constructs this survey converges on, and both are statically checkable:

**`Parallel`** declares an array of `Branches`, each a self-contained state machine with its own `StartAt` and `States`. Step Functions runs the branches concurrently and "wait[s] until all branches terminate (reach a terminal state) before processing the `Parallel` state's `Next` field" — a structural AND-join. And the sealing rule is stated in the documentation in exactly the terms this document has been reaching for:

> "Each branch must be self-contained. A state in one branch of a `Parallel` state must not have a `Next` field that targets a field outside of that branch, nor can any other state outside the branch transition into that branch."

That single sentence is the well-handledness condition, written as a format rule an author can obey and a validator can check. From the outside, a `Parallel` state is one state with one entry and one exit. The specification adds a data-scoping rule in the same spirit: "a state in one branch MUST NOT reference any variable assigned in another branch." Control and data are sealed together.

**And the static checking is real, not hypothetical.** Amazon ships a `ValidateStateMachineDefinition` interface that "validates the syntax of a state machine definition… **without creating a state machine resource**", returns coded diagnostics with document locations and an overall pass or fail, and is explicitly recommended for "a Git **pre-commit hook**" — with Amazon's own word for the warning-severity mode being "**static analysis**". That is `rtm doctor`'s shape, from a cloud vendor, over a declarative document. The `statelint` validator covers "most of the grammatical constraints" independently, and the editor integration lists among the defects it identifies "**Non terminal state**" and "**Nonexistent states that are pointed to**" — dangling transitions and missing endings, caught offline, which is precisely ratmac's `RB202`–`RB205` family.

One honest boundary: the hosted validation interface documents only syntax checking plus a pass or fail verdict, and warns callers not to depend on the wording or ordering of diagnostics. The reachability-style checks above are documented for the editor tooling, not for that interface. The two should not be conflated — but between them, the declarative model demonstrably supports the offline analysis the imperative model cannot.

**`Map`** goes further and is the closer match to ratmac's need: one `ItemProcessor` sub-machine, applied once per item in a dataset. The **iteration count is data-dependent** — it comes from the input, not from the declaration — while **the shape is static**, because there is exactly one sub-machine definition in the document however many items arrive. In Distributed mode each iteration runs as a child execution with "its own, separate execution history from that of the parent workflow".

Dynamic width, static shape, own history per instance, structural join, sealed branches. That is the entire ask, expressed in a declarative document that a validator can check offline.

The reason it works reduces to one sentence, and it is the thesis of this whole survey: **there is no user code in the graph, so there is nothing at the orchestration layer that could be nondeterministic. Determinism is a property of the language rather than a contract on the author.** That is the exact inverse of Temporal, Restate, DBOS, and Durable Functions, and it is the property ratmac already has and should not trade away.

### What survives

| Feature | Survives? |
| :--- | :--- |
| Deterministic replay from a journal | **Yes as a principle** — position derived from evidence, not stored. |
| Child workflow identity and isolation | **Yes.** A naming and file-layout decision. |
| Process definition as imperative code | **No.** Deletes the graph, and with it every current doctor finding. |
| Server-backed durability, task queues, workers | **No.** Violates offline-and-deterministic and the not-a-scheduler non-goal. |
| Declarative `Parallel` / `Map` with sealed branches | **Yes.** This is the model to copy. |

---

## 6. Process calculi — for the vocabulary

Taken only as a source of precise words for composition, as instructed. Four items earn their place.

**Parallel composition and synchronisation (Hoare's Communicating Sequential Processes, Milner's Calculus of Communicating Systems).** The vocabulary item is that `P || Q` is itself a process — composition is *closed*, so a composite can be used anywhere a primitive can. That is the property that makes a fan-out phase "one phase from the outside", and it is worth naming because it is what keeps the runbook graph flat while the work underneath is parallel.

**Refinement.** CSP's checker FDR decides whether `Spec ⊑ Impl` — whether an implementation only does things the specification allows — over three models: traces, stable failures, and failures-divergences. The transferable idea is that **a sub-machine has an interface, and the parent needs only the interface.** Check each machine against its own interface once; do not check the product. This is what makes offline analysis *compositional*, and therefore what keeps it cheap as the number of branches grows. Without it, analysis cost multiplies with branch count; with it, it adds.

The tool's own guidance is the empirical confirmation. FDR handles state spaces in the billions, and the way it does so is by state-space compression applied **to components, never to the whole system** — with hiding pushed "as far down the process tree as possible", because compression "will have little affect on a process that contains no hiding". The practitioners' rule is: seal a part, hide its internals, compress it, then compose. That is the same instruction the recommendation gives, arrived at from the tooling side.

**Mobility (the pi-calculus).** Channel names can be passed as messages — with scope extrusion letting a bound name escape its original scope — so the communication topology changes at runtime. This is exactly "spawn a subtask and hand it a handle", and it is exactly what makes static topology impossible. Name-passing plus replication is what makes the calculus a universal model of computation.

The recovery is instructive and lands in the same place as everything else here: decidability comes back only by **bounding the dynamism** — the finite-control fragment, where the number of parallel components is bounded by a constant, or the depth-bounded fragment. And there is a sting worth knowing: *recognising* the depth-bounded fragment is itself undecidable. A restriction is only useful if membership in it is cheap to check, which is the argument for a syntactic rule over a semantic property, one more time.

The value of naming all this is to be able to refuse it deliberately: ratmac should have no mechanism by which one branch learns of another at runtime. Branches meet only at the join.

**Session types, and multiparty session types (Honda, Yoshida and Carbone).** The most useful import in this section. A **global type** describes the whole protocol as "a global scenario"; it is **projected** onto each participant to give a local endpoint type; and because the global type is "a shared contract among participants", type checking is done efficiently by projection onto each peer rather than over the product of all of them.

The guarantees are three, and it is worth using the authors' own words rather than the ones usually attributed to them: **communication safety, progress, and session fidelity**. "Deadlock freedom" is the common paraphrase and is narrower than it sounds — progress subsumes deadlock-freedom for a *single* session, while guarantees across dynamically interleaved sessions took a separate later result.

The structure of the argument is the same one the block-structured workflow-net result gives, arrived at from the typing side: **write the global shape down first, derive the parts from it, and whole-system properties follow from local checks.**

Translated into ratmac's terms: the fan-out declaration in the runbook is the global type; each branch machine is a projection; the doctor checks projections locally and gets the whole-system property for free.

The line of work has already been pushed at exactly the two points that matter here. *Dynamic participant count* — the fan-out width that is not known when the document is written — is handled by parameterised and dynamic multirole session types (Deniélou and Yoshida, Principles of Programming Languages, 2011), which is the theoretical warrant for "static shape, data-dependent width". *Failure* — a branch that dies mid-protocol — is handled by Maty (Fowler and Hu, 2026), which is the first work to carry supervision and cascading failure into a session-typed actor language without losing the guarantees. Both of ratmac's hard cases have been solved in theory; neither is solved in a formalism ratmac could adopt without adopting a programming language with it.

**Bracketed parallelism, empirically — and this is the sharpest version of the whole argument.** Netzer and Miller drew the boundary in 1992, five years before the algorithms that exploit it, and they drew it exactly at fork-join:

> "Apparent races can be exhaustively located efficiently only for programs that use synchronization incapable of implementing mutual exclusion (such as **fork/join** or Post/Wait synchronization without Clear operations); detection is **NP-hard** for more powerful types of synchronization (such as semaphores)."

The criterion is precise and is not "structured versus unstructured" in a vague sense: it is *whether the synchronization primitive is strong enough to implement mutual exclusion*. Below that line, exhaustive detection is efficient; above it, NP-hard.

The algorithms then cash the boundary in. Because a fully-strict fork-join computation has **every fork matched by a join**, its computation graph is a **series-parallel parse tree** whose internal nodes are series or parallel nodes and whose leaves are threads. "Are these two threads logically parallel?" is then a tree-ancestry query. Cilk's SP-bags answers it in `O(T₁·α(v,v))` — the serial running time times the inverse Ackermann function over the count of shared memory locations — and SP-order improves the per-operation cost to `O(1)` amortised, so any fork-join program running in `T₁` time serially "can be checked on the fly for determinacy races in `O(T₁)` time".

**That is the same property as visibly pushdown languages, in a different field.** The matched-bracket structure is *visible in the program text* rather than emerging at runtime, and visibility is what makes the analysis cheap. Unstructured synchronization destroys the parse tree and the problem goes NP-hard at precisely the point where the primitive becomes powerful enough to implement mutual exclusion.

Four literatures — well-handled workflow nets, multiparty session types, visibly pushdown languages, and fork-join race detection — independently locate the same line. That convergence is the strongest evidence in this document.

---

## Comparison: what each model costs the three properties

| Model | P-1 Position cheap to verify | P-2 Offline analysis total | P-3 Data, no expression language | Gives a real join? | Per-subtask instance? |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Flat finite state machine (today)** | One field, membership test | Complete, linear | Yes | No | No |
| **Statechart, depth only** | Same after flattening | Complete; PTIME, linear algorithms | Yes | No | No |
| **Statechart, orthogonal + history + conditions** | Configuration plus side table | **No** — EXPSPACE at best; Turing-powerful via the unbounded event queue | **No** — guard expressions | By event convention | `<invoke>`, but dynamic |
| **Actor / supervision tree** | **No position exists** | **Essentially none** | N/A — behaviour is code | **No** — hand-written | Yes, but dynamic |
| **Pushdown / recursive state machine, single entry** | Stack, not field | Linear for reachability, cycles, linear-time properties | Yes | Return is a join of one | Yes, by call |
| **Recursion + concurrency, unbounded** | Stack per branch | **No — undecidable** (Ramalingam) | Yes | Yes | Yes |
| **General Petri / workflow net** | Vector; reachability Ackermann-complete | **No** — search is not primitive recursive | Yes | **Yes, structural** | Yes, per token |
| **Block-structured workflow net** | Bounded tree; membership test | **Complete, by parsing** | Yes | **Yes, structural** | Yes, per branch |
| **Durable execution (Temporal / Restate)** | Journal fold, opaque | **No graph to analyse** | **No** — code | Yes, by `await` | Yes, child workflow |
| **Amazon States Language `Parallel` / `Map`** | Bounded tree | Complete, by validation | Yes | **Yes, structural** | Yes, per branch |

---

## Recommendation

### The shape

Add exactly one phase kind to the runbook. Everything else stays as it is.

```toml
[phases.build]
prompt = "Work every open ticket to green."

  [phases.build.fanout]
  over    = "tickets"        # a closed vocabulary of derivable instance sets
  machine = "ticket-cycle"   # a sub-machine declared in this same file
  join    = "all"            # the only accepted value
  on-branch-failure = "hold" # supervision policy, from a closed set

[machines.ticket-cycle]
# an ordinary phases + transitions block, sealed:
# no edge may leave it and none may enter it except at its entry phase
```

This is a schema extension, and it is not free: `machines` becomes a second permitted top-level key alongside `phases` and `transitions`, and `fanout` becomes a third permitted phase field alongside `prompt` and `guards`. Both are currently `RB103`, unknown key. The strict-parser rule is what makes that a controlled change rather than a silent one — `.arca/runbook-spec.md` grows two rows and a diagnostic code, and every runbook written before the change still parses, because nothing existing is removed or given a new meaning.

Six rules make it work, and each one exists to protect a named property.

**1. The instance set is derived from artifacts, never spawned.** `over = "tickets"` means the branch set is read from `.arca/ticket/*.md` at step time — the same enumeration `src/contract.rs` already performs, with the same dependency ordering and the same acyclicity check it already enforces via `find_cycles`. ratmac creates nothing and schedules nothing; it *observes* how many branches there are and verifies that each reached its terminal phase. Fan-out width is dynamic in value and static in shape, which is exactly the `Map` state's bargain. This is what keeps "not a scheduler" true.

**2. Branches are sealed, with exactly one entry.** No transition may cross a branch boundary in either direction. This is Step Functions' self-contained-branch rule and the well-handledness condition at once, and it is what makes the composite phase a single phase with one entry and one exit when viewed from the runbook graph. A new diagnostic code — call it `RB208`, "a transition crosses a fan-out branch boundary" — enforces it, and it is a purely syntactic check.

Sealing is not stylistic. It is the parameter that every complexity result in this survey turns on: a single-entry component keeps reachability, cycle detection, and linear-time properties **linear** even under recursion, while a wide interface makes them cubic and branching-time properties exponential in the exit count. ratmac's existing `RB202`/`RB203` initial-phase checks already enforce single entry — run per sub-machine, they are the whole requirement.

**3. `join = "all"` is the only join.** The AND-join is local and structural. The OR-join is the non-local semantics that has never been settled cleanly in BPMN. A runbook must be unable to express it.

**4. Composition is block-structured, and nesting depth is bounded by declaration.** Sequence, and sealed parallel block. A fan-out phase may contain a fan-out phase, so nesting depth is whatever the runbook declares — and because it is declared, the doctor computes the maximum depth by walking the file.

This rule is the one that is *not* optional, and the reason is Ramalingam's result: **concurrency combined with recursion is undecidable**, even for the simplest analyses. A fan-out phase that could reach itself through a chain of sub-machine references would put this design on the wrong side of that line. Bounding depth is what keeps the configuration space finite and every check a graph walk. Note also the last row of Alur's cube — hierarchy and concurrency together cost two exponentials rather than one — which is the same warning at a lower severity: these two features do not compose for free, and the whole design is arranged so that they never interact except across a sealed boundary.

**5. One state file per branch, one writer each.** The branch's state lives at its own root — in a worktree, that root is the worktree. The one-writer invariant generalises rather than weakens: every state file has exactly one writer, and a branch's writer is its own engine instance. Nothing writes another's file.

**6. Supervision, not messaging.** The parent declares what happens when a branch fails, drawn from a closed set modelled on the OTP strategies (`hold` the branch and keep the siblings running; `hold-all`; escalate). Branches never message each other. They communicate only through artifacts, which guards already read. No channel means no queue, which means no unbounded state, which is why offline analysis survives at all.

### The state file already has the slot

Position on disk is `RunState` in `src/model.rs`, and `src/state.rs` requires exactly seven fields, refusing any file with an eighth. So making position a tree looks like a format break. It is smaller than that, because one of the seven is already the right shape and is not yet used:

> **`active_refs`** — "the Scheduler-written list of what a Run is currently working on, ticket and requirement ids. In the format and in the fixtures since the start; nothing populates it yet." (`.arca/dict.md`)

A fan-out phase's live branch set **is** the list of what the run is currently working on, as ticket identifiers. The field was specified, reserved, and left empty from the beginning; this design is what fills it. Branch-local positions live in each branch's own state file at its own root, so the parent records *which* branches are open and each branch records *where it is* — and no file gains a field.

That is a genuine reduction in cost, and it is worth being suspicious of how neatly it lands. It is not evidence the design is right; it is evidence the shape was anticipated once already.

### The offline checks this adds, in full

The claim "analysis survives" is only worth something if the new checks can be listed and each one shown to terminate. Here they are. Every one is a walk over the declaration; none needs a reachable-state search.

| New check | Decides | Cost |
| :--- | :--- | :--- |
| Branch sealing | No transition names a phase inside a fan-out sub-machine from outside it, and none inside names a phase outside it | One pass over the transition list |
| Sub-machine well-formedness | Each `[machines.*]` block has exactly one entry phase and at least one terminal phase | The existing `RB202`/`RB203` pass, run per sub-machine |
| Sub-machine reachability | Every phase in a sub-machine is reachable from its entry | The existing `RB204` pass, run per sub-machine |
| Instance-set vocabulary | `over` names a derivable set the engine knows how to enumerate | Membership in a closed list |
| Join vocabulary | `join` is `"all"` | Equality against one literal |
| Supervision vocabulary | `on-branch-failure` is in the closed policy set | Membership in a closed list |
| Nesting depth | The fan-out nesting graph is acyclic, so depth is finite and computable | Cycle detection over sub-machine references — the algorithm `src/contract.rs` already has |
| Orphan sub-machine | Every declared sub-machine is referenced by some fan-out phase | Set difference |

Note the last two: sub-machine references form a directed graph, and a cycle in it is exactly the case that would turn bounded nesting into unbounded recursion — the boundary between this proposal and a pushdown system. Detecting it is the same cycle check the record gate already runs over ticket dependencies. The one structural risk this design carries is caught by code that already exists.

The reason the list is this short is rule 2. Sealing is what lets every other check run **per sub-machine** rather than over the product of all of them — the refinement idea from the process-calculi section, cashed in. Analysis cost adds with branch count instead of multiplying.

### Why this and not the others

**Against statecharts:** their base semantics posits an unbounded event queue, which is Turing-powerful before an author writes anything; their semantic variance defeats "authored from the written schema"; and conditional transitions with an undefined expression language delete P-3. Depth alone is worth having as sugar and is genuinely cheap — Alur's cube puts hierarchy at PTIME with linear algorithms, and linear-time properties no harder than flat. The rest is not worth the specification it would require.

**Against actors:** they give isolation and lifecycle brilliantly and give no join at all, no position, no offline analysis, and a mailbox that is a queue the non-goals forbid. Take the supervision relation, which is a finite declaration a linter can read. Leave everything that runs.

**Against a general stack:** recursion costs the *shape* of position without costing decidability, which is a smaller loss than it appears — but it turns `rtm doctor` from a graph walk into a saturation fixpoint. Declared, bounded nesting buys the same reuse for a linear check.

**Against general Petri nets:** they contain the right primitive and the wrong complexity — reachability Ackermann-complete, soundness EXPSPACE-complete. The block-structured restriction is the whole reason this recommendation is affordable, and the size of that saving is a proved corollary rather than a hope: the same soundness property is polynomial-time decidable on well-structured nets. Restricting the format buys a complexity-class jump, and soundness by construction means the doctor never runs even the polynomial check — it proves the property **by parsing**.

**Against durable execution:** it needs a server and it is a scheduler — two invariants and one non-goal, breached. Its declarative cousin, Amazon States Language, has the same isolation and composition with none of the cost, because the process stayed data.

**For this:** three independent literatures — well-handled workflow nets, multiparty session types, and fully-strict fork-join — converge on the same restriction. Bracket the parallelism and the whole-system property follows from local checks. That is not a coincidence; it is the same theorem in three dialects, and it is the only place in this survey where a real barrier and a total offline analysis coexist.

### What you give up

**"Position is one field" is gone, and it cannot be kept.** A join means several places are occupied simultaneously; that is what a join *is*. Position becomes a bounded tree: the top-level phase name, plus one phase name per open branch keyed by branch identity. Verification stays a membership test with no search — every name must be a declared phase of its own machine, every key must be in the derived instance set — so P-1 degrades from "one field" to "a small record", which is the smallest possible version of this loss. But `rtm status` now prints a report rather than a name, and anything in prose or code that says "the current phase" needs rereading.

**The change reaches most of the engine, though shallowly.** Twelve of seventeen modules in `src/` mention a phase. The concentration is in three — `machine.rs`, `scheduler.rs`, and `graph.rs` hold roughly three-quarters of the references — and the rest pass a phase through without reasoning about it. So the work is a real but bounded change to the parser, the run lifecycle, and the graph type, not a rewrite. `completion.rs`, `goal.rs`, `pin.rs`, and `receipt.rs` — the whole evidence layer — mention phases nowhere at all, which is the load-bearing fact: **evidence is already phase-agnostic**, so nothing about receipts, pinning, goal freeze, or the completion gate has to change to work per branch.

**Two phases can be current at once.** The Phase Prompt render has to answer "whose prompt?" — the answer being that a subagent working a branch gets its own branch phase's prompt, and the parent's prompt covers the fan-out itself. That is a real spec change to R-028's neighbourhood.

**A held branch stalls the join.** If one branch is held, `join = "all"` never completes. This is genuine, and the mitigation is that it is *legible* rather than silent: block structure means the doctor can prove there is no cyclic wait structurally, so the only runtime stall is "branch X is held", which is already a named, reported, human-authorised state. An honest stall is the product, per the feasibility direction note.

**This does not answer the other open question.** `.arca/steering.md` asks "how much routing does a runbook get" — whether edges gain conditions and loops. Fan-out is orthogonal to that, and the two must not be solved together: conditional edges are what delete P-3, and mixing the changes would hide that. Worth noticing, though: a bounded fan-out over a derived instance set **is** an iteration — "for each open ticket" — so it may absorb most of the demand for a loop without introducing a single condition. That is an argument for doing this one first.

**No OR-join, no cancellation region, no multiple-instance-without-synchronisation.** YAWL exists because plain nets could not express those. ratmac should not want them, but this is a real expressiveness ceiling and it should be stated rather than discovered.

### On the recursive-self-improvement layer: build it as a machine, not a feature

The recursive-self-improvement layer — the process definition revised from results, with a human as the value function — needs **no new formalism at all**, and building it into the machine would destroy everything above.

If the runbook is data and the doctor is total, then "revise the process" is already a well-formed, checkable operation: edit a file, lint it, review it. So the improvement loop is an ordinary machine whose output artifact happens to be *the next machine's runbook*:

> run a machine → collect receipts → propose a runbook change → `rtm doctor` the proposal → **human accepts or rejects** → the accepted runbook becomes the next run's machine class.

The human-as-value-function is the accept step, and it is already required: `.arca/runbook-spec.md` states that a runbook is reviewed by a human before it becomes the project's machine class, and the ownership table marks that rule prose-only. This makes the review a phase with a guard instead of a convention.

A machine that rewrites itself *while running* would break three things at once: the goal-freeze invariant would have nothing stable to pin, since a run pins a goal revision by content hash and a self-rewriting run has no fixed identity; determinism would go, because the same inputs would no longer give the same walk; and offline analysis would become meaningless, since the artefact the doctor checked is not the artefact that ran. The generational form — one machine produces the next machine's definition, a human signs it, the doctor checks it before it runs — gets the entire capability and keeps all three.

State this as a design rule and it stops being a temptation: **the machine never writes its own runbook. A run may write a *proposed* runbook as an ordinary artifact, and only a reviewed proposal becomes a machine class.** That is one sentence, it is enforceable by the ownership lint that already exists in `src/ownership.rs`, and it is the entire recursive-self-improvement story.

---

## The strongest argument against this recommendation

**You may already have all of it, and this may be over-engineering.**

Every path in the engine is root-relative. `StateStore::new` joins `.arca/state.toml` to a root; `Scheduler::open` joins `.arca/ratmac.toml` to a root; the log, the lock, and the evidence tree all hang off the same root. **A git worktree is its own root.** Therefore, today, with no code change whatsoever:

- N worktrees are N independent runs.
- Each has its own runbook, its own state file, its own append-only log, its own lock, and its own evidence tree.
- Each has exactly one writer, and the invariant holds per file without generalisation.
- Fan-in is already available: the parent's `record_contract` and `completion_gate` guards read receipts from disk and refuse until every declared check is green. A guard that counts **is** a barrier.

So the cheapest thing that could possibly work is: the parent creates worktrees, N independent runs happen, the parent's existing guards refuse to advance until all N have landed their receipts. That is per-subtask instances, isolation, fan-out, and fan-in, at a cost of **zero new formalism, zero change to position, zero change to the doctor** — and the only genuinely missing piece is a naming convention linking a parent run to its children.

Against that baseline, the parallel-phase machinery has to justify itself, and it has exactly one honest justification: **without a declaration, the fan-out is invisible to offline analysis.** If the parallel structure lives in whoever wrote the shell script that made the worktrees, then the parallelism is process-as-prompt again — undeclared, unlinted, unreviewable — which is the precise failure mode the project exists to attack. The fan-out declaration is what makes the branch structure a thing `rtm doctor` can see.

But notice the shape of that justification. It is a **thesis** argument, not a **need** argument. It says the structure ought to be data because data is the bet; it does not say anything breaks without it. The feasibility direction note already warns that ratmac must beat a much cheaper baseline on measured outcomes rather than on feel, and the same test applies here. The disciplined order is therefore:

1. Run the worktree arrangement by hand, with today's engine, unchanged.
2. See whether the missing declaration actually costs anything — a wrong fan-out that a linter would have caught, a stall nobody noticed, a branch whose position could not be recovered.
3. Add the fan-out phase only if step 2 produces a real defect that a static check would have prevented.

If step 2 comes back empty, the correct action is to write down the worktree convention and add nothing to the engine. This survey's recommendation is what to build **if** the declaration turns out to be needed — not an argument that it is.

---

## Sources

1. https://en.wikipedia.org/wiki/Petri_net — places, transitions, tokens, markings; state machine nets as the one-token subclass; boundedness; reachability decidability (Mayr 1981), non-elementary lower bound (2018), Ackermann-completeness (Leroux; Czerwiński & Orlikowski, 2021); free-choice nets; workflow nets, soundness, well-handled nets, and Well-handled nets with Regular Iteration as sound-by-construction composition (Ping et al., 2004)
2. https://www.vdaalst.rwth-aachen.de/publications/p98.pdf — van der Aalst, *The Application of Petri Nets to Workflow Management*: the workflow-net definition, the three soundness conditions, the theorem that soundness equals liveness plus boundedness of the short-circuited net, the well-handled / well-structured / S-coverable structural notions, and the corollaries giving polynomial-time soundness for free-choice and well-structured workflow nets
3. https://arxiv.org/abs/2201.05588 — Blondin, Mazowiecki and Offtermatt (Logic in Computer Science, 2022), "The Complexity of Soundness in Workflow Nets": classical and structural soundness EXPSPACE-complete, generalised soundness PSPACE-complete. This supersedes the widely repeated but unproven "EXPSPACE-hard" folklore claim, whose usual attribution is to a paper that proved PSPACE-completeness for 1-safe nets
4. https://docs.temporal.io/workflows — Event History as the ordered source of truth; replay by restarting the code and walking history forward rather than restoring a snapshot; the determinism requirement and its named hazards (clock, randomness, unwrapped network calls)
5. https://docs.aws.amazon.com/step-functions/latest/dg/state-parallel.html — Amazon States Language `Parallel` state: branch array, wait-for-all-branches join, the self-contained-branch rule, branch failure semantics
6. https://docs.aws.amazon.com/step-functions/latest/dg/state-map.html — Amazon States Language `Map` state: one `ItemProcessor` sub-machine over a data-supplied item set; Distributed mode giving each iteration its own separate execution history
7. https://homepages.inf.ed.ac.uk/kousha/toplas2005.pdf — Alur, Benedikt, Etessami, Godefroid, Reps and Yannakakis, *Analysis of Recursive State Machines* (Transactions on Programming Languages and Systems 27(4), 2005; earlier at Computer Aided Verification 2001 and International Colloquium on Automata, Languages and Programming 2001). The `O(nθ²)` time and `O(nθ)` space bound with `θ` the maximum over components of the minimum of entry and exit counts; the bisimilarity of pushdown systems with recursive state machines and of context-free systems with single-exit machines; and the entry/exit complexity table (Table III) showing single-entry machines linear for reachability, cycle detection and linear-time properties
8. https://www.cis.upenn.edu/~alur/Stoc04.pdf — Alur and Madhusudan, *Visibly Pushdown Languages* (Symposium on Theory of Computing, 2004): the call/return/local alphabet partition, the closure and decision-problem tables against regular, context-free and deterministic context-free languages, determinisation blow-up, EXPTIME-completeness of inclusion and universality, and the observation that complexity is "polynomial in the model and exponential only in the specification". Journal version: https://www.cis.upenn.edu/~alur/Jacm09.pdf (Journal of the Association for Computing Machinery 56(3), 2009), where the determinisation bound is proved tight
9. https://www.cs.umd.edu/~nau/papers/erol1996complexity.pdf — Erol, Hendler and Nau, *Complexity results for HTN planning* (Annals of Mathematics and Artificial Intelligence 18(1), 1996): the complexity table, undecidability by reduction from context-free grammar intersection emptiness, and the statement that restricting methods to be regular produces STRIPS-style planning as right-linear grammars produce regular sets
10. https://ojs.aaai.org/index.php/ICAPS/article/download/13721/13570 — Alford, Bercher and Aha, *Tight Bounds for HTN Planning* (International Conference on Automated Planning and Scheduling, 2015): the completeness results that close Erol/Hendler/Nau's open bounds, including EXPTIME-completeness for totally-ordered propositional planning
11. https://www.uni-ulm.de/fileadmin/website_uni_ulm/iui.inst.090/Publikationen/2014/Hoeller2014HtnLanguages.pdf — Höller, Behnke, Bercher and Biundo, *Language Classification of Hierarchical Planning Problems* (European Conference on Artificial Intelligence, 2014): totally-ordered hierarchical task network languages are exactly the context-free languages, and partially-ordered ones lie strictly between context-free and context-sensitive
12. https://grim7reaper.github.io/static/misc/valgrind/what-are-race-conditions.pdf — Netzer and Miller, *What are Race Conditions?* (Letters on Programming Languages and Systems 1(1), 1992): exhaustive race detection is efficient only for synchronization incapable of implementing mutual exclusion, naming fork/join, and NP-hard for stronger primitives
13. https://people.csail.mit.edu/jfineman/sporder.pdf — Bender, Fineman, Gilbert and Leiserson, *On-the-Fly Maintenance of Series-Parallel Relationships in Fork-Join Multithreaded Programs* (Symposium on Parallelism in Algorithms and Architectures, 2004): every fork matched by a join gives a series-parallel parse tree; `O(1)` amortised per operation, so a fork-join program is race-checked in `O(T₁)` time
14. https://cilk.mit.edu/tools/ — Cilk's SP-bags determinacy-race detector and its `O(T₁·α(v,v))` bound, with `v` the number of shared memory locations
15. https://pure.itu.dk/files/83523575/a9_honda.pdf — Honda, Yoshida and Carbone, *Multiparty Asynchronous Session Types* (Journal of the Association for Computing Machinery 63(1), 2016; originally Principles of Programming Languages, 2008): the global type as a shared contract, type checking by projection onto each peer, and the three guarantees — communication safety, progress, and session fidelity
16. https://github.com/temporalio/sdk-go/blob/master/contrib/tools/workflowcheck/README.md — Temporal's Go determinism analyser and its stated inability to catch global variable mutation, with the reason that deterministic and nondeterministic use "cannot be reliably distinguished" syntactically
17. https://github.com/temporalio/sdk-java/tree/master/temporal-workflowcheck — Temporal's Java bytecode analyser, self-described as beta quality, with its rejected-alternatives list (CodeQL "Too slow")
18. https://learn.microsoft.com/en-us/azure/durable-task/common/durable-task-code-constraints — Azure Durable Functions orchestrator code constraints, including the admission that the runtime detector "won't catch all violations" and that the constraint list "isn't comprehensive"
19. https://www.nuget.org/packages/Microsoft.DurableTask.Analyzers — the Roslyn determinism analysers, where every orchestrator-determinism rule is Warning or Info severity and only binding misuse is an Error
20. https://docs.aws.amazon.com/step-functions/latest/apireference/API_ValidateStateMachineDefinition.html — offline validation of a state machine definition without creating a resource, recommended for pre-commit hooks, with Amazon's own description of the warning mode as "static analysis"
21. https://docs.aws.amazon.com/toolkit-for-vscode/latest/userguide/bulding-stepfunctions.html — the editor tooling's enumerated checks, including "Non terminal state" and "Nonexistent states that are pointed to"
22. https://states-language.net/spec.html — the Amazon States Language specification, including the rule that a state in one branch must not reference a variable assigned in another
23. https://cocotec.io/fdr/manual/optimising/compression.html — FDR's state-space compression guidance: apply to components rather than the whole system, and push hiding as far down the process tree as possible
24. https://arxiv.org/abs/1502.00944 — D'Osualdo and Ong, on depth-boundedness in the pi-calculus: the fragment enjoys decidability of important verification problems, but membership in it is undecidable
25. https://www.cis.upenn.edu/~alur/Zohar03.pdf — Alur, *Formal Analysis of Hierarchical State Machines* (2003): the reachability complexity cube over hierarchy, concurrency and variables (NLOGSPACE / PTIME / PSPACE / EXPSPACE); linear reachability and cycle detection for hierarchical machines; linear-time model checking PSPACE-complete with hardness inherited from flat structures; branching-time cost exponential in exit-node count and proved unavoidable
26. https://doi.org/10.1145/349214.349241 — Ramalingam, *Context-sensitive synchronization-sensitive analysis is undecidable* (Transactions on Programming Languages and Systems 22(2), 2000): concurrency combined with recursion is undecidable "even for the simplest analysis problems" — the result that forces bounded nesting depth
27. https://doi.org/10.1145/322374.322380 — Brand and Zafiropulo, *On Communicating Finite-State Machines* (Journal of the Association for Computing Machinery 30(2), 1983): unbounded channels make communicating finite-state machines Turing-powerful, which is why an unbounded event queue in the UML and SCXML execution semantics is undecidable before any guard is written
28. https://link.springer.com/content/pdf/10.1007/3-540-36576-1_8.pdf — Bertrand and Schnoebelen (Foundations of Software Science and Computation Structures, 2003), stating the Brand–Zafiropulo attribution directly; with Schnoebelen (Information Processing Letters 83(5), 2002, https://doi.org/10.1016/s0020-0190(01)00337-4) on lossy channel systems, where what remains decidable is not primitive recursive
29. http://www.cs.rice.edu/~vardi/papers/conc97rj.ps.gz — Harel, Kupferman and Vardi, *On the Complexity of Verifying Concurrent Transition Systems* (Concurrency Theory, 1997): containment EXPSPACE-complete against PSPACE-complete sequentially, model checking PSPACE-complete even for a fixed formula, and the conclusion that "the state-explosion problem cannot be avoided"
30. https://en.wikipedia.org/wiki/UML_state_machine — Harel's 1987 paper as the precursor; nesting as "programming by difference" and event propagation to the enclosing state; additive-versus-multiplicative complexity of orthogonal regions; run-to-completion and its longest-step cost; innermost-first priority; guard evaluation order left unspecified; guard and action expression syntax undefined by the specification
31. https://www.erlang.org/doc/system/sup_princ.html — Open Telecom Platform supervision principles: the four restart strategies, restart intensity and period with escalation to the parent, child specifications returned by `init/1` at runtime, static versus dynamic children, reverse-order shutdown, and restart budgets multiplying down the tree
32. https://www.erlang.org/doc/apps/dialyzer/dialyzer.html — Dialyzer's stated scope: success typings, "sound warnings without false positives", definite type errors and unreachable code; no mention of concurrency or message protocols
33. https://github.com/WhatsApp/eqwalizer/blob/main/FAQ.md — the authors' own contrast between a type checker and a static analysis tool, and the resulting asymmetry in what each reports
34. https://doi.org/10.1145/3798267 — Fowler and Hu, Maty (Object-Oriented Programming, Systems, Languages and Applications, April 2026): session-typed actors extended with Erlang-style supervision and cascading failure, metatheory preserved
35. https://doi.org/10.1145/1926385.1926435 — Deniélou and Yoshida, "Dynamic Multirole Session Types" (Principles of Programming Languages, 2011): the theory behind a statically declared protocol with a runtime-determined participant count
36. https://en.wikipedia.org/wiki/Communicating_finite-state_machine — the Brand and Zafiropulo boundary: decidable for a single message type between two machines, undecidable at two or more
37. https://dspace.mit.edu/handle/1721.1/6952 — Agha, *Actors: A Model of Concurrent Computation in Distributed Systems* (MIT AITR-844, 1986)
38. `.arca/steering.md` (this repository) — Ideal shape, invariants, non-goals, and the two open questions on ownership enforcement and routing expressiveness
39. `.arca/runbook-spec.md` (this repository) — the runbook format, the closed guard-kind vocabulary, the ownership table, and the `RB*` diagnostic codes
40. `src/graph.rs`, `src/doctor.rs`, `src/contract.rs`, `src/state.rs`, `src/model.rs` (this repository) — the straight-walk routing rule, the four doctor passes, the existing ticket dependency graph with cycle detection, the seven required state fields, and root-relative state file resolution
41. `.arca/dict.md` (this repository) — `active_refs` as a specified but unpopulated state field
42. `.arca/research/ratmac-feasibility/direction.md` (this repository) — the cheaper-baseline discipline this recommendation is measured against
