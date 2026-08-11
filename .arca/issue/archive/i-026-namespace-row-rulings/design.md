# Issue design

## Proposed mechanics

No mechanism is proposed. Both asks are rulings about which of two rules already in force
gives way; a mechanism authored before the ruling would prejudge it.

What the ruling has to touch is worth stating, so the cost of each branch is visible:

- **Held fact, moved into Engine-owned state.** The hold writes only Engine-owned files, a
  declared root may read the ticket, the working rules stop saying the hold marks the ticket,
  and the blocked-route requirement is reworded. Any human-readable mark on the ticket
  becomes a contributor action or a receipt, never an Engine write.
- **Held fact, named exception.** The goal row keeps the Engine's one write under the working
  folder, names it, and names its owner. Nothing else moves, and the two-writer file stays.
- **No-literal check, named exception.** The goal row admits one declared spelling, names its
  owner, and states how the check pins it; the check keeps failing on a second literal or a
  rename.
- **No-literal check, detection moved to data.** Residue detection reads a declared value
  instead of a source literal, which means a project that still carries the retired folder
  must supply that value from somewhere it can still be read.

This file is incoming evidence. Integrated mechanics remain authoritative only in the accepted forward authority.
