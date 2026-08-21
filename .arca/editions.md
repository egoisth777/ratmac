# Editions - the committed record of what each edition marks

`EDN-003` says an edition never moves and never disappears. Version control cannot
be made to refuse a moved tag, so this file is what makes a move visible: it
records, in the repository's own history, the commit each edition was cut at. The
edition audit compares every row here against the tag database, so a tag that was
moved or deleted becomes a reported difference instead of an invisible edit.

A row is written in the same landing that cuts the edition, and is never edited
afterwards. The commit column holds a full hash because that is the field's whole
purpose - it is the record a citation resolves against, not a name.

| Edition | Commit | What it marks |
| :--- | :--- | :--- |
| `edition-001` | `18bc304200cc0cfb20dbff42b5b966a79ec3526f` | The rest that closed the cycle-as-runbook sprint: the shipped Machine Class became this repository's own five-stage cycle, and the addressed report answers where a sprint stands. |
| `edition-002` | `8276bd0bd0a353ebe5a5b2489b771e45d8dbfc08` | The rest that closed the archived-record-freeze sprint: the record contract proven on aged fixtures, run-002's blocker retired, i-029 and i-030 archived. |
| `edition-003` | `929c5834fb270604f9d9e25ee43705bf6ff73451` | The rest that closed the engine-channel sprint: the engine pin carries channel and source-commit, stable resolves from this ledger, and the doctor reports provenance and off-pin live Runs. |
| `edition-004` | `46e7309b84ed976d3b9d96b0f4478bc0c6b489c2` | The rest that closed the operator-protocol sprint: the stable bootstrap builds the tagged commit from any healthy main (ELR-002), status and step teach each guard's reads and the one next act (AOP-001/002), the engine writes its own identity-stamped operator skill (AOP-003/004), and the qa checker reads a ticket's checks from its tags alone (CGD-001/002). |
