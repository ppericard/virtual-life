# VirtualLife

An algorithm designer's experiment with a biologist's curiosity: what behavior can emerge from simple local interactions between individual agents?

The aim is not realistic biology, particle physics, or complicated equations. Understandable, elegant, efficient algorithms and data structures are outcomes in their own right. Biological descriptions are interpretations of observations, not programmed objectives. Resources are agents with different properties, not a separate class of objects.

## Current state and next step

**6 September 2026 — Rust development baseline on `master`.** The [Task 01 implementation in PR #2](https://github.com/ppericard/virtual-life/pull/2), at `a9e2ebffc2d6935324751b428101ca1f30901a5d`, supplies synchronous movement, creation, removal, value changes, conflicts, headless execution, and an optional native viewer. The five scripted transitions test machinery, not autonomous behavior or emergence. A source review found no blocking defect in that fixture. The migration preserves its Rust source, tests, toolchain, dependencies, and license unchanged; legacy Python is removed from the active tree and retained in archival branches below.

**Verification is incomplete:** Work reports 21 headless and 22 all-feature tests passing. The lead reviewed the code and assertions but could not rerun Cargo in the review environment. No repository-owned CI exists yet, and the native GUI is built but not visually verified. This is an integrated development baseline, not an accepted release. The detailed review is in PR #2.

**Task 02 is in progress:** [browser viewer and verification CI (#3)](https://github.com/ppericard/virtual-life/issues/3), based on `4943a324fe610612118ec4e927d70f5b87c12697`, targeting `master`. Pierre chose a small local browser interface on 6 September 2026. This decision supersedes the earlier proposal/native-only wording. The native Rust engine and headless executable stay; the native viewer will be removed after browser replacement coverage passes.

The first increment adds bounded control requests with worker-applied receipts, exhaustive small-world invariants, integer/full-grid boundary checks, and bounded test cleanup. **29 headless tests pass locally on Linux with the pinned Rust 1.95.0 toolchain.** The engine transition function is unchanged. Browser parity, visual inspection, and repository-owned CI are still pending at this increment. The older native instructions/results below are transitional Task 01 references, not browser verification.


## Starting direction

Use Rust, a 2D wraparound grid, eight neighboring squares, and at most one agent per square. Agents represent individuals at comparable spatial scales. Decisions use the unchanged starting world; each starting agent acts at most once per tick, and new agents first act next tick. Creation, removal, and property changes belong early.

Live visualization and useful measurements are part of the prototype. The same engine runs headlessly: rendering cannot determine simulation steps or outcomes. Multiple occupancy, different sizes, containment, and continuous space remain future possibilities, not abstractions to build now. Reconsider them only for an explicit experimental need. Simple algorithms do not excuse weak verification: testing, reproducible builds, CI, and review are first-class engineering work.

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

The following results were reported by the Task 01 implementer on Ubuntu 24.04 x86_64, Rust/Cargo 1.95.0, for commit `a9e2ebffc2d6935324751b428101ca1f30901a5d`. They are not CI results or independent lead reruns:

| Check | Work-reported result |
|---|---|
| Formatting and `git diff --check` | Pass |
| Headless tests, with default features disabled | 21 pass |
| Headless Clippy, all targets, warnings denied | Pass |
| Headless runs at 0, 5, and 8 ticks | Expected states and totals |
| Optional GUI build | Pass with the environment workaround below |
| All-feature tests | 22 pass with the workaround, including the viewer completion/control race regression |
| All-feature Clippy, all targets, warnings denied | Pass with the workaround |
| Native window | **Built, not visually verified**: launch reports no DISPLAY/Wayland display; Xvfb also could not establish a listening socket there |

The normal GUI debug build failed in Work's execution environment with corrupt metadata or empty object files in dependencies. Reduced debug information and a single codegen unit produced a successful build. This is an environment observation, not an established upstream bug; changing toolchains or trying an older egui release did not resolve it. The final package keeps egui/eframe 0.36.1 and has no permanent build-profile overrides. The exact successful GUI commands were:

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

Task 02 must exercise the ordinary commands on clean runners before adopting any environment workaround. Add release-mode headless testing, generated small-world invariant tests, targeted boundary cases, and actual workflow evidence. Coverage and mutation testing can guide subsequent review; an arbitrary percentage or a green badge alone is not evidence of correctness. Keep local and CI commands aligned and document skipped or failed checks.

## Continuity and contribution

[AGENTS.md](AGENTS.md) holds the short contributor guide; [MODEL.md](MODEL.md) states intended behavior. Code and execution show actual behavior. Investigate disagreement. Pierre controls direction; the lead reviews bounded tasks. These maintained documents supersede planning attachments; Git and task/PR discussions hold routine history. **`master` is now the active integration and default branch.** Use issues, task branches, and PRs into master; do not continue new work against the retired restart integration branch.

Historical tips were verified before migration:

| Archive | Preserved commit |
|---|---|
| [archive/master-before-rust](https://github.com/ppericard/virtual-life/tree/archive/master-before-rust) | `4caa8fc5d4ce0672c579a6fb4dcd5c8929d375c3` |
| [archive/comprehensive-improvements](https://github.com/ppericard/virtual-life/tree/archive/comprehensive-improvements) | `31ab057b57fcdaa2925483b0eea7328685578c79` |
| [archive/original-all-manual](https://github.com/ppericard/virtual-life/tree/archive/original-all-manual) | `98225432ca878a064045d3505ea6f3b6e1180488` |

Original historical branch names remain available so existing links keep working. `restart/rust-v0.1` and `work/task-01-live-grid` remain as prior-work references, not competing active development lines. The one-time master migration retains old master and the Task 01 head as parents, preserving both histories without a force-push. It removes inherited `.py` files and their obsolete ignore file from the active tree. [LICENSE](LICENSE) is unchanged. Repeated activation, same-tick newborn activation, and acting after expiry in the original Python were bugs; display coupling was unfinished implementation. Do not restore them as model rules.
