# VirtualLife — model and examples

**Model status:** the implemented rules below describe the deterministic technical fixture, not an approved autonomous research model or evidence of emergence. The browser is the selected interface. See [README](README.md) for the project overview and run instructions, and [the development reference](docs/development.md#verification) for checks; this document records rules, their rationale and open model choices.

## Confirmed starting constraints

An agent represents an individual with identity, properties, and state. Resources are agents with different properties. No species hierarchy, programmed predation/cooperation, or separate resource layer. Inactivity alone does not imply removal. Biological labels describe interpretations afterward.

The world is a discrete rectangular 2D grid with wraparound edges and eight immediate neighboring squares. Each square holds at most one agent. Agents are treated at roughly the same spatial scale; mixed microscopic/macroscopic sizes, overlap, multiple occupancy, and containment are out of scope initially. Keep those future possibilities open without implementing layers or spatial frameworks now.

Updates are synchronous: decisions consult the unchanged starting state; each starting agent acts at most once; a new agent first acts on the following tick. Creation, removal, and property changes belong early. An agent already meeting a removal condition must not act before removal; the actual autonomous condition is not defined yet.

The engine must run without a viewer. With the same initial state, rules, and number of ticks, observation alone must not change the resulting world. Future randomness must have explicit reproducibility settings; do not promise cross-version identity from a seed alone. User interventions are different from observation and must be applied between ticks.

## Task 01 fixture: a trustworthy live grid

These rules specify a small, deterministic test of the machinery. They do not prescribe the first autonomous experiment. There is no random action selection, automatic expiry, creation cost, energy, or biological meaning attached to its value field.

### State and input

The grid has width and height of at least 3, avoiding duplicated neighbors on tiny wrapped grids. Validate dimensions, indexing, and occupancy. An agent has a stable numeric ID and one integer `value`; its containing square determines its location. IDs remain unchanged on movement and are unique and non-reusing within a run. Allocate accepted creations in a documented stable order independent of proposal input order. No generic property bag is needed.

The implementation uses `u64` IDs and `i64` values. Accepted creations receive increasing IDs in row-major order of their creators' **starting squares**: increasing y, then x. Allocation starts above the highest initial ID (at 1 for an empty grid), and rejected creations consume no IDs. The largest `u64` is reserved as an exhaustion sentinel; exhausted allocation, tick, or event counters return an error before committing a transition. Setting an unchanged value succeeds as a no-op and adds zero to the value-change total.

A short ordinary function supplies at most one proposal per starting agent: wait, move to a neighboring square, set its own value, create a copy in a neighboring square, or remove itself. Missing proposals mean wait. A copy receives a new ID and the creator's starting value; the creator remains unchanged. Removal is the actor's only action that tick.

Unknown actors, duplicate proposals for one actor, and out-of-range or non-neighbor targets are invalid input. Reject invalid input without partially changing the world, counters, tick, or ID allocation. Occupied targets and destination conflicts are ordinary rejected actions, not invalid input.

### One tick

```text
Read the starting grid without changing it.
Validate the proposals for the starting agents.
Resolve competing destination claims.
Build the next grid from the accepted outcomes.
Replace the current grid and update the tick and accepted-event totals.
```

Moves and creations can succeed only into squares empty at the start. If two or more proposals claim the same eligible square, all those claims fail, including mixed move/create claims. Rejected movers remain at their starting square; rejected creation creates nothing. A square vacated by movement or removal is available only next tick. No same-tick swaps or movement chains.

Changing proposal input order must not change the next world, accepted-event totals, or allocated IDs. New agents occur only in the next world and cannot propose an action in the tick that creates them. Count accepted events during the transition; displayed frame differences are not an event log.

### Worked example

Use a 5 × 5 grid with zero-based `(x, y)` coordinates. Initially A (ID 1, value 10) is at `(0,2)`, B (ID 2, value 20) at `(2,2)`, and C (ID 3, value 30) at `(4,2)`. Unmentioned agents wait.

| Transition | Proposals | Expected result |
|---|---|---|
| 0 → 1 | A and B move to `(1,2)`; C sets value 31. | Both moves fail; C changes; count 3. |
| 1 → 2 | A removes itself; B creates at `(3,2)`; C moves across the edge to `(0,2)`. | A disappears; D appears with ID 4 and value 20; C stays because A's square was occupied at the start; count 3. D has not acted. |
| 2 → 3 | B sets value 21; C moves to `(0,2)`; D moves to `(3,3)`. | All succeed; D still has value 20; count 3. |
| 3 → 4 | B creates at `(1,2)`. | E appears with ID 5 and value 21; count 4. |
| 4 → 5 | C and D remove themselves. | B and E remain; count 2. |

Final state: B (ID 2, value 21) at `(2,2)` and E (ID 5, value 21) at `(1,2)`. Accepted totals: two moves, two creations, three removals, two value changes. The finite demonstration ends clearly; do not present it as autonomous persistence.

The viewer ends at tick 5. A headless request for more than five ticks uses missing proposals (all wait) afterward: only the tick advances, with the final agents, values, IDs, and accepted totals unchanged. A zero-tick request reports the initial state.

## Observation and controls

The live tool receives immutable, tick-stamped samples without blocking simulation on rendering. Show the grid, inspection of an agent's ID/value, counts, accepted-event totals, and a bounded count-versus-tick plot. Mark sampling gaps; missing samples are not missing simulation steps. Paused and completed runs must make the actual final state available. Headless and observed runs use the same transitions.

Pause/resume and single-step act between ticks. Start paused; single-step advances exactly once only while paused, and cannot succeed beyond the finite end. Queued commands are distinguished from worker-applied actions by receipts. Completed state remains readable; stopped/failed or disconnected states are visible. A lost non-idempotent control response is not automatically retried.

The native process owns the experiment. Refreshing, closing, hiding, or disconnecting the browser does not pause, reset, or terminate it. A new page reconnects to its current/final snapshot; page-local plot history starts there without fabricating earlier samples. Ctrl+C explicitly shuts down the server and cleanly stops/joins the worker. Restarting the process starts a new fixture. Speed limiting belongs in the runner, not the transition. State editing and modest exports follow early but are outside Tasks 01–02. Record future edits with their applied tick. Lossy live observation is not a complete recording or replay system.

**9 September 2026 — restart recovery (Pierre's decision):** when a retained page detects a different server run, it clears the previous run's page-local history, selection and pending control state, displays a new-experiment notice, and follows the new run's actual state. It does not resume, step or retry commands automatically. Same-run reconnect retains history. Delayed old-run control requests are rejected and old replies cannot change the new page state. Run identity belongs to the HTTP adapter, not simulation state or model randomness.

## Decisions, limits, and next model checkpoint

**6 September 2026 — geometry and scheduling (Pierre-confirmed):** grid, eight neighbors, single occupancy, comparable scales, and synchronous updates keep the starting algorithm understandable. This does not exclude future changes; occupancy changes may require redesign.

**6 September 2026 — conflict policy (Task 01 working choice):** reject all competing claims and use start-of-tick emptiness. This avoids traversal-order winners and complicated move chains, but can produce congestion. The minimum dimension and integer value are also fixture choices, not universal requirements. Alternatives such as seeded random winners need a model checkpoint rather than an unnoticed implementation change.

**6 September 2026 — browser interface (Pierre's decision):** replace the native window with a small local browser observer/control surface, retaining the native Rust engine and headless execution. Observation is bounded and independent of rendering. Browser lifetime no longer owns worker lifetime. This changes application lifecycle, not transition rules or the scripted table.

**Still open:** the first autonomous properties and decision procedure, local interaction, creation/removal conditions, and any conservation or transfer rules. Do not silently restore the old lifetime distribution, introduce a resource economy, or rename hardcoded biological behavior to make it seem neutral. Scripted tests establish implementation behavior, not emergence. Choose the first autonomous rule set with Pierre after reviewing the live demonstrator.

## Preparing the first autonomous experiment

Choose one small experiment with Pierre before implementing new rules. These are discussion prompts, not selected behaviour or a new specification template:

- What question should the experiment make observable, and what initial arrangement will help investigate it?
- What named properties does an individual carry, what local information can it read, and how does it choose its action?
- When can creation or removal happen? Are any quantities transferred or conserved, and how are incompatible proposals resolved? Do not assume the fixture's action vocabulary already supports joint interactions.
- What should a few worked ticks do, what invariants must always hold, and what behaviour remains genuinely open? If randomness is chosen, specify reproducibility inputs as well.

Record the agreed rules and rationale here, then implement one bounded headless-and-observable increment with matching tests. Keep the scripted fixture as regression coverage. The walkthrough should let Pierre follow one individual's decision, predict a small example and find its Rust implementation. Understanding the algorithm and investigating the question are useful outcomes; spectacular emergence is not a release gate. No new autonomous rule is selected by this preparation.
