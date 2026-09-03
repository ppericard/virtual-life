# VirtualLife contributor guide

Read this file before changing the project. Keep it short; update it only when a durable principle or workflow changes.

## Purpose

VirtualLife studies how complex behavior can emerge from simple mathematical rules applied to generic agents. The implementation is part of the intellectual work: prefer code, algorithms, and data structures that are clear, minimal, elegant, and efficient.

## Non-negotiable principles

- Program rules, not biological interpretations.
- Do not add engine concepts such as predator, prey, food, resource, species, genome, reproduction, or death merely because an observed behavior resembles biology.
- Agents are generic stateful entities. Biological vocabulary belongs in analysis or discussion unless the mathematical model itself proves it necessary.
- Multiple agents may occupy the same environment position. Do not encode occupancy as a single optional agent per tile.
- The simulation engine must be independent of visualization, recording, and analysis. Attaching a viewer must never change a trajectory or throttle a headless run.
- Scheduling and randomness are part of the model semantics. Never let container iteration order, thread timing, or UI timing define them accidentally.
- Start with the simplest correct representation. Add sophistication only for a demonstrated capability, clarity, or performance benefit.
- Keep a transparent reference implementation when optimizing; optimized paths must be checked against it.
- Rust is new to the project owner. Prefer ordinary structs, enums, vectors, functions, and explicit control flow over clever language machinery. Document non-obvious Rust ownership or representation choices.

## Working style

This is a lean research project, not an enterprise platform.

1. Read `README.md`, this file, and `docs/MODEL.md`.
2. Make small reviewable changes with tests for semantic invariants.
3. Update documentation only when the model, architecture, or a durable decision changes.
4. Do not create new process/governance documents unless existing files cannot hold genuinely necessary information.
5. Benchmark before optimizing; record why a more complex structure was introduced.
6. Keep dependencies few and justified.

## Current architecture

- Rust library: mathematical world and simulation semantics only.
- `virtual-life`: headless runner.
- `virtual-life-viewer`: optional egui/eframe viewer consuming read-only snapshots on another thread.
- Analysis tooling may later use Python/R and recorded data; it must remain downstream of the engine.

The bootstrap rule in `docs/MODEL.md` is intentionally uninteresting. Its job is to validate deterministic scheduling, stacking, observation boundaries, and the Rust foundation before designing the first emergence experiment.

## Historical context

The 2014-2016 implementation is preserved on branch `original-all-manual`. It is provenance, not an architecture constraint for the reboot.
