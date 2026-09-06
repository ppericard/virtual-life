# VirtualLife

An algorithm designer's experiment with a biologist's curiosity: what behavior can emerge from simple local interactions between individual agents?

The aim is not realistic biology, particle physics, or complicated equations. Understandable, elegant, efficient algorithms and data structures are outcomes in their own right. Biological descriptions are interpretations of observations, not programmed objectives. Resources are agents with different properties, not a separate class of objects.

## Current state and next step

**6 September 2026 — Task 01 implemented, awaiting review.** One small Rust package implements the [scripted live-grid assignment](https://github.com/ppericard/virtual-life/issues/1): synchronous movement, creation, removal, value changes, conflicts, headless execution, and an optional native viewer. The task branch starts at `edcf0783cd09e55989597571526c961ffb0e62d9` and targets `restart/rust-v0.1`. The five scripted transitions test machinery, not autonomous behavior or emergence.

The next checkpoint is the lead's review of the actual diff and verification evidence, followed by a live walkthrough with Pierre. Then choose the first autonomous local rules with Pierre; those rules are still open. State editing and modest exports are early follow-ups, outside this task. No merging, release, or next-task implementation is implied.

## Starting direction

Use Rust, a 2D wraparound grid, eight neighboring squares, and at most one agent per square. Agents represent individuals at comparable spatial scales. Decisions use the unchanged starting world; each starting agent acts at most once per tick, and new agents first act next tick. Creation, removal, and property changes belong early.

Live visualization and useful measurements are part of the prototype. The same engine runs headlessly: rendering cannot determine simulation steps or outcomes. Multiple occupancy, different sizes, containment, and continuous space remain future possibilities, not abstractions to build now. Reconsider them only for an explicit experimental need.

## Setup and run

Install [Rust through rustup](https://rust-lang.org/tools/install/) and your platform's native linker/build tools. `rust-toolchain.toml` selects **Rust/Cargo 1.95.0**, with rustfmt and Clippy; rustup downloads that toolchain when you first run Cargo here. `Cargo.lock` records the resolved dependencies. No Python setup is needed.

Headless commands need no graphics backend and build only the standard-library engine/runner:

```sh
cargo run --locked --no-default-features --bin headless -- --ticks 5
cargo run --locked --no-default-features --bin headless -- --ticks 0
cargo run --locked --no-default-features --bin headless -- --ticks 8
```

The default is five ticks. Tick 0 reports the initial three agents. At tick 5, IDs 2 and 5 both have value 21, at `(2,2)` and `(1,2)`. Totals are two moves, two creations, three removals, and two value changes. Requests beyond five execute all-wait ticks: the tick advances, but the final state and event totals remain unchanged. Negative, malformed, or overflowing tick counts fail with an error.

For a native window:

```sh
cargo run --locked --features viewer --bin viewer
cargo run --locked --features viewer --bin viewer -- --running --tick-ms 1000 --sample-every 2
```

The viewer starts paused at tick 0, samples every tick, and uses a 750 ms minimum interval when resumed. Pause, resume, and single-step apply between ticks; single-step works only while paused. Click a square to inspect the agent's stable ID and value. Colors identify IDs, without biological roles. The demonstration ends at tick 5; close and relaunch to repeat it. Closing also stops and joins its worker.

`--sample-every N` must be positive; controls and completion always request a sample. `--tick-ms 0` removes pacing, so a viewer may see only the beginning and end of this short fixture. The interval is a speed ceiling, not a real-time timing guarantee. Both binaries support `--help`.

The optional GUI uses **egui/eframe 0.36.1** and its OpenGL (`glow`) renderer, with X11 and Wayland support on Linux. Native rendering requires a desktop display and working graphics libraries/drivers. Linux needs a C linker and any development libraries requested by the backend; Windows needs the Rust MSVC build tools, and macOS needs Xcode command-line tools. Windows, macOS, and Wayland runtime behavior have not been tested here. No web viewer is supplied.

The implementation was checked against the versioned [eframe API/source](https://github.com/emilk/egui/blob/0.36.1/crates/eframe/src/epi.rs) (`App::ui`, `run_native`) and [egui painter API](https://docs.rs/egui/0.36.1/egui/struct.Painter.html). The small count plot uses egui's painter directly, without another plotting dependency.

## Algorithm and code-reading order

Read [MODEL.md](MODEL.md) for the intended rules and five-tick table, then:

1. **`src/demo.rs`** supplies the initial agents and proposals for each tick. This is the script, without autonomous decisions.
2. **`src/engine.rs`**, especially `World::step`, performs the transition. Validate all proposals and attach each to its actor's starting square. Count claims on initially empty destinations. Turn occupied/conflicting claims into waits. Compute accepted totals and check counter/ID capacity. Copy the starting grid into the reusable next buffer, apply accepted actions once, then swap buffers.
3. **`src/runner.rs`** owns the engine on a standard thread, applies controls between ticks, and sends independent snapshots through a bounded channel. Pacing lives here. **`src/bin/headless.rs`** runs that worker without an observer and prints its result.
4. **`src/bin/viewer.rs`** receives snapshots and draws the grid, inspection, controls, totals, and bounded plot. It never advances or edits the engine. **`tests/`** exercises model behavior, worker boundaries, and the actual headless executable.

For example, A and B both claim `(1,2)`, so both wait. When A removes itself, its square was still occupied at the start; C must wait until the following tick to enter. A child exists only in the next grid, so it cannot be activated by this tick's loop.

Some Rust vocabulary: a `struct` groups named fields, like a small C struct or Python dataclass. `Vec` is a contiguous dynamic array. `Option<Agent>` means `Some(agent)` or `None` (empty); index `(x,y)` as `y * width + x`. `&World` borrows for reading; `&mut World` permits exclusive mutation. Transferring ownership into the worker avoids sharing a mutable world with the UI. Swapping buffers changes which array is current without copying the finished grid back.

The tiny fixture uses linear ID lookup. With G squares and P proposals, validation takes O(G × P), plus O(G) transition passes. This is a simplicity trade-off, not a large-world performance claim. Measure before adding an index or other optimization.

## Observation and limits

The channel holds one queued snapshot; the worker holds at most one pending snapshot, replaced by a newer sample. It uses [`try_send`](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send), never a blocking snapshot send. Cloning happens at sample/control/completion boundaries, not every fast tick. A paused or completed worker retries pending delivery until it succeeds, the viewer disconnects, or shutdown is requested. A separate one-shot completion signal lets tests establish that computation finished even with an unread buffer.

The UI retains at most 128 `(tick, count)` points, labels sampling gaps, and leaves them unconnected. Samples are not a complete event log or replay. Accepted-event totals are counted inside transitions, so missing frames cannot hide events from those totals. Observation costs time and memory; identical outcomes do not imply identical throughput. User controls intentionally affect a run and differ from observation.

Agents have just an ID and an integer value. No autonomous rules, randomness, automatic expiry, energy economy, species hierarchy, generic property system, multiple occupancy, world editing, save/reload, exports, or post-run analysis framework is implemented. See MODEL for allocation order, no-op value changes, integer exhaustion, and open research decisions.

## Verification

Run from the package root:

```sh
cargo fmt --all -- --check
cargo test --locked --no-default-features
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo build --locked --features viewer --bin viewer
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
git diff --check
```

Results on Ubuntu 24.04 x86_64, Rust/Cargo 1.95.0:

| Check | Actual result |
|---|---|
| Formatting and `git diff --check` | Pass |
| Headless tests, with default features disabled | 21 pass |
| Headless Clippy, all targets, warnings denied | Pass |
| Headless runs at 0, 5, and 8 ticks | Expected states and totals |
| Optional GUI build | Pass with the environment workaround below |
| All-feature tests | 22 pass with the workaround, including the viewer completion/control race regression |
| All-feature Clippy, all targets, warnings denied | Pass with the workaround |
| Native window | **Built, not visually verified**: launch reports no DISPLAY/Wayland display; Xvfb also could not establish a listening socket here |

The normal GUI debug build failed in this execution environment with corrupt metadata or empty object files in dependencies. Reduced debug information and a single codegen unit produced a successful build. This is an environment observation, not an established upstream bug; changing toolchains or trying an older egui release did not resolve it. The final package keeps egui/eframe 0.36.1 and has no permanent build-profile overrides. The exact successful GUI commands were:

```sh
export CARGO_PROFILE_DEV_CODEGEN_UNITS=1
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_CODEGEN_UNITS=1
export CARGO_PROFILE_TEST_DEBUG=0
cargo build --locked --features viewer --bin viewer -j 2
cargo test --locked --all-features -j 2
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Use the same variables with `cargo run` if your environment exhibits this build problem. `target/debug/viewer --help` passed, and zero sample cadence was rejected. Native launch was attempted but displayed no window. The viewer regression test exercises its control/worker logic without drawing; it is not visual evidence. Tests compare complete worlds and allocation state, not just counts. Ten-second timeouts guard against deadlock; they are not throughput thresholds. No cross-platform execution or performance benchmark is claimed.

Manual native check: launch paused; single-step through MODEL's table, inspect IDs/values (including a removed selection), try pause/resume, and check completion totals and count points. Relaunch with `--running --tick-ms 1000 --sample-every 2` to see gaps. Close while paused and while running to check clean termination. A successful compilation alone is **built, not visually verified**.

## Continuity and contribution

[AGENTS.md](AGENTS.md) holds the short contributor guide; [MODEL.md](MODEL.md) states intended behavior. Code and execution show actual behavior. Investigate disagreement. Pierre controls direction; the lead reviews bounded tasks. These maintained documents supersede planning attachments; Git and task/PR discussions hold routine history. Do not assume `master` is the restart branch.

Inherited `.py` files under `src/` and `old/` remain untouched historical reference, unused by Cargo. The original work is also preserved on [original-all-manual](https://github.com/ppericard/virtual-life/tree/original-all-manual). Repeated activation, same-tick newborn activation, and acting after expiry were bugs; display coupling was unfinished implementation. Do not restore those as model rules. [LICENSE](LICENSE) remains unchanged.
