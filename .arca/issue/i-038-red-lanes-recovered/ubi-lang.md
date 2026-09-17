# Ubiquitous language

## Terms

| Term | Meaning |
| :--- | :--- |
| `RLR` | This issue's stable requirement-ID prefix: **Red-lane recovery** - the rule and the work that turn a red, unexpired sweep verdict back into `pass` or an honest expiry marker. |
| Triage class | The cause a red verdict is sorted into before any act: retired contract, fixture drift, or live regression. The class, not the failure message, decides what may be done to the lane. |
| Retired contract | A red verdict whose lane asserts an interface, path, record key, or wording that a later, authorized landing replaced on purpose (for example the `.arca/` engine root, the Run Record key `phase`, the public `doctor::render_json`). The behaviour the lane was cut to prove still exists under the new spelling. |
| Fixture drift | A red verdict whose lane's own scaffold no longer builds what the Engine accepts - an obsolete layout, a citation taken before the Engine mints it - while the behaviour under test and the Engine's answer to it are unchanged. |
| Live regression | A red verdict whose lane is right: the Engine violates a requirement that is still frozen in the goal. Fixed in the Engine under that requirement's own gap record, never by editing the lane. |
| Port | Rewriting a lane's fixture and expectations to today's spelling while keeping its lane id and the requirement it was cut for. A port that drops a lane or weakens its oracle is not a port. |
| Roster | The declared list of landed hidden crates in `.ratmac/lanes.toml` that the sweep must find in the folder. A crate in the folder the roster does not name is a stray; a crate the roster names and the folder lacks is missing. |
