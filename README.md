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
cargo run --locked --features web --bin web
```

Open the printed address, normally **http://127.0.0.1:7878**, in your browser. Use Resume, Pause or Single step, and click an agent to inspect it. The page shows the grid, population count and event totals.

The executable currently runs a **five-tick scripted demonstration**, not autonomous behaviour. It starts paused and keeps the final state visible when finished.

Stop the server with **Ctrl+C in the terminal**; closing the browser does not stop the experiment. To repeat it, restart the command. An open page reconnects automatically, clears its old history and selection, and shows a new-experiment notice.

To run without a browser:

```sh
cargo run --locked --no-default-features --bin headless -- --ticks 5
```

Node is not needed to run either mode. See the [development reference](docs/development.md#running-options) for additional launch options.

## Develop

Start with [MODEL.md](MODEL.md) for the rules and worked example. [src/demo.rs](src/demo.rs) supplies the script, [src/engine.rs](src/engine.rs) applies transitions, and [src/runner.rs](src/runner.rs) handles execution. The local server lives in `src/web.rs`; the browser interface lives in `web/`.

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
