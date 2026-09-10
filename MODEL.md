# VirtualLife — model and examples

**Model status:** the approved autonomous default is wear and repair with Copy reduced by local crowding. Four inherited property bundles and starting neighbour occupancy drive individual choices; survival follows integrity and operating history. The earlier random-removal Option A and scripted fixture remain available for comparison and regression coverage. None is evidence of biological realism or emergence. See [README](README.md) for the overview and [the development reference](docs/development.md#verification) for usage and checks.

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

**10 September 2026 — seeded restart (Pierre's decision):** an explicit browser restart replaces an autonomous experiment using an editable fixed seed (default 1) or an OS-random seed disclosed for reproduction. It retains the world, property, survival and run settings, starts paused at tick 0 (completed for a zero-tick limit), and clears prior run evidence and page histories. Completed runs can restart. Every replacement gets a new HTTP identity, including repeats with the same seed. Random seed selection consumes no simulation draws. The scripted demo and headless lifecycle are unchanged; no run restarts automatically.

## Decisions, limits, and next model checkpoint

**6 September 2026 — geometry and scheduling (Pierre-confirmed):** grid, eight neighbors, single occupancy, comparable scales, and synchronous updates keep the starting algorithm understandable. This does not exclude future changes; occupancy changes may require redesign.

**6 September 2026 — conflict policy (Task 01 working choice):** reject all competing claims and use start-of-tick emptiness. This avoids traversal-order winners and complicated move chains, but can produce congestion. The minimum dimension and integer value are also fixture choices, not universal requirements. Alternatives such as seeded random winners need a model checkpoint rather than an unnoticed implementation change.

**6 September 2026 — browser interface (Pierre's decision):** replace the native window with a small local browser observer/control surface, retaining the native Rust engine and headless execution. Observation is bounded and independent of rendering. Browser lifetime no longer owns worker lifetime. This changes application lifecycle, not transition rules or the scripted table.

**9 September 2026 — Option A (Pierre's decision):** implement the four-property-group autonomous experiment below. This supersedes issue #8's earlier identical-properties proposal and awaiting-model-choice wording. The subsequently approved wear-repair crowding rule below supplies the first neighbour-dependent choice.

## Wear and repair: survival through operating history

**10 September 2026 — Pierre's decision:** replace a chosen random death with a small integrity/maintenance experiment, before adding neighbourhood or energy rules. Each individual has one mutable nonnegative integer `integrity`. All individuals share adjustable maximum/initial/newborn integrity, upkeep, extra move/copy wear and gross repair. Their species is the exact **Wait/Move/Copy/Repair weight tuple** within this shared rule; current integrity, ID, ancestry and position do not define a species. Identical bundles coalesce as before. Properties do not mutate during life.

The default is `--survival wear-repair`, with maximum 10, base upkeep 1, extra crowding upkeep 1 at five or more occupied neighbours, extra move wear 1, extra copy wear 2 and gross repair 4. The fourth choice is Repair, replacing Remove. Default bundles are A `(2,4,1,3)`, B `(2,2,2,4)`, C `(4,1,1,4)` and D `(1,5,2,2)`. The 32 × 24 grid, occupancy 0.3, equal proportions, seed 1 and 500-tick limit are unchanged. These are adjustable illustrative settings, not values tuned for coexistence or biological calibration.

For every tick, validate all state, rules and proposals without mutation. Count occupied squares among the eight wrapped neighbours in the unchanged starting world. With count `N`, effective upkeep is `base upkeep + (N >= crowding threshold ? crowding upkeep : 0)`. An individual with `integrity <= effective upkeep` fails maintenance and is removed before choosing or performing any action, with no action or destination draw. It cannot repair itself in that tick. Otherwise it pays effective upkeep and chooses one weighted action using the crowding policy below:

- **Wait:** no further change; upkeep has still been paid.
- **Move or Copy:** draw uniformly among all eight neighbours, before checking affordability or occupancy. If integrity after upkeep is less than or equal to the extra wear, the individual fails before the spatial action. It makes no claim, creates no child and allocates no ID. Otherwise charge the wear, even when occupancy or competing claims subsequently reject the destination. There are no retries.
- **Repair:** use the entire action opportunity to add gross repair after upkeep, capped at the maximum. There is no material or energy cost yet. Count one repair when integrity actually increases relative to its value after upkeep; a capped repair with a positive restoration still counts if it restores upkeep loss.

Only affordable move/copy attempts claim destinations. All claims still use the unchanged starting grid: competing affordable claims all fail, including mixed move/copy claims. A square vacated by failure remains start-occupied and unavailable until the next tick. Normal wrapping, single occupancy, stable IDs and row-major creation-ID allocation remain unchanged. Integrity updates, spatial outcomes, IDs, events and failure evidence commit atomically; invalid input or exhausted counters leave both buffers and all state unchanged.

A successful copy inherits the parent's weight tuple, receives a fresh ID and starts at shared maximum integrity. The parent pays upkeep and copy wear and retains its ID. The newborn first acts and first pays upkeep next tick. This is a fresh functional condition, **not an energy-conservation model**; there is no energy or material account to duplicate. Initial individuals also start at maximum integrity.

### Crowding increases upkeep

**10 September 2026 — Pierre's decision:** shared base upkeep is increased by a fixed surcharge when starting occupied neighbours meet the threshold. The default threshold is 5 and the surcharge is 1. Every group and every action, including Wait and Repair, pays by the same rule. Neighbours that will die or move during the tick still count. Movement pays the origin's cost; escaping crowding can reduce upkeep only on the following tick.

`--crowding-threshold` accepts integers 0–8: zero applies the surcharge everywhere, and eight requires all neighbours occupied. `--crowding-upkeep` accepts nonnegative `u32` values; zero disables the surcharge. Both are rejected in random/demo modes, including zero-valued overrides. Base plus surcharge uses `u64` without wrapping or clamping: two `u32::MAX` costs total 8,589,934,590, which no allowed integrity can survive. This changes maintenance cost, without adding an energy account, neighbour properties or another action choice.

### Copy tendency responds to local space

**10 September 2026 — Pierre's decision:** multiply Copy's base probability by the fraction of the eight starting neighbours that are empty, transferring the removed probability to Wait. This applies only to autonomous wear-repair. For inherited weights `(W,M,C,R)` and `E` empty neighbours, choose from the exact integer tickets:

```text
Wait: 8W + (8-E)C    Move: 8M    Copy: EC    Repair: 8R
Total: 8(W+M+C+R)
```

Multiplying all tickets by eight avoids rounding fractional Copy shares. With `(2,4,1,3)`, all eight empty gives `(16,32,8,24)` and the original probabilities; four empty gives `(20,32,4,24)`, halving Copy from 1/10 to 1/20; none empty gives `(24,32,0,24)`. With odd Copy weight 3 in `(1,2,3,4)`, one empty gives `(29,16,3,32)` and seven empty gives `(11,16,21,32)`. Zero Copy stays zero. Individual weights, inheritance and species grouping are unchanged; these temporary tickets are not new properties.

Read occupancy from the unchanged starting world, including wrapped neighbours and individuals that will fail upkeep this tick. Do not count their squares as empty in advance. If the chooser itself fails upkeep it makes no draw. Move and Repair probabilities remain unchanged; selected Move/Copy still draw uniformly among **all eight neighbours**, before affordability, with no filtering or retry. Crowding changes Copy intent frequency, not the existing rules for whether an attempt succeeds. Full-neighbourhood Copy-only individuals choose Wait and pay only upkeep that tick.

### Worked conditions and boundaries

The first table uses fewer than five occupied neighbours throughout, so effective upkeep is 1:

| Starting integrity and choice | Default result |
| --- | --- |
| 10, Move to an eligible unclaimed neighbour | 8: pay 1 upkeep and 1 movement wear |
| 8, another Move | 6 |
| 6, Repair | 9: pay 1, then restore 4 |
| 9, Repair | 10: restoration is capped |
| 10, Copy rejected by occupancy or another claim | Parent 7; no child and no new ID |
| 10, successful Copy | Parent 7; child 10, inactive until the following tick |
| 2, Move | Upkeep leaves 1; extra wear 1 would leave zero, so remove before claiming |
| 3, Copy | Upkeep leaves 2; extra wear 2 would leave zero, so remove before claiming |
| 1, Repair | Upkeep failure; no repair occurs |

| Starting integrity and choice | 4 occupied neighbours | 5 or 8 occupied neighbours |
| --- | --- | --- |
| 6, Wait | 5 | 4 |
| 6, Repair | 9: pay 1, restore 4 | 8: pay 2, restore 4 |
| 2, Repair | 5 | Upkeep failure before repair or any random draw |
| 6, Move attempt | 4 | 3; with 8 occupied neighbours the attempt is rejected and the individual stays in place |

Maximum must be positive. Costs/restoration may be zero and may exceed the maximum; arithmetic uses wider integers or comparisons before subtraction. There is no survivor at integrity zero. There is no lifespan draw, age limit, random death choice, inactivity penalty, hidden balancing or reseeding. Sufficient repair can sustain an individual indefinitely; zero upkeep/costs can also permit persistence. Extinction and continued survival are both valid, and the finite run limit remains independent of either outcome.

### Failure evidence and reproducibility

Every failure has an engine-recorded cause: **upkeep**, **move wear** or **copy wear**. Records include ID, resulting tick, starting position, starting integrity, starting occupied-neighbour count, base upkeep, applied crowding surcharge, effective upkeep and selected extra wear (zero when upkeep failed first). Each run retains the latest 128 records in tick then starting-square order, counts discarded older records, and retains cumulative repairs/failures. This evidence is recorded on every simulation tick independently of viewers or sample cadence; it is bounded causal evidence, not a complete event journal. Live inspection shows occupied neighbours and potential next-tick upkeep for the **displayed neighbourhood**, not the last charged cost. Absent-agent inspection uses its recorded starting conditions even if the square is now empty or occupied by someone else, or states that its record has been discarded. Plot gaps do not fabricate events. Reconnection retains the same run's server evidence; a new experiment clears it.

Initialization and bounded random draws use the SplitMix64 procedure below. The **wear-repair crowding v3** policy scans occupied starting squares in row-major order, skips all random draws for individuals unable to survive effective upkeep, then makes one bounded action draw below `8(W+M+C+R)` for every survivor. Tickets and their total use `u64`; with four `u32` base weights the total is at most `32 × u32::MAX`, safely below the limit. Do not simplify the common factor even when all eight neighbours are empty or Copy is zero: the bound is part of the reproducible protocol. Move/Copy then draws below 8, including unaffordable attempts. Wait/Repair draws no destination. Bounded-draw rejection still consumes raw generator values as documented; counting occupancy, adjusting tickets, integrity costs and repair add no random draws.

V3 preserves v2's Copy-to-Wait tickets and draw bounds; the surcharge can change survival, affordability and later draws. Setting `--crowding-upkeep 0` reproduces **wear-repair crowding v2** simulation trajectories with otherwise matching settings and seed, while output retains v3's expanded evidence format. V2 changed the action bound and crowding choices relative to **wear-repair v1**, even where probabilities coincide; a v1 wear result requires its earlier source version. Initialization is unchanged. The random-removal comparison remains **random v1**, with its original bounds and draw order. Record the action protocol as well as settings, seed, source commit and Cargo.lock when reproducing results.

## Option A comparison: random removal

Select `--survival random` to run the earlier approved Option A, without integrity or repair. Its original default bundles and random draw/iteration order are preserved; the same baseline configuration still produces the earlier fixed-tick results. Maintenance overrides are rejected for this mode. The scripted `--mode demo` remains independent of both autonomous policies.

A species is the exact tuple of four nonnegative integer weights: **wait, move, copy, remove**. At least one weight must be positive. Individuals with identical tuples belong to one group, regardless of labels, IDs, positions or ancestry. `(1,1,0,0)` and `(2,2,0,0)` are distinct bundles despite having equal action probabilities; grouping compares actual properties, not normalized probabilities. Labels A–D, colours and symbols are presentation only. The implementation supports one to eight distinct bundles; four is the default experiment, not four action implementations.

Each starting individual uses the same procedure. Imagine numbered tickets in four adjacent piles: a `(4,5,1,1)` individual has 4 wait tickets, 5 move tickets, 1 copy ticket and 1 remove ticket. Draw one of the 11 tickets uniformly. A zero-weight action has no tickets and cannot be chosen. A move/copy draws one of **all eight neighbours**, uniformly, without checking occupancy first. Wait/removal draws no destination. The selected action becomes an ordinary proposal; the existing engine resolves the full batch against the unchanged starting grid.

For example, a move from group A and a copy from group B both targeting a start-empty square both fail. They do not choose another target. If a third individual removes itself, its old square still cannot receive a move/copy in that tick. A successful copy retains the parent's complete weight tuple, receives a fresh ID in creator row-major order, and first draws an action next tick. Movement preserves ID and properties. Weights remain fixed throughout life; the fixture-only integer value is unused by autonomous choices and is not a species property.

Removal is a random selected action, **not ageing or an inactivity penalty**. No mutation, energy, resources, neighbour-property-dependent choices, global population control, balancing, automatic reseeding or guaranteed coexistence are implemented. Any or all groups can become extinct. Every configured group remains in the legend and population observations at zero. The finite tick limit ends a run even when it is already empty; extinction does not trigger a special transition.

### Illustrative initial settings

The random-removal comparison defaults to 32 × 24, occupancy 0.3, four equal initial proportions, seed 1 and 500 ticks. Its bundles are A `(4,5,1,1)`, B `(4,3,2,1)`, C `(6,2,1,1)` and D `(2,6,2,1)`. These are adjustable experimental settings, **not scientifically calibrated values**. They illustrate different tendencies; no hidden mechanism tunes a winner or maintains coexistence. Both autonomous modes share the following initialization procedure.

Occupancy specifies an **exact total**, rounded down: `floor(width × height × occupancy)`. For 768 squares and 0.3, place 230 agents. Initial proportions are nonnegative integer ratios. First combine proportions of identical bundles in first-appearance order. Allocate each group's exact quota by rounding down, then give the leftover individuals to the largest fractional remainders, breaking ties by that canonical group order. Equal ratios therefore yield 58, 58, 57, 57 agents. Zero ratios produce zero initial agents and remain visible; the total ratio must be positive even on an empty grid. The actual canonical groups, ratios and initial counts are recorded in browser and headless output.

Shuffle all row-major square indices with descending Fisher–Yates; assign successive shuffled slots to the groups' exact quotas, then assign starting IDs 1, 2, … in occupied row-major order. This produces random placement without overlaps, including exact empty/full grids. Dimensions are at least 3 in each direction and at most 262,144 squares for bounded local allocations. Occupancy accepts up to six decimal places from 0 through 1. Each weight/ratio is a `u32`; totals and quota arithmetic use wider integers.

### Reproducibility contract

The generator is **SplitMix64**, using [Vigna's public-domain 2015 reference](https://prng.di.unimi.it/splitmix64.c), with `VirtualLife sampling v1` bounded draws in `src/experiment.rs`. The original action protocol is separately labelled **random v1**. Seed is the initial unsigned 64-bit state. Each raw draw adds `0x9e3779b97f4a7c15` with wrapping arithmetic, then mixes with shifts 30/27/31 and multipliers `0xbf58476d1ce4e5b9` and `0x94d049bb133111eb`. Reference output vectors are tested. This generator is for reproducible experiments, not security.

To draw uniformly below N, discard raw values below `(-N modulo 2^64) modulo N`, then return the accepted value modulo N. Rejected raw values advance the generator. This avoids remainder bias. Initialization draws once per Fisher–Yates iteration (plus any rejected raw values), from the last square through index 1, even for an empty population. Quota calculation and ID allocation consume no randomness. The same generator then continues into ticks. Iterate occupied starting squares in row-major order: draw one weighted action per individual, then draw a destination only for move/copy. Neighbours have the stable order northwest, north, northeast, west, east, southwest, south, southeast, with wraparound. Failed destinations never retry and still consume their original draws. Newborns enter only the next tick's iteration.

Record the seed, effective configuration, generator/protocol name and crate version with results; keep the source commit/Cargo.lock for exact reproduction. The same configuration and code produces the same fixed-tick world and accepted-event totals with observation disabled, enabled, saturated, disconnected or sampled differently. Rendering, pacing and the HTTP adapter's OS-random run identity consume no simulation draws. A seed alone does not promise identical outcomes across arbitrary code or dependency changes. Live samples are bounded and incomplete, not a saved trajectory.

### Limits of the local rule

Action selection reads starting occupancy through `neighbors` and `agent_at`. The approved crowding rule considers whether a square is empty, not its occupant's properties or condition. Transition resolution and presentation do not need duplicate implementations. No neighbourhood framework or interaction matrix is introduced.

Further neighbour-property-dependent choices, resources, energy economy, mutation and interaction matrices remain outside the model. Choose the question and exact rule with worked examples before adding another interaction.
