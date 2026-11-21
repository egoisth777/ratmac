# Process formats that models already know

**Date:** 2026-11-21
**Producing agent:** `astra-format-fluency` (astra-planner)
**Method:** web research only; no model was tested by us
**Known defects:** None known. Popularity counts measure package or question volume, not training coverage.

---

## Evidence boundaries

*U = source undated; accessed 2026-11-21; download counts cover 2026-11-01–07.*

1. **DOCUMENTED — 2024-02-29, older:** StarCoder2 subsampled JSON, XML and YAML training data to 8GB each, and Graphviz DOT to 1GB. [Training]

2. **DOCUMENTED — U:** OpenAI supports constrained JSON Schema subsets and custom Lark/regex grammars, but these do not guarantee correct workflow behaviour. [Structured] [Grammar]

3. **OPINION — inference, U:** JSON forms of XState, ASL and Open Workflow fit schema-constrained generation; YAML needs serialization, while XML, diagram languages and code need custom grammars. [Structured] [Grammar]

4. **OPINION — inference, 2025-07-15:** All candidates are text-diffable, but formatting stability matters, and BPMN’s graphical-layout XML adds substantial noise. [Representations]

## XState JSON/TypeScript

5. **MEASURED — 2026-11-01–07:** `xstate` received **8,648,144 npm downloads**, which measures package activity rather than machine definitions or training examples. [Downloads]

6. **DOCUMENTED — U:** MIT-licensed XState supports cycles, parallel states, guards and context, but JSON serialization leaves function implementations outside the document. [XState] [Serialization]

## SCXML

7. **MEASURED — 2026-11-21:** Stack Overflow had **62 `scxml` questions**. [Questions]

8. **DOCUMENTED — 2015-09-01, older; parser U:** SCXML defines guarded cycles, parallel regions and data models, with script semantics depending on the selected data model; Apache Commons SCXML provides an Apache-2.0 parser/engine. [SCXML] [Commons]

## Amazon States Language

9. **MEASURED — 2026-11-01–07:** `asl-validator` received **435,760 npm downloads**, not necessarily that many distinct ASL authors. [Downloads]

10. **DOCUMENTED — U:** ASL JSON supports cycles, Choice guards, Map/Parallel processing and data transformations, but prohibits transitions across parallel-branch boundaries. [ASL]

11. **DOCUMENTED — U:** `asl-validator` uses JSON schemas and path checks under Apache-2.0, rather than proving successful task execution. [ASLValidator]

## BPMN XML

12. **MEASURED — 2026-11-01–07:** `bpmn-moddle` received **662,637 npm downloads**. [Downloads]

13. **DOCUMENTED — 2014, older; parser U:** BPMN XML expresses loops, parallel gateways, conditions and data associations, while MIT-licensed `bpmn-moddle` parses XML rather than validating service behaviour. [BPMN] [Moddle]

14. **MEASURED — 2025-07-15:** Llama-3.3-70B’s BPMN generation scored **0.43 overall element similarity** across 55 process descriptions, not execution success. [Representations]

## Mermaid stateDiagram and flowchart

15. **MEASURED — 2026-11-01–07:** `mermaid` received **25,860,923 npm downloads**, without separating diagram types. [Downloads]

16. **DOCUMENTED — U:** `stateDiagram` depicts cycles, fork/join and concurrent regions, while `flowchart` supplies graph notation; neither specifies executable guards or data passing. [MermaidState] [MermaidFlow]

17. **DOCUMENTED — U:** MIT-licensed Mermaid exposes `mermaid.parse` for syntax validation, not workflow validation. [Mermaid] [MermaidAPI]

18. **MEASURED — 2025-12-03:** Flowchart2Mermaid gave GPT-4o a **1.000 mean judge-assigned syntax score** across 200 images, not a parser-confirmed success rate. [FlowchartStudy]

## Graphviz DOT

19. **MEASURED — 2026-11-21:** Stack Overflow had **3,008 `graphviz` questions**, covering more than workflow notation. [Questions]

20. **DOCUMENTED — U:** DOT represents cyclic graphs and labelled branches without scheduling, data-flow or guard semantics, and the current Graphviz parser is EPL-2.0. [DOT] [GraphvizLicence]

21. **MEASURED — 2025-07-15:** Llama-3.3-70B’s DOT outputs omitted **7.96 nodes on average** in the 55-process representation study. [Representations]

## PlantUML

22. **MEASURED — 2026-11-21:** Stack Overflow had **470 `plantuml` questions**. [Questions]

23. **DOCUMENTED — U:** PlantUML depicts parallel states and cyclic transitions without executing them, and parser builds are available under GPL, MIT and Apache licences. [PlantState] [PlantLicences]

24. **MEASURED — 2026-04-07:** OmniDiagram reported **83 PlantUML syntax errors** for its 7B model, although this was diagram generation rather than workflow execution. [OmniDiagram]

## GitHub Actions YAML

25. **MEASURED — 2026-11-21:** Stack Overflow had **11,209 `github-actions` questions**, not a count of YAML workflow files. [Questions]

