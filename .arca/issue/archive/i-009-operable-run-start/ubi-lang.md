# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| Run-start sign-off | Explicit human authorization for the Main-Agent to invoke argument-free `rtm start` for the current target project; conversational instruction suffices, and no token, file, or Engine state encodes it. |
| Project-local bootstrap | One documented command run from the project root that locates or builds the Stable Engine binary, verifies its recorded identity, and reports the resolved path without global installation or PATH mutation. |
| Doctor report | Read-only diagnosis output that names the resolved Engine identity, distinguishes the human-authored Runbook from Scheduler-owned runtime state, and states the next legitimate action. |
| Behavioral evidence | Proof derived from recorded attempted commands or tool calls in role scenarios — what a caller actually invoked or refrained from invoking. |
| Guidance-consistency evidence | Proof that active guidance texts agree with each other; it can never substitute for behavioral evidence on invocation claims. |
