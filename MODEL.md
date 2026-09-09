# VirtualLife — model and examples

**Model status:** Option A is the approved first autonomous experiment. Four default property bundles drive random individual choices; the scripted fixture remains regression coverage. Neither is evidence of biological realism or emergence. The browser is the selected interface. See [README](README.md) for the project overview and run instructions, and [the development reference](docs/development.md#verification) for checks; this document records rules, their rationale and open model choices.

## Confirmed starting constraints

An agent represents an individual with identity, properties, and state. Resources are agents with different properties. No species hierarchy, programmed predation/cooperation, or separate resource layer. Inactivity alone does not imply removal. Biological labels describe interpretations afterward.

The world is a discrete rectangular 2D grid with wraparound edges and eight immediate neighboring squares. Each square holds at most one agent. Agents are treated at roughly the same spatial scale; mixed microscopic/macroscopic sizes, overlap, multiple occupancy, and containment are out of scope initially. Keep those future possibilities open without implementing layers or spatial frameworks now.

Updates are synchronous: decisions consult the unchanged starting state; each starting agent acts at most once; a new agent first acts on the following tick. Creation, removal, and property changes belong early. An agent already meeting a removal condition must not act before removal; Option A random removal is the individual's sole chosen action that tick.

The engine must run without a viewer. With the same initial state, rules, and number of ticks, observation alone must not change the resulting world. Randomness has explicit reproducibility settings; do not promise cross-version identity from a seed alone. User interventions are different from observation and must be applied between ticks.

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

The live tool receives immutable, tick-stamped samples without blocking simulation on rendering. Show the grid, inspection of an agent's ID/properties/position (fixture value in demo mode), counts, accepted-event totals, and bounded population-versus-tick plots. Mark sampling gaps; missing samples are not missing simulation steps. Paused and completed runs must make the actual final state available. Headless and observed runs use the same transitions.

Pause/resume and single-step act between ticks. Start paused; single-step advances exactly once only while paused, and cannot succeed beyond the finite end. Queued commands are distinguished from worker-applied actions by receipts. Completed state remains readable; stopped/failed or disconnected states are visible. A lost non-idempotent control response is not automatically retried.

The native process owns the experiment. Refreshing, closing, hiding, or disconnecting the browser does not pause, reset, or terminate it. A new page reconnects to its current/final snapshot; page-local plot history starts there without fabricating earlier samples. Ctrl+C explicitly shuts down the server and cleanly stops/joins the worker. Restarting the process starts a new experiment. Speed limiting belongs in the runner, not the transition. State editing and modest exports follow early but are outside Tasks 01–02. Record future edits with their applied tick. Lossy live observation is not a complete recording or replay system.

**9 September 2026 — restart recovery (Pierre's decision):** when a retained page detects a different server run, it clears the previous run's page-local history, selection and pending control state, displays a new-experiment notice, and follows the new run's actual state. It does not resume, step or retry commands automatically. Same-run reconnect retains history. Delayed old-run control requests are rejected and old replies cannot change the new page state. Run identity belongs to the HTTP adapter, not simulation state or model randomness.

## Decisions, limits, and next model checkpoint

**6 September 2026 — geometry and scheduling (Pierre-confirmed):** grid, eight neighbors, single occupancy, comparable scales, and synchronous updates keep the starting algorithm understandable. This does not exclude future changes; occupancy changes may require redesign.

**6 September 2026 — conflict policy (Task 01 working choice):** reject all competing claims and use start-of-tick emptiness. This avoids traversal-order winners and complicated move chains, but can produce congestion. The minimum dimension and integer value are also fixture choices, not universal requirements. Alternatives such as seeded random winners need a model checkpoint rather than an unnoticed implementation change.

**6 September 2026 — browser interface (Pierre's decision):** replace the native window with a small local browser observer/control surface, retaining the native Rust engine and headless execution. Observation is bounded and independent of rendering. Browser lifetime no longer owns worker lifetime. This changes application lifecycle, not transition rules or the scripted table.

**9 September 2026 — Option A (Pierre's decision):** implement the four-property-group autonomous experiment below. This supersedes issue #8's earlier identical-properties proposal and awaiting-model-choice wording. Neighbour-dependent behaviour is the immediate follow-up, but its rules require Pierre's next decision.

## Option A: shared properties, autonomous choices

A species is the exact tuple of four nonnegative integer weights: **wait, move, copy, remove**. At least one weight must be positive. Individuals with identical tuples belong to one group, regardless of labels, IDs, positions or ancestry. `(1,1,0,0)` and `(2,2,0,0)` are distinct bundles despite having equal action probabilities; grouping compares actual properties, not normalized probabilities. Labels A–D, colours and symbols are presentation only. The implementation supports one to eight distinct bundles; four is the default experiment, not four action implementations.

Each starting individual uses the same procedure. Imagine numbered tickets in four adjacent piles: a `(4,5,1,1)` individual has 4 wait tickets, 5 move tickets, 1 copy ticket and 1 remove ticket. Draw one of the 11 tickets uniformly. A zero-weight action has no tickets and cannot be chosen. A move/copy draws one of **all eight neighbours**, uniformly, without checking occupancy first. Wait/removal draws no destination. The selected action becomes an ordinary proposal; the existing engine resolves the full batch against the unchanged starting grid.

For example, a move from group A and a copy from group B both targeting a start-empty square both fail. They do not choose another target. If a third individual removes itself, its old square still cannot receive a move/copy in that tick. A successful copy retains the parent's complete weight tuple, receives a fresh ID in creator row-major order, and first draws an action next tick. Movement preserves ID and properties. Weights remain fixed throughout life; the fixture-only integer value is unused by autonomous choices and is not a species property.

Removal is a random selected action, **not ageing or an inactivity penalty**. No mutation, energy, resources, neighbour-property-dependent choices, global population control, balancing, automatic reseeding or guaranteed coexistence are implemented. Any or all groups can become extinct. Every configured group remains in the legend and population observations at zero. The finite tick limit ends a run even when it is already empty; extinction does not trigger a special transition.

### Illustrative initial settings

The default autonomous world is 32 × 24, occupancy 0.3, four equal initial proportions, seed 1 and 500 ticks. Default bundles are A `(4,5,1,1)`, B `(4,3,2,1)`, C `(6,2,1,1)` and D `(2,6,2,1)`. These are adjustable experimental settings, **not scientifically calibrated values**. They illustrate different tendencies; no hidden mechanism tunes a winner or maintains coexistence.

Occupancy specifies an **exact total**, rounded down: `floor(width × height × occupancy)`. For 768 squares and 0.3, place 230 agents. Initial proportions are nonnegative integer ratios. First combine proportions of identical bundles in first-appearance order. Allocate each group's exact quota by rounding down, then give the leftover individuals to the largest fractional remainders, breaking ties by that canonical group order. Equal ratios therefore yield 58, 58, 57, 57 agents. Zero ratios produce zero initial agents and remain visible; the total ratio must be positive even on an empty grid. The actual canonical groups, ratios and initial counts are recorded in browser and headless output.

Shuffle all row-major square indices with descending Fisher–Yates; assign successive shuffled slots to the groups' exact quotas, then assign starting IDs 1, 2, … in occupied row-major order. This produces random placement without overlaps, including exact empty/full grids. Dimensions are at least 3 in each direction and at most 262,144 squares for bounded local allocations. Occupancy accepts up to six decimal places from 0 through 1. Each weight/ratio is a `u32`; totals and quota arithmetic use wider integers.

### Reproducibility contract

The generator is **SplitMix64**, using [Vigna's public-domain 2015 reference](https://prng.di.unimi.it/splitmix64.c), with the explicit `VirtualLife sampling v1` draw protocol in `src/experiment.rs`. Seed is the initial unsigned 64-bit state. Each raw draw adds `0x9e3779b97f4a7c15` with wrapping arithmetic, then mixes with shifts 30/27/31 and multipliers `0xbf58476d1ce4e5b9` and `0x94d049bb133111eb`. Reference output vectors are tested. This generator is for reproducible experiments, not security.

To draw uniformly below N, discard raw values below `(-N modulo 2^64) modulo N`, then return the accepted value modulo N. Rejected raw values advance the generator. This avoids remainder bias. Initialization draws once per Fisher–Yates iteration (plus any rejected raw values), from the last square through index 1, even for an empty population. Quota calculation and ID allocation consume no randomness. The same generator then continues into ticks. Iterate occupied starting squares in row-major order: draw one weighted action per individual, then draw a destination only for move/copy. Neighbours have the stable order northwest, north, northeast, west, east, southwest, south, southeast, with wraparound. Failed destinations never retry and still consume their original draws. Newborns enter only the next tick's iteration.

Record the seed, effective configuration, generator/protocol name and crate version with results; keep the source commit/Cargo.lock for exact reproduction. The same configuration and code produces the same fixed-tick world and accepted-event totals with observation disabled, enabled, saturated, disconnected or sampled differently. Rendering, pacing and the HTTP adapter's OS-random run identity consume no simulation draws. A seed alone does not promise identical outcomes across arbitrary code or dependency changes. Live samples are bounded and incomplete, not a saved trajectory.

### Immediate model checkpoint: local neighbours

Action selection is a separate function receiving the read-only starting `World`; it can already read each individual's position and the eight neighbours' actual properties through `neighbors` and `agent_at`. Transition resolution and presentation do not need duplicate implementations. No neighbourhood framework or interaction matrix is introduced.

A small next proposal for Pierre is **crowding-sensitive copy selection**: after an individual draws Copy, let it wait instead when all eight starting neighbours are occupied. This makes local information explicit without changing successful outcomes, but changes destination draw consumption and therefore later random trajectories; it also adds little observable behaviour because the engine already rejects occupied targets. A more informative alternative is to scale copy tendency with the number of empty neighbours before drawing the action; that changes copy probabilities and population dynamics and needs an explicit formula and worked examples. Neither rule is selected or implemented. Choose the question and exact rule before that next increment.