26. **DOCUMENTED — U:** Actions supports acyclic job dependencies, conditional execution, matrix parallelism and outputs, with MIT-licensed `actionlint` checking the workflow dialect. [Actions] [Actionlint]

27. **MEASURED — 2025-07-23:** A six-model CI-generation study reported up to **69% reference similarity** but only **3% perfect matches**, not equivalent execution rates. [CIStudy]

## Makefile

28. **MEASURED — 2026-11-21:** Stack Overflow had **25,645 `makefile` questions**. [Questions]

29. **DOCUMENTED — U:** GPL-3.0-or-later GNU Make parses and parallelizes dependency tasks but drops dependency cycles, leaving runtime loops and data exchange to recipes, variables and files. [Make] [MakeManual]

## Open Workflow Specification

30. **MEASURED — 2026-11-01–07:** The renamed `@openworkflowspec/sdk` package received **16,406 npm downloads**. [WorkflowDownloads]

31. **DOCUMENTED — U:** Its JSON/YAML DSL defines `for`/`while`, `fork`, `switch` and input/output mappings, while its Apache-2.0 TypeScript SDK validates document structure. [WorkflowDSL] [WorkflowSDK]

32. **DOCUMENTED — 2025-01-27:** Version 1.0 merged states and actions into tasks, so exposure to older Serverless Workflow examples does not establish knowledge of the newer grammar. [WorkflowRelease]

## YAML-based agent formats

33. **MEASURED — 2026-11-21:** Stack Overflow had **38 `crewai` questions**, providing no YAML-dialect-specific prevalence measurement. [Questions]

34. **DOCUMENTED — U:** CrewAI YAML defines agents and tasks while Python selects the process, and MIT-licensed PyYAML validates YAML syntax rather than orchestration behaviour. [Crew] [PyYAML]

35. **DOCUMENTED — U:** Microsoft’s declarative workflow YAML instead contains workflow logic, illustrating why “agent YAML” is not one interchangeable grammar. [AgentYAML]

## Plain Python and shell

36. **MEASURED — 2026-11-21:** Stack Overflow had **2,204,338 `python` questions** and **156,528 `bash` questions**, neither restricted to orchestration. [Questions]

37. **DOCUMENTED — U:** Python and Bash can express loops, conditions, data exchange and concurrent work, rather than restricting execution to a static graph. [PythonAST] [PythonTasks] [Bash]

38. **DOCUMENTED — U:** CPython’s AST parser uses the PSF licence and ShellCheck uses GPL-3.0-or-later, but successful parsing does not establish correct execution. [PythonLicence] [PythonAST] [ShellCheck]

39. **MEASURED — 2025:** BigCodeBench reported GPT-4o solving roughly **60%** of 1,140 Python completion tasks and fewer than half of instruction-only variants. [BigCodeBench]

40. **MEASURED — 2025-03-18:** EnvBench’s GPT-4o Bash Agent successfully configured **196/665 JVM repositories**, judged through compilation checks rather than complete runtime tests. [EnvBench]

## JSON Schema-described structures

41. **MEASURED — 2026-11-01–07:** Ajv received **522,655,870 npm downloads**, largely measuring validator dependency traffic rather than workflow-schema adoption. [Downloads]

42. **DOCUMENTED — U:** MIT-licensed Ajv validates JSON against schemas, but JSON Schema itself defines no loop, parallel-branch, scheduling or data-passing execution semantics. [Ajv] [JSONSchema]

43. **MEASURED — 2025-02-27:** JSONSchemaBench tested approximately **10,000 real-world schemas** and found substantial differences in decoder coverage, so schema availability alone does not establish constrained-generation support. [SchemaBench]

## Open questions

- Which matters: valid syntax, faithful process meaning, or successful execution?
- Must the file contain executable guards and transformations, or reference separately installed functions?
- Which exact grammar versions and model snapshots must interoperate?

## Could not verify

- Audited per-format training counts for current proprietary models: **“huge training coverage” remains an assumption**, not established by downloads or questions.
- Dedicated generation/interpretation accuracy benchmarks for XState, SCXML, ASL, Makefile, Open Workflow, Mermaid `stateDiagram`, or individual agent-YAML dialects.
- A matched benchmark covering all candidates, or measured comparative diff quality.
- OmniDiagram’s denominator for the reported PlantUML error count; it should not be converted into an accuracy percentage.

## Sources

