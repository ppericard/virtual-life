# VirtualLife

VirtualLife is a multi-agent laboratory for studying emergent behavior from simple mathematical rules.

The project does **not** aim to simulate biology directly. Agents are generic mathematical entities with state and local interactions. Terms such as predation, resources, species, or genetics may be useful descriptions of observed patterns, but they should not be built into the engine unless the mathematics independently requires them.

The code is part of the experiment: keep the model explicit, the implementation small and readable, and the data structures as simple as possible until measurements justify something more sophisticated.

## Current status

This branch is the Rust reboot foundation. The bootstrap model deliberately does only enough to validate the architecture:

- generic agents with integer properties;
- a toroidal grid;
- any number of agents per tile;
- deterministic random-sequential turns;
- a headless runner;
- an optional graphical viewer consuming read-only snapshots without blocking the simulation.

The bootstrap movement rule is infrastructure, not a claim about what the first emergence experiment should be. Its exact semantics and open questions are in [`docs/MODEL.md`](docs/MODEL.md).

Project principles and instructions for AI/human contributors are in [`AGENTS.md`](AGENTS.md).

The original 2014-2016 implementation is preserved on branch [`original-all-manual`](https://github.com/ppericard/virtual-life/tree/original-all-manual).

## Run

Requires stable Rust with edition 2024 support.

```bash
cargo test
cargo run --release -- 100000
```

The optional viewer uses `eframe`/`egui`:

```bash
cargo run --release --features viewer --bin virtual-life-viewer
```

The viewer runs the simulation on a worker thread and uses bounded, non-blocking snapshot delivery. If rendering falls behind, visual samples are dropped rather than slowing the engine.

## Development rule

Before changing the model, read `AGENTS.md` and `docs/MODEL.md`. Prefer a small concrete implementation over a generalized framework. Benchmark before optimizing.
