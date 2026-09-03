# Bootstrap model

This document defines the executable semantics of the first Rust version. It is intentionally small and is not the first serious emergence hypothesis.

## State

The environment is a finite two-dimensional toroidal grid of width `W` and height `H`.

An agent has:

- a stable identifier `id` used only for bookkeeping and scheduling;
- a position `(x, y)`;
- an ordered vector of integer properties.

A tile contains a set/list of zero or more agent identifiers. Agents are stored independently from tiles, so stacking is native to the model and population size may change later.

Agent identifiers have no effect on transition rules.

## Initialization

A run is fully determined by its configuration and random seed. The bootstrap initializer places `N` agents independently at uniformly random grid positions and assigns each property an integer uniformly sampled from `0..=255`.

These property values currently have no effect on behavior; they exist to establish generic agent state.

## One turn

The bootstrap scheduler is random sequential:

1. Take exactly the agents present at the beginning of the turn.
2. Uniformly shuffle their identifiers using the run RNG.
3. In that order, each agent chooses uniformly among its eight Moore-neighbor positions.
4. Move the agent there, wrapping at grid boundaries.
5. Increment the turn counter once after every scheduled agent has acted.

Stacking is allowed, so movement needs no collision rule. Every starting agent acts exactly once per turn regardless of where it moves.

Properties do not change in this bootstrap rule.

## Observation

A snapshot is a read-only copy of the turn number, environment dimensions, positions, identifiers, and properties. Observation is not part of the transition function.

A viewer may sample or drop snapshots at any frequency. This must not change simulation state or execution speed except for the explicit cost of producing a requested snapshot.

## Invariants currently tested

- Equal configuration + equal seed => equal trajectory.
- Stacking is representable directly.
- Toroidal offsets are correct.
- Moving an agent keeps spatial indexes consistent.
- Bootstrap turns preserve population and all properties.

## Deliberately unresolved

The first research model still needs explicit decisions about:

- which agent properties exist and their domains;
- which local interactions are possible;
- whether positions, agents, or both may be created/removed;
- how interaction partners are selected;
- conflict semantics for rules that modify several agents;
- useful conservation laws and symmetries;
- what measurements help detect emergence without defining it into existence.

Do not answer these by importing biological concepts. They are mathematical design questions.