- **Activity:** [npm downloads][Downloads], [Open Workflow downloads][WorkflowDownloads] — 2026-11-01–07; [Stack Overflow API][Questions] — 2026-11-21.
- **Training and decoding:** [StarCoder2][Training] — 2024-02-29, older; [Structured Outputs][Structured] and [custom grammars][Grammar] — U.
- **State machines:** [XState][XState], [serialization][Serialization], [Commons SCXML][Commons], [ASL specification][ASL], [ASL validator][ASLValidator] — U; [SCXML recommendation][SCXML] — 2015-09-01, older.
- **BPMN:** [OMG specification][BPMN] — 2014, older; [bpmn-moddle][Moddle] — U; [representation experiment][Representations] — 2025-07-15.
- **Diagrams:** [Mermaid states][MermaidState], [flowcharts][MermaidFlow], [repository/licence][Mermaid], [parser API][MermaidAPI], [DOT grammar][DOT], [Graphviz licence][GraphvizLicence], [PlantUML states][PlantState], [licence variants][PlantLicences] — U.
- **Diagram experiments:** [Flowchart2Mermaid][FlowchartStudy] — 2025-12-03; [OmniDiagram][OmniDiagram] — 2026-04-07.
- **CI/build:** [Actions syntax][Actions], [actionlint][Actionlint], [GNU Make][Make], [Make manual][MakeManual] — U; [CI-generation study][CIStudy] — 2025-07-23.
- **Workflow dialects:** [Open Workflow DSL][WorkflowDSL], [SDK][WorkflowSDK], [CrewAI][Crew], [PyYAML][PyYAML], [Microsoft declarative workflows][AgentYAML] — U; [Workflow 1.0 release][WorkflowRelease] — 2025-01-27.
- **Scripts:** [Python AST][PythonAST], [task groups][PythonTasks], [licence][PythonLicence], [Bash manual][Bash], [ShellCheck][ShellCheck] — U; [BigCodeBench][BigCodeBench] — ICLR 2025; [EnvBench][EnvBench] — 2025-03-18.
- **Schemas:** [Ajv][Ajv], [JSON Schema specification][JSONSchema] — U; [JSONSchemaBench][SchemaBench] — 2025-02-27.

[Downloads]: https://api.npmjs.org/downloads/point/2026-11-01:2026-11-07/xstate,mermaid,bpmn-moddle,ajv,scion,asl-validator
[WorkflowDownloads]: https://api.npmjs.org/downloads/point/2026-11-01:2026-11-07/@openworkflowspec%2Fsdk
[Questions]: https://api.stackexchange.com/2.3/tags/xstate;scxml;amazon-states-language;aws-step-functions;bpmn;mermaid;graphviz;dot;plantuml;github-actions;makefile;serverless-workflow;crewai;python;bash;jsonschema/info?site=stackoverflow&pagesize=100&filter=default
[Training]: https://arxiv.org/html/2402.19173
[Structured]: https://platform.openai.com/docs/guides/structured-outputs
[Grammar]: https://platform.openai.com/docs/guides/function-calling
[Representations]: https://arxiv.org/html/2507.11356
[XState]: https://github.com/statelyai/xstate
[Serialization]: https://stately.ai/docs/xstate/v6/serialization
[SCXML]: https://www.w3.org/TR/scxml/
[Commons]: https://github.com/apache/commons-scxml
[ASL]: https://states-language.net/spec.html
[ASLValidator]: https://github.com/ChristopheBougere/asl-validator
[BPMN]: https://www.omg.org/spec/BPMN/2.0.2/PDF
[Moddle]: https://github.com/bpmn-io/bpmn-moddle
[MermaidState]: https://mermaid.js.org/syntax/stateDiagram.html
[MermaidFlow]: https://mermaid.js.org/syntax/flowchart.html
[Mermaid]: https://github.com/mermaid-js/mermaid
[MermaidAPI]: https://mermaid.ai/open-source/config/usage.html
[FlowchartStudy]: https://arxiv.org/html/2512.02170v2
[DOT]: https://graphviz.org/doc/info/lang.html
[GraphvizLicence]: https://gitlab.com/graphviz/graphviz/-/raw/main/COPYING
[PlantState]: https://plantuml.com/state-diagram
[PlantLicences]: https://plantuml.com/download
[OmniDiagram]: https://arxiv.org/html/2604.05514v1
[Actions]: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
[Actionlint]: https://github.com/rhysd/actionlint
[CIStudy]: https://arxiv.org/html/2507.17165
[Make]: https://www.gnu.org/software/make/
[MakeManual]: https://www.gnu.org/software/make/manual/make.html
[WorkflowDSL]: https://github.com/open-workflow-specification/specification/blob/main/dsl-reference.md
[WorkflowSDK]: https://github.com/open-workflow-specification/sdk-typescript
[WorkflowRelease]: https://open-workflow-specification.org/blog/releases/release-100/
[Crew]: https://docs.crewai.com/en/concepts/crews
[PyYAML]: https://github.com/yaml/pyyaml
[AgentYAML]: https://learn.microsoft.com/en-us/agent-framework/workflows/declarative
[PythonAST]: https://docs.python.org/3/library/ast.html
[PythonTasks]: https://docs.python.org/3/library/asyncio-task.html
[PythonLicence]: https://docs.python.org/3/license.html
[Bash]: https://www.gnu.org/software/bash/manual/bash.html
[ShellCheck]: https://github.com/koalaman/shellcheck
[BigCodeBench]: https://proceedings.iclr.cc/paper_files/paper/2025/file/a6a90bcc2aa470c3871b2d39a67d26e8-Paper-Conference.pdf
[EnvBench]: https://arxiv.org/html/2503.14443v1
[Ajv]: https://github.com/ajv-validator/ajv
[JSONSchema]: https://json-schema.org/specification
[SchemaBench]: https://arxiv.org/html/2501.10868v3