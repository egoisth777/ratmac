# Residual Record Template

```yaml
residual-id: "{{residual-id}}"
goal-requirement-ref: "{{goal-requirement-ref}}"
frozen-goal-bundle-revision: "{{frozen-goal-bundle-revision}}"
implementation-revision: "{{implementation-revision}}"
concrete-evidence-refs:
  - "{{evidence-ref-1}}"
classification-rationale: "{{classification-rationale}}"
status: "{{missing|partial|satisfied}}"
mutation-kill:
  - "{{mutation-and-the-named-test-that-kills-it}}"
required-test-refs:
  - "{{required-test-ref-1}}"
```

> **Note**: Missing evidence cannot yield satisfied. `mutation-kill` is the sole physical home for
> deliberate-damage evidence (schema.md, "Deliberate damage and discard safety"): each line is written
> only after the observed failure, from the safety commit, and the owning ticket only points here.
