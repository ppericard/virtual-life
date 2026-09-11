# Disposable material prototype

Question: can common agent interactions conserve finite material through acquisition, Repair, Copy and wear, and what does sharing a cell change?

Run from the repository root:

```sh
npm run prototype:material
```

Open http://127.0.0.1:7881/. Ctrl+C stops this preview. The ordinary Rust simulation on 7880 is independent. No new dependencies, persistence, external services or production endpoints. Browser refresh resets this laboratory.

The logic is a small pure module in `model.js`; the browser exposes its complete material state after each manual step. This is a logic prototype with a visual interface because Pierre asked to inspect the model visually. Its JavaScript is disposable; the Rust engine is unchanged.

## Hypotheses, not approved rules

- Each agent carries structure, reserve and loose material. For these examples structure doubles as integrity; all three count toward conserved material.
- All agents have the same graph and material fields. The resource-like initial presets have every outgoing arrow leading to Wait. The other preset exposes several arrows for manual exploration; the lab does not sample or adjust probabilities. Take is a candidate fifth action, omitted in the alternative acquisition setting.
- Zero structure does not change the graph, disable all actions or convert reserve. Wait follows the graph; Repair can restore structure when reserve is available. Move and Copy require structure after attempt wear. A fully absorbed zero-material holder may be removed; the transferred material remains in other agents.
- Loose material is a property held by an agent. There is no debris field, world recycling service or automatic refill. A manual separation probe can form another agent from held loose material.
- Shared cells and one-agent cells both have an illustrative 20-material capacity. Shared occupancy permits splitting in place without increasing occupied material. Moving and cross-cell acquisition check destination capacity.
- Compare explicit Take (a candidate additional unit action) with Repair obtaining needed material from a local target. Neither acquisition contract is approved. Take uses only another agent's loose material and does not inspect a species label.
- Repair converts at most 4 reserve to structure; Copy funds 3 structure and 2 reserve in a child. Move/Copy attempts convert 1/2 structure into retained loose material. These numbers are illustrative.
- Children and separated fragments inherit the source graph in this example. No hardcoded resource/remains type or functioning/inert flag is introduced.
- Wait leaves material unchanged here. Wear is a separate manual probe; there is no pacing, population lifetime inference, autonomous FSM sampling or random choice. The lab has no simultaneous competing claims or mutation.

## Review boundary

Use the worked cases and the state accounting to choose acquisition and spatial rules before production implementation. In particular, inspect whether loose material held by any other agent should be externally accessible, and whether separation should be possible into the same cell. Do not mistake deterministic examples for emergent survival.

Pierre rejected the first sketch's functioning/inert categories: passive behaviour should come from a Wait-only FSM using the common agent model. This clarification is recorded in MODEL. Acquisition, capacity, access and exact transfer rules remain open. Once the question is answered, remove this lab or replace it with the validated production behaviour; do not keep an alternative simulation indefinitely.
