# Theory: state machines, graphs and related models

**Date:** 2026-11-21
**Producing agent:** `astra-theory-models` (astra-planner)
**Method:** web research only; formal results are the authors' claims, not reproduced
**Known defects:** None known. Many sources are older than 2025, as marked. Items 16 and 37–40 are the researcher's inferences.

---

## FSMs and graphs

1. **DOCUMENTED:** A DFA is a finite, input-labelled graph with an initial state and accepting states. [2025](https://web.stanford.edu/class/archive/cs/cs103ace/cs103ace.1254/materials/Week7.pdf)

2. **DOCUMENTED:** A DFA has one current state, whereas an NFA represents alternative possible runs rather than requiring concurrent execution of every branch. [2025-09-24](https://courses.knox.edu/cs375/notes/NFAs2.pdf)

3. **DOCUMENTED:** Subset construction preserves accepted language while converting an n-state NFA into a DFA with at most 2ⁿ states. [2025-09-24](https://courses.knox.edu/cs375/notes/NFAs2.pdf)

4. **DOCUMENTED:** Moore outputs depend on states, whereas Mealy outputs depend on states and inputs and appear on transitions. [Spring 2026](https://courses.cs.washington.edu/courses/cse371/26sp/lectures/02/EECSE371-Sp26-02-FSM.pdf)

5. **CLAIMED:** Language equivalence compares accepted traces, whereas bisimulation also requires matching branching behaviour step by step. [2005-07; older](https://www.cis.upenn.edu/~alur/toplas2005.pdf)

## Statecharts, hierarchy and stacks

6. **DOCUMENTED:** An active SCXML compound state has one active immediate child, together with that child’s active descendants. [2015-09-01; older](https://www.w3.org/TR/2015/REC-scxml-20150901/)

7. **DOCUMENTED:** An active SCXML parallel state activates every child region simultaneously. [2015-09-01; older](https://www.w3.org/TR/2015/REC-scxml-20150901/)

8. **DOCUMENTED:** SCXML shallow history remembers immediate children, while deep history remembers the previously active descendant configuration. [2015-09-01; older](https://www.w3.org/TR/2015/REC-scxml-20150901/)

9. **CLAIMED:** Hierarchical machines can describe finite state spaces exponentially larger than their compact definitions. [2005-07; older](https://www.cis.upenn.edu/~alur/toplas2005.pdf)

10. **CLAIMED:** Recursive state machines add an unbounded call stack and translate to and from pushdown systems in linear time. [2005-07; older](https://www.cis.upenn.edu/~alur/toplas2005.pdf)

11. **CLAIMED:** Recursive machines express context-free behaviour beyond regular FSM languages while retaining decidable reachability. [2005-07; older](https://www.cis.upenn.edu/~alur/toplas2005.pdf)

## Petri nets and workflow nets

12. **DOCUMENTED:** A Petri net’s state is its complete token marking, potentially marking many places and enabling several transitions. [2022-04-25; older](https://teaching.model.in.tum.de/2021ss/petri/material/PNSkript.pdf)

13. **DOCUMENTED:** A bounded Petri net has a finite reachability graph whose nodes represent whole markings, not individual places. [2022-04-25; older](https://teaching.model.in.tum.de/2021ss/petri/material/PNSkript.pdf)

14. **CLAIMED:** Workflow nets require one source place, one sink place, and every node on a source-to-sink path. [2008-11; older](https://eprints.qut.edu.au/224464/1/7115c.pdf)

15. **CLAIMED:** Classical soundness requires every reachable marking to retain a path to clean completion and every transition to be usable. [2022-01-14; older](https://arxiv.org/html/2201.05588)

16. **OPINION — inference:** Soundness therefore guarantees completion remains possible, not that every scheduler or agent choice eventually completes. [2022-01-14; older](https://arxiv.org/html/2201.05588)

17. **CLAIMED:** Classical workflow-net soundness is EXPSPACE-complete, illustrating that decidability does not imply practical checking costs. [2022-01-14; older](https://arxiv.org/html/2201.05588)

18. **CLAIMED:** Ordinary Petri-net reachability is decidable but Ackermann-complete. [2022-10-25 revision; older](https://arxiv.org/abs/2104.13866)

19. **CLAIMED:** Ordinary Petri-net termination remains decidable despite potentially unbounded markings. [2017-07-04; older](https://arxiv.org/html/1707.01157)

20. **CLAIMED:** Generalised workflow soundness becomes undecidable with reset arcs that clear places. [2025-03-06 preprint; conference version 2024](https://arxiv.org/html/2503.04440)

## Dataflow and DAGs

21. **DOCUMENTED:** A DAG’s topological ordering respects dependencies but need not describe a connected execution path through the graph. [2025-08-06](https://web.stanford.edu/class/archive/cs/cs106b/cs106b.1258/lectures/25-graphs/)

22. **CLAIMED:** Dataflow graphs carry token streams through channels, and feedback channels can preserve state between actor firings. [1995-05; older](https://ptolemy.berkeley.edu/publications/papers/95/processNets/proceedings.pdf)

23. **CLAIMED:** Kahn-style networks use concurrent processes and FIFO channels, with deterministic stream histories under their model assumptions. [1995-05; older](https://ptolemy.berkeley.edu/publications/papers/95/processNets/proceedings.pdf)

24. **CLAIMED:** General dataflow deadlock is undecidable, whereas fixed-rate synchronous dataflow permits static deadlock analysis. [2006; older](https://ptolemy.berkeley.edu/publications/papers/06/causalityInterfaces_EMSOFT06/CausalityInterfaces_EMSOFTpaper_ZHOU.pdf)

## Pregel and bulk-synchronous execution

25. **CLAIMED:** Pregel stores vertex and edge values, processes previous-superstep messages, and delivers newly sent messages in the following superstep. [2010; older](https://www.cs.ucdavis.edu/~amenta/f15/pregel.pdf)

26. **DOCUMENTED:** LangGraph’s bulk-synchronous execution runs selected nodes concurrently and makes their updates visible after the step finishes. [2025-09-04](https://blog.langchain.com/building-langgraph/)

27. **DOCUMENTED:** LangGraph separates channel-held data from subscriptions and channel versions that determine which nodes execute next. [2025-09-04](https://blog.langchain.com/building-langgraph/)

## Behaviour trees

28. **CLAIMED:** Behaviour trees tick children returning Running, Success or Failure, while parallel nodes can manage several running children. [2025-02-17](https://arxiv.org/html/2502.11904v1)

29. **CLAIMED:** A published translation maps behaviour trees into Fiacre timed transition systems for model checking and runtime verification. [2025-02-17](https://arxiv.org/html/2502.11904v1)

30. **CLAIMED:** One stateful-tree formalisation is FSM-equivalent with finite blackboards but Turing-complete when suitable unbounded-integer blackboards are allowed. [2024-11-21; older](https://arxiv.org/html/2411.14165v1)

## BPMN and actors

31. **DOCUMENTED:** BPMN distinguishes control tokens on sequence flows from data associations and allows parallel gateways to split and synchronise control. [2014-01; older](https://www.omg.org/spec/BPMN/2.0.2/PDF)

32. **CLAIMED:** Dijkman’s BPMN-to-Petri-net translation covers a control-flow subset and replaces gateway data predicates with nondeterministic choices. [2008-11; older](https://eprints.qut.edu.au/224464/1/7115c.pdf)

33. **DOCUMENTED:** Akka actors hold local state and mailboxes, handle messages sequentially, and execute concurrently with other actors. [Undated; accessed 2026-11-21](https://doc.akka.io/libraries/akka-core/current/typed/guide/actors-intro.html)

34. **CLAIMED:** Finite-state message-passing processes with unbounded communication channels can simulate Turing machines. [2026-05-27](https://arxiv.org/pdf/2407.06968)

35. **CLAIMED:** Global-state reachability is PSPACE-complete for mailbox systems whose executions can be reordered into the paper’s send-then-receive rounds. [2026-05-27](https://arxiv.org/pdf/2407.06968)

## Checking and the premise

36. **CLAIMED:** Concurrent finite processes can become one global automaton through Cartesian-product construction, causing state explosion. [2005; older](https://spinroot.com/gerard/pdf/Advances2005.pdf)

37. **OPINION — inference:** For a closed, fully specified finite model, graph analysis can decide reachability, deadlock and whether every run terminates. [2005; older](https://spinroot.com/gerard/pdf/Advances2005.pdf)

38. **OPINION — inference:** An acyclic orchestration graph does not establish termination when node implementations or external calls can run forever. [2005; older](https://spinroot.com/gerard/pdf/Advances2005.pdf)

39. **OPINION — inference:** The premise holds when “graph” means a complete finite transition system whose states include data and concurrent configurations. [2005; older](https://spinroot.com/gerard/pdf/Advances2005.pdf)

40. **OPINION — inference:** It breaks when “graph” means a task drawing, because storage bounds, stacks, queues and scheduling determine behaviour and expressive power. [2026-05-27](https://arxiv.org/pdf/2407.06968)

## Open questions

- Which equivalence matters: accepted traces, branching behaviour, timing, or observable tool effects?
- Must task count, mailbox size, data domains and recursion depth be bounded?
- Does “completion” mean completion is possible, inevitable, or merely likely?
- What assumptions govern fairness, cancellation, retries and indefinitely blocked tools?

## Could not verify

- A universal translation preserving all these models’ data, concurrency, timing and cancellation semantics.
- Empirical evidence that their representational equivalences imply equal usability for language models.
- A publication date for the living Akka documentation.
- Independent reproduction of the cited proofs; **CLAIMED** denotes authors’ stated formal results, not experimental measurements.

## Source list

- Stanford, *DFAs, NFAs & Regular Languages*, Winter 2025.
- Knox College, *NFAs and Regular Expressions*, 2025-09-24.
- University of Washington, *FSM Review*, Spring 2026.
- W3C, *SCXML 1.0 Recommendation*, 2015-09-01.
- Alur et al., *Analysis of Recursive State Machines*, July 2005.
- Esparza, *Petri Nets Lecture Notes*, 2022-04-25.
- Blondin et al., *The Complexity of Soundness in Workflow Nets*, 2022.
- Czerwiński and Orlikowski, *Reachability in Vector Addition Systems Is Ackermann-complete*, revised 2022.
- Akshay et al., *On Petri Nets with Hierarchical Special Arcs*, 2017.
- Blondin et al., *Soundness of Reset Workflow Nets*, 2025 preprint.
- Stanford, *Graphs*, 2025-08-06.
- Lee and Parks, *Dataflow Process Networks*, May 1995.
- Zhou and Lee, *A Causality Interface for Deadlock Analysis in Dataflow*, 2006.
- Malewicz et al., *Pregel*, 2010.
- LangChain, *Building LangGraph*, 2025-09-04.
- Ingrand, *A Formal Implementation of Behavior Trees to Act in Robotics*, 2025.
- Serbinowska et al., *Formalizing Stateful Behavior Trees*, 2024.
- OMG, *BPMN 2.0.2*, January 2014.
- Dijkman et al., *Semantics and Analysis of Business Process Models in BPMN*, 2008.
- Akka, *How the Actor Model Meets the Needs of Modern, Distributed Systems*, undated.
- Delpy et al., *An Automata-based Approach for Synchronizable Mailbox Communication*, 2026.
- Holzmann, *Software Model Checking with SPIN*, 2005.