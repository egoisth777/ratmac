# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `ARF` | **Archived record freeze** - this issue's stable requirement-ID prefix. |
| Judgment-time revision | The goal revision that was frozen for the planning pass in which a gap record was written. It is a fact about the record, not about today. |
| Live record | A gap record in the active folder: it describes work now, so it must speak about today's frozen goal. |
| Archived record | A gap record whose requirement read `satisfied` and which took the authorized archive move. Its bytes are preserved; a re-judgment moves it back rather than editing it in place. |
