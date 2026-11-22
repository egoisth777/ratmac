# ratmac - product definition

**Defined:** 2026-11-22, by Billy.
**Status:** locked.

## Change rule

This file is the core product definition of ratmac. It does not change with a sprint, a goal, or an issue.

- An agent MUST NOT edit this file, unless Billy starts a conversation as a product revision.
- Only Billy decides a change. In a product revision, an agent may write the change that Billy decides.
- An agent that finds a conflict between this file and any other file reports it to Billy. It does not edit this file to remove the conflict.
- Direction flows from this file to `steering.md`, then to the goal, then to issues, then to tickets.

## What ratmac is

ratmac is a tiny, robust runner for work that follows a graph of steps. It defines the graph as a finite state machine, walks it, and executes it. It works with any agent.

## Core problem

When an AI agent says a step of a multi-step process is done, nobody can trust it without checking the work themselves.

The rule that follows from it: a step is done only when checks on the real state pass. This applies whether ratmac, a script, or an agent ran the step. A step's own return or claim never counts.

## Core missions

Each mission states a problem and the approach Billy chose for it.

### Mission 1 - a reliable local engine

- **Problem:** the tool that decides whether work is done must itself not fail, and must need no server and no setup.
- **Chosen approach:** a local engine written in Rust, a memory-safe language. It keeps everything in text files and runs no background service.

### Mission 2 - parallel traversal, orchestration, and execution

- **Problem:** many steps and many tasks must run side by side without blocking or damaging each other.
- **Chosen approach:** a multi-threaded engine.
  - **Traversal:** many branches of one graph, and many graphs, are active at once. Each has its own state on disk.
  - **Orchestration:** ratmac decides what can start, what must wait, and where branches join.
  - **Execution:** ratmac starts mechanical steps itself (scripts and commands). For an agent step, ratmac names the step and waits; an outside agent picks it up. ratmac does not launch agents.
- **Bound by the core problem:** running a step never proves the step. Every step is done only when its checks pass.

### Mission 3 - a language that agents already know

- **Problem:** an agent must write and follow a graph definition correctly. An unfamiliar format is likely to cause mistakes.
- **Chosen approach:** the graph is defined in a general scripting or markup language that agents have seen in very large amounts during training. The language is not chosen yet.

## Evidence status on 2026-11-22

The research is in [the agent graph engineering study](research/re-agent-graph-engineering/synthesis.md). Nothing in it was run or repeated by us.

- **Core problem:** supported by measured results from the sources. In one study of 9,876 runs, false completion made up 45-48% of failures in two task areas and 3% in a third (report 11, item 2). In one survey, 74% of respondents with deployed agents depended primarily on human evaluation (report 09, item 28).
- **Not established:** that checking the real state is the cure. The authors of the false completion study call it an observation, not a cause (report 10, item 32). The same study reports that simple classifiers over each run's transcript separate false completions fairly well, within limits its authors state (report 10, items 30 and 33).
- **ratmac's value is a hypothesis.** Other tools can host checks. The hypothesis is that making the checks mandatory for every step, and easy to write, is worth a product. StateM works on the same problem (report 06).
- **Mission 1:** a requirement on us. No finding shows that buyers ask for it.
- **Mission 2:** supported by user complaints and one bug report in other tools. Not measured.
- **Mission 3:** unproven. How well models write and follow graphs in each candidate language was not measured in a matched test that our research could verify (report 12).

## What this file does not decide

- The language of mission 3.
- How many things may run at once, and the rules for two branches that write the same file.
- The product rules of the engine as built today. Those stay in `goal/` until a planning pass folds this definition in.
