# VirtualLife

An algorithm designer's experiment with a biologist's curiosity: what behavior can emerge from simple local interactions between individual agents?

The aim is not realistic biology, particle physics, or complicated equations. Understandable, elegant, efficient algorithms and data structures are outcomes in their own right. Biological descriptions are interpretations of observations, not programmed objectives. Resources are agents with different properties, not a separate class of objects.

## Current state and next step

**6 September 2026 — documentation bootstrap only.** The Rust restart is on `restart/rust-v0.1`, based on historical commit `98225432ca878a064045d3505ea6f3b6e1180488`. No Rust application, Cargo manifest, tests, or CI have been implemented here yet. The inherited Python files remain unchanged for now; they do not implement the new specification.

The next bounded task is **Task 01: a trustworthy live grid**, tracked in the [repository issues](https://github.com/ppericard/virtual-life/issues). It will demonstrate synchronous movement, creation, removal, and property changes, with a live viewer and headless execution. Its actions are scripted tests, not autonomous behavior or evidence of emergence. Use the task issue's pinned base commit; do not recreate the restart from the historical branch.

The next checkpoint is review of the actual diff, checks, and live behavior, with a beginner-friendly walkthrough. Then choose the first autonomous local rules with Pierre. Those rules are not yet decided.

## Starting direction

Use Rust, a 2D wraparound grid, eight neighboring squares, and at most one agent per square. Agents represent individuals at comparable spatial scales. Decisions use the unchanged starting world; each starting agent acts at most once per tick and new agents first act next tick. Creation, removal, and property changes belong early.

Live visualization and useful measurements are part of the first working prototype. The same engine must also run headlessly: rendering must not determine simulation steps or outcomes. State editing and modest post-run analysis follow early, without delaying the live tool.

Multiple occupancy, different sizes, containment, and continuous space remain future possibilities, not abstractions to build now. Reconsider them only for an explicit experimental need.

## Read and contribute

- [MODEL.md](MODEL.md): confirmed constraints, demonstrator rules, worked example, and open model questions.
- [AGENTS.md](AGENTS.md): short contribution guide for humans and AI agents; task boundaries, checks, and handoffs.
- `src/` and `old/`: inherited historical Python, not the Rust code map. Replace this entry with the actual Rust reading order when implemented.
- [LICENSE](LICENSE): the existing license, preserved unchanged.

These repository documents carry forward the maintained content of the September 2026 vision brief and handoff. Earlier chat attachments are historical snapshots, not competing live specifications. Git and task/PR discussions hold routine history. The model describes intended behavior; code and execution show what actually happens. Investigate disagreements.

Pierre directs the project; the lead defines bounded tasks and reviews results. Work implements one task at a time by default, on a separate task branch, returning a diff/commit and evidence before further scope. Assume no Rust knowledge: explain the algorithm with small examples, then the syntax, with Python/C/C++ refreshers where useful.

## Implementation starting point

Start with one small Cargo package: an engine library and headless/viewer launch paths. Prefer ordinary structs, vectors, functions, and standard threading tools. A flat grid with current/next buffers is the starting representation. Keep the engine free of GUI dependencies, drawing, file output, sleeps, and wall-clock pacing. An optional native egui/eframe viewer is the current implementation preference, not part of the model; verify its current API when implementing.

A simulation worker advances ticks independently and provides bounded, nonblocking, tick-stamped snapshots to the UI. A slow viewer may miss samples, not simulation steps. Count accepted events in the transition, not from displayed frames. Observation has a cost; do not promise identical throughput or complete histories from sampled data.

No initial ECS, generic rule language, plugin system, async runtime, GPU engine, database, or distributed agent team. Add complexity only for a concrete benefit; measure performance claims rather than guessing.

## Setup and verification

There are no Rust run commands to execute yet. Task 01 must document the commands actually established: formatting, Cargo tests, Clippy, headless execution, and the optional GUI build, including toolchain/dependency versions and platform requirements. Record dependency resolution in `Cargo.lock` when the application exists. Do not present planned checks as passing checks.

Test scheduling, conflicts, identity, occupancy, counts, invalid-input handling, and observation independence. Distinguish built, executed, visually inspected, reviewed, and accepted. If a native window cannot be displayed in the execution environment, report that limitation and provide a short manual check.

The original Python work remains available on [original-all-manual](https://github.com/ppericard/virtual-life/tree/original-all-manual). Its biological examples were intended as possible emergent outcomes. Repeated activation, same-tick activation of newly created agents, and acting after expiry were bugs; display coupling was unfinished implementation. Do not reproduce them as intended rules.
