# VirtualLife

What kinds of behaviour can emerge from simple interactions between individuals?

VirtualLife is a Rust experiment exploring that question through worlds of agents. It combines a biologist's curiosity with an algorithm designer's interest in simple, elegant data structures and procedures. Understanding how the world works matters as much as what happens inside it.

## The idea

The goal is an interactive place to explore: give individuals a few properties and local rules, watch populations change, inspect particular agents, and investigate the patterns that appear. Over time, the tools should support changing a world while it runs and exporting observations for further analysis.

Agents are individuals, not predefined species or roles. Even resources are agents with different properties. Biological-looking behaviour is something to discover and interpret, not a result to hardcode.

The starting world is a 2D grid with wraparound edges, eight neighbouring squares and at most one agent per square. Each tick uses the same starting state for all agents. The simulation runs independently of its viewer, so watching it does not change its outcomes. The aim is exploration through understandable algorithms, rather than realistic biology or particle physics.

## Run

Install Git and [Rust through rustup](https://rust-lang.org/tools/install/). On Windows, Rust also needs the MSVC build tools. The repository selects its [Rust toolchain](rust-toolchain.toml) automatically.

```sh
git clone https://github.com/ppericard/virtual-life.git
cd virtual-life
cargo run --locked --features web --bin web -- --mode autonomous --automaton-preset mixed
```

Open the printed address, normally **http://127.0.0.1:7878**, in your browser. Use Resume, Pause or Single step, and click an agent to inspect it. The page shows a reproducibly populated grid, four property groups with colours and letters, individual inspection, population trajectories and accepted event totals.

The whole world fits below compact run controls on desktop, with square cells and exact total/group counts. On narrow screens the world appears directly below the controls. Click an occupied square or choose **Inspect** to follow an agent. Automaton runs show its state graph and current transition probabilities; the selector explores inherited arrows from each state. Integrity adjusts Repair and Copy, while crowding adjusts Move and Copy. The last selected action may have failed; initial agents and newborns have not yet acted.

Open **Analysis** for the existing population plot, accepted event totals and failure records, or **Details** for inherited weights, effective settings and rules. Sampling continues while Analysis is closed. Panel toggles and resizing preserve observations and do not send simulation commands; connection errors and uncertain outcomes stay visible above the world.

Without an automaton option, the flat comparison experiment starts with 230 randomly placed agents in a 32 × 24 grid. Each chooses wait, move, copy or repair from its group's inherited base weights. Crowding scales Copy's chance by the fraction of empty neighbouring squares and transfers the rest to Wait. Move/Copy still target any of the eight neighbours, so attempts can fail. Base upkeep is 1, with an extra 1 when at least five starting neighbours are occupied; Move/Copy add wear and Repair restores integrity. Failure to maintain positive integrity removes the individual. Inspect current neighbours, potential next-tick upkeep and recorded failure costs alongside population changes. These adjustable settings are illustrative, not calibrated biology; continued survival and extinction are valid outcomes. It starts paused and keeps the final state visible after 500 ticks. Use `--crowding-threshold` and `--crowding-upkeep` to adjust the surcharge (`--crowding-upkeep 0` reproduces v2 trajectories with matching settings), `--survival random` for the earlier random-removal comparison without crowding, or `--mode demo` for the five-tick scripted regression demonstration.

Open **New experiment** and choose **Mixed automata** or one of **Movement runs**, **Repair cycles**, **Copy bursts** and **Wait cycles**. Preview inherited arrows under **Next restart graphs and weights**. The earlier **Original**, **Moderate movement**, **Wide movement range** and **Lower copying** flat-weight comparisons remain available. Selection stays pending until restart and replaces inherited properties and proportions using equal shares. **Keep current custom settings** cancels a pending choice for a custom run. Closing the panel retains edits.

Use **Restart with seed** to apply the choice with a fixed seed, or **Restart with random seed** to explore a new one. The seed starts with the launched value (normally 1); a random restart displays its chosen seed for reproduction. Restarts retain all other current settings, start paused at tick 0, and clear previous histories and selection. A zero-tick run starts completed. Presets are available in wear-repair mode, including after completion; they are illustrative combinations without a promise of coexistence.

Stop the server with **Ctrl+C in the terminal**; closing the browser does not stop the experiment. Restart the command to repeat the scripted demo or change launch settings. An open page follows a new experiment automatically and shows a notice. A lost restart reply has an unknown outcome: check the current experiment and seed before trying again; requests are never automatically retried.

To run without a browser:

```sh
cargo run --locked --no-default-features --bin headless -- --mode autonomous --seed 1 --ticks 500
```

Node is not needed to run either mode. See the [development reference](docs/development.md#running-options) for additional launch options.

## Develop

Start with [MODEL.md](MODEL.md) for the rules and worked example. [src/experiment.rs](src/experiment.rs) initializes the autonomous world and chooses actions; [src/demo.rs](src/demo.rs) supplies the regression script, [src/engine.rs](src/engine.rs) applies transitions, and [src/runner.rs](src/runner.rs) handles execution. The local server lives in `src/web.rs`; the browser interface lives in `web/`.

Basic Rust checks:

```sh
cargo fmt --all -- --check
cargo test --locked --no-default-features
cargo test --locked --features web
```

For frontend checks and real-browser tests, install Node 24.19.0 and follow the [platform prerequisites](docs/development.md#verification), then run:

```sh
npm ci
npm run check
npx playwright install --with-deps chromium
npm run test:e2e
```

Browser assets are embedded in the Rust executable: after editing them, restart the server through `cargo run` and reload the page.

The [development reference](docs/development.md) covers the code-reading guide, API and full verification checklist, including release tests, Clippy and dependency audits. Use task branches and pull requests into `master`, and read [AGENTS.md](AGENTS.md) for contributor guidance. Bugs, ideas and ongoing work belong in [GitHub issues](https://github.com/ppericard/virtual-life/issues).

[License](LICENSE)
