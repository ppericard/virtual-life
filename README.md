# VirtualLife

An algorithm designer's experiment with a biologist's curiosity: what behavior can emerge from simple local interactions between individual agents?

The aim is not realistic biology, particle physics, or complicated equations. Understandable, elegant, efficient algorithms and data structures are outcomes in their own right. Biological descriptions are interpretations of observations, not programmed objectives. Resources are agents with different properties, not a separate class of objects.

## Current state and next step

**6 September 2026 — Task 02 browser demonstrator, awaiting review in [draft PR #4](https://github.com/ppericard/virtual-life/pull/4).** Pierre chose a local browser GUI to replace the native viewer. The native Rust engine, synchronous transitions, five-tick script, integer domains, and headless executable remain. The native frontend and egui/eframe dependencies have been removed after replacement browser coverage passed; their history remains in PR #2. This is scripted behavior, not emergence or an accepted autonomous model.

Task [#3](https://github.com/ppericard/virtual-life/issues/3) starts at `4943a324fe610612118ec4e927d70f5b87c12697`; work uses a task branch and a draft PR into `master`. The next checkpoint is review of the working browser demonstrator and verification, then selection of the first autonomous local rules with Pierre. World editing and modest exports remain future project goals, outside this task.

## Setup and run

Install [Rust through rustup](https://rust-lang.org/tools/install/) and your platform's linker/build tools (MSVC build tools on Windows). `rust-toolchain.toml` selects **Rust 1.95.0**, rustfmt, and Clippy. Cargo downloads the pinned toolchain when needed. `Cargo.lock` records dependencies. No graphics libraries, Python setup, Node application server, or cloud service is needed.

Headless builds use only the standard-library engine and runner:

```sh
cargo run --locked --no-default-features --bin headless -- --ticks 0
cargo run --locked --no-default-features --bin headless -- --ticks 5
cargo run --locked --no-default-features --bin headless -- --ticks 8
```

The default is five ticks. Tick 0 has three agents. At tick 5, IDs 2 and 5 have value 21 at `(2,2)` and `(1,2)`, respectively. Totals are two moves, two creations, three removals, and two value changes. Headless requests beyond five run all-wait ticks: only the tick advances. Invalid or overflowing arguments fail with a message.

For the browser interface:

```sh
cargo run --locked --features web --bin web
cargo run --locked --features web --bin web -- --running --tick-ms 1000 --sample-every 2
```

Open the printed address, normally **http://127.0.0.1:7878**. The first command starts paused at tick 0, samples every tick, and has a 750 ms minimum tick interval when resumed. Click Resume, Pause, or Single step. Click an occupied grid square (or use the Agent selector) to follow its stable ID, value, and position through movement and removal. Colors distinguish IDs, not biological roles. The demonstration finishes at tick 5; controls are then disabled and the final state remains available.

`--port N` selects a loopback port (`0` chooses a free one). `--sample-every N` must be positive. `--tick-ms N` accepts 0–60000; zero removes pacing and may leave only the initial/final samples visible. Pacing is a speed ceiling, not a real-time guarantee. Both binaries support `--help`.

**Lifecycle:** the Rust process owns one experiment. Refreshing, closing, hiding, or disconnecting a page neither resets nor pauses nor stops it. A new page reads the current/final cache. Its plot starts with the first sample that page actually receives; earlier history is not invented or replayed. Stop the server with **Ctrl+C in its terminal**. In-flight connections have a three-second deadline; shutdown then stops/joins the simulation worker and joins the snapshot collector. Restart the executable to repeat the experiment. No reset endpoint is supplied.

## Algorithm and code-reading order

Read [MODEL.md](MODEL.md), including its five-tick table, then:

1. **`src/demo.rs`** supplies initial agents and scripted proposals. No autonomous decisions are hidden in the UI.
2. **`src/engine.rs`, `World::step`** validates all proposals against the starting grid, counts destination claims, and turns occupied/conflicting claims into waits. It checks counter/ID capacity before mutation, copies the starting grid to the reusable next buffer, applies accepted actions once, and swaps buffers. Creation IDs follow creators' starting squares in row-major order.
3. **`src/runner.rs`** owns the world on a standard thread. It applies bounded commands between ticks and publishes independent snapshots at sampling/control/completion boundaries. **`src/bin/headless.rs`** runs this worker without observation.
4. **`src/web.rs`** collects snapshots into one cache and serves exact JSON plus embedded assets. **`src/bin/web.rs`** parses launch options, binds loopback, and handles Ctrl+C. **`web/app.js`** polls and sends controls; **`web/display.js`** draws Canvas 2D grids/plots and readouts. Neither JavaScript file computes transitions.
5. **`tests/engine.rs`, `tests/runner.rs`, `tests/headless.rs`, `tests/web.rs`** check the engine, thread boundaries, real executable, and real HTTP API. **`tests/browser/`** drives the actual Rust process and frontend with Playwright.

For example, A and B both claim `(1,2)`, so both wait. A's removed square was occupied at the beginning, so C enters only on the following tick. A child exists in the next grid and cannot act during its creation tick. These rules are independent of how frequently the page reads or draws.

A Rust `struct` groups fields, like a Python dataclass or C struct. `Vec<Option<Agent>>` is a contiguous array of occupied (`Some`) or empty (`None`) squares, indexed as `y * width + x`. `&World` borrows for reading; `&mut World` permits exclusive mutation. Moving the world into the worker transfers ownership, avoiding shared mutable world state.

A channel is a bounded mailbox between threads. A control receipt means the worker actually processed a request; sending it into the mailbox alone is not success. `Arc` gives shared ownership of the tiny adapter cache; `Mutex` permits a short exclusive copy/update. HTTP work uses Tokio tasks so stalled sockets do not stall other requests. **Hyper** parses HTTP; **http-body-util** bounds bodies; **Serde/serde_json** serialize/validate JSON; **hyper-util** adapts Tokio sockets. These optional dependencies stay outside the engine. The simulation does not become async because the server is async. HTML/CSS/JavaScript assets are embedded at compile time and served locally without a frontend framework, bundler, or CDN.

## Observation and the small API

The runner's channel stores one queued snapshot and at most one replaceable pending snapshot. It uses `try_send`, never a blocking snapshot send. The collector owns one cached completed sample; each HTTP handler copies the cache before serialization/network I/O. The engine never receives a request to produce a frame. At most 16 connections are handled concurrently, and the page has at most one snapshot read outstanding. Old samples can be missed, with bounded storage rather than a backlog. Observation changes overhead, not outcomes; no throughput claim is made.

The page keeps at most 128 `(tick, count)` points. It labels observed ticks and sampling gaps and does not draw lines across gaps. Accepted-event totals come from transitions, not differences between frames. The plot is neither an event log nor a replay.

| Request | Meaning |
|---|---|
| `GET /` and the three explicit asset paths | Locally embedded page, CSS, and JS; no generic file serving |
| `GET /api/snapshot` | Latest cached state; never advances the world |
| `POST /api/control` with `{"command":"pause"}`, `"resume"`, or `"step"` | Queue one validated command and wait briefly for its applied receipt |

Snapshot IDs, values, ticks, counts, and event totals are decimal **strings**, preserving `u64`/`i64` values above JavaScript's safe-number range and at signed limits. Grid dimensions are small numbers. JavaScript uses `BigInt` for tick comparisons and converts only approximate plot coordinates to `Number`. Exact readouts retain their strings.

A successful control response is `{"applied":true,"tick":"1","status":"paused",...}` for an already applied action. Single-step applies exactly once while paused. A non-applicable/completed/stopped request returns 409; a full 16-command queue returns 503. Unknown fields, duplicate fields, invalid commands/JSON, oversized bodies, and unsupported content types are rejected. If a receipt times out (504), or the response is lost, its outcome is unknown: inspect the latest tick before another command. The browser **never automatically retries a control**. A rejected completion race cannot report a successful sixth step.

After an applied receipt, the page waits for a snapshot read started after the command response and at or beyond its applied tick. Controls then follow that snapshot's current status: another Pause or Resume can supersede the acknowledged status without advancing the tick. Reads started before or during the command remain ignored.

The server binds IPv4 loopback and validates Host plus Origin. Browser controls require the exact same origin and `application/json`. Local API clients must send `Origin: http://127.0.0.1:PORT` (or the matching localhost origin). Foreign origins/fetch sites are rejected; no wildcard CORS is enabled. Request bodies are capped at 128 bytes, headers at 32 with an 8 KiB parser buffer, and each connection at three seconds including slow responses. Keep-alive is disabled. Excess connections are closed before another parser is allocated. A command queue cannot prevent explicit worker shutdown. This is a local single-user tool, without accounts or remote access.

## Verification

Local equivalents of `.github/workflows/ci.yml` (Node **24.19.0** and Playwright **1.63.0** are development/test tools only):

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --no-default-features -- -D warnings
cargo clippy --locked --all-targets --features web -- -D warnings
cargo test --locked --no-default-features
cargo test --locked --release --no-default-features
cargo test --locked --features web
cargo test --locked --doc --no-default-features
cargo test --locked --doc --features web
cargo build --locked --features web --bin web
npm ci
npm run check
npx playwright install --with-deps chromium
npm run test:e2e
cargo install cargo-audit --version 0.22.2 --locked
cargo audit --file Cargo.lock
npm audit --audit-level=low
git diff --check
```

Headless debug/release tests invoke the real CLI at 0/5/8 ticks and need no HTTP dependency or browser. The five-state oracle and all 120 proposal permutations remain. The bounded exhaustive test covers three shapes, 64 occupancy masks, and 36 two-tick programs, checking order, identity, occupancy, count, accepted events, allocation, and atomic rejection. Explicit tests cover full grids, both signed boundaries, last usable ID, and tick/event-counter exhaustion. Private counters are changed only inside engine tests; no production setters or randomness were added.

Runner tests cover disabled, normal, saturated/unread, and disconnected observation, completion delivery, receipts, and bounded overload. Test-side joins and HTTP shutdown have deadlock guards; sleeps are not proof of completion. API tests use a real loopback server, including an incomplete request that remains stalled while the worker finishes. A separate adapter test uses the completion signal with no HTTP and with a retained old response, then checks the durable final cache. A 64-byte in-memory HTTP transport also forces response backpressure: the actual Hyper snapshot handler remains blocked on output while the worker completes and the collector publishes its final cache. Releasing the reader during failed-test cleanup prevents a lock regression from hanging the test process.

Each Playwright test starts a **fresh Rust process** on an OS-assigned port; a fresh browser context alone is insufficient. The suite checks every scripted state, actual canvas pixels and clicks, moved/removed selections, controls/completion races, final totals, refresh/close/reconnect, no connected page, network errors and lost command replies, sampled plot gaps, and a narrow window. Supplemental precision tests exercise synthetic extreme values in the real display functions; these are distinct from the real-fixture E2E acceptance tests. The two Node tests check bounded history and exact tick/value handling.

On Windows, the test fixture starts the real server in a private terminal using **node-pty 1.1.0** and writes Ctrl+C to its input. Node's [`child.kill('SIGINT')`](https://nodejs.org/api/child_process.html#subprocesskillsignal) forcibly terminates a Windows process, so it cannot test the server's graceful console shutdown. Linux keeps its ordinary process/SIGINT path. Both paths still require exit code 0 within the shutdown deadline; forced cleanup fails the test. Two focused regressions stop paused and running servers, then verify that their listeners are closed. This is test-only plumbing; the application, engine, and HTTP API are unchanged.

`node-pty` is a development dependency installed by `npm ci`. It uses Windows ConPTY (Windows 10 version 1809 or newer); its packaged Windows x64 binaries worked without an additional build here. On platforms without a packaged binary, npm builds its native addon and needs Python, make, and a C++ compiler; see [the upstream prerequisites](https://github.com/microsoft/node-pty#dependencies). The application itself still needs neither Node nor a terminal library.

**Verified evidence:** [CI run 34013927145](https://github.com/ppericard/virtual-life/actions/runs/34013927145) passed all five jobs for web replacement revision `9d8341665006cbb6425caa23cad4fb1bad37d3ed`. Logs confirm GitHub tested temporary PR merge commit `056dcff8c651f03a6191e87d4cab6d8a56571f5e`, whose tree matches that revision. Linux and Windows each passed 29 headless tests in debug and release, 35 web-enabled tests, and the ordinary server build. Linux quality checks and all **7 Chromium E2E scenarios** passed. Both dependency audits reported no known vulnerabilities; RustSec loaded 1,239 advisories. These are inspected job/log results, not a workflow-file claim.

After those parity results, the native frontend was removed and the deterministic HTTP backpressure regression added. The resulting local Linux suite passes **29 headless / 36 web-enabled tests**. Current PR-revision CI run links, tested head/temporary-merge SHAs, and job results are maintained in [PR #4](https://github.com/ppericard/virtual-life/pull/4), so verification evidence does not require a self-referential commit SHA in this document. Both the engine's production transition implementation and the browser assets are unchanged by native removal. The removed native completion/control-race regression is replaced by worker receipts, real API completion rejection, and the concurrent browser-control race scenario.

**Visual inspection:** actual Chromium screenshots from the passing replacement run were downloaded and inspected: [paused](docs/screenshots/paused.png), [completed](docs/screenshots/completed.png), [sampled gaps](docs/screenshots/sampled-gaps.png), and [390-pixel window](docs/screenshots/narrow-paused.png). Grid coordinates/occupancy/ID labels, selection outline/readout, event totals, finite completion/disabled controls, disconnected plot segments, and narrow layout were readable and matched the fixture. No pixel-regression baseline was automatically accepted. These images are source-controlled for durable review; this inspection is separate from Pierre's acceptance.

The initial implementation's local Chromium downloads failed; its browser evidence above came from GitHub's Linux job. **8 September 2026 — local Windows verification:** the two new shutdown regressions failed with the original helper (`signal: SIGINT` instead of exit code 0), then passed with terminal Ctrl+C. `npm run test:e2e` passed all **10 scenarios**, including the eight original scenarios, with no retries or skips (Rust 1.95.0, Node 24.11.1, Playwright 1.63.0, Chromium 153.0.8010.12). CI continues to pin Node 24.19.0. These are local results, not a claim that the updated CI workflow has run. Local Rust builds use ordinary profiles, with no retired egui workaround. Firefox/WebKit, macOS, manual keyboard/assistive-technology review, and throughput remain unverified. Documentation tests ran successfully but there are currently no executable doc examples.

CI defines Quality (Linux), Tests (Linux), Tests (Windows), Browser (Linux Chromium), Browser (Windows Chromium), and Dependency audit jobs, with timeouts and superseded-run cancellation. The Windows browser job guards against recurrence of the shutdown-helper bug. Actions use verified full commit SHAs; permissions are `contents: read`, without privileged PR triggers. Each browser job keeps its HTML report, known-state screenshots, and failure traces/screenshots as a separately named 14-day artifact. There is no arbitrary coverage/performance gate. No browser engines beyond Chromium are covered.

The old 21/22-test report and native GUI build workaround remain historical evidence in [PR #2](https://github.com/ppericard/virtual-life/pull/2). That window was never visually verified. Retiring the native frontend retires its workaround; it does not establish that the former build problem was fixed. Visual inspection of real browser screenshots is separate from automated tests, source review, and Pierre's acceptance.

## Limits

Agents still have only an ID and integer value. There are no autonomous rules, production randomness, automatic expiry, resource classes, property framework, world editing, persistence, replay, exports, WebAssembly engine, cloud deployment, or analysis framework. Multiple occupancy, variable sizes, and continuous space remain future possibilities. The linear actor lookup is O(grid squares × proposals), plus O(grid squares) passes: a simplicity trade-off for this fixture, not a scalable performance claim. Measure before optimizing.

Browser refresh loses page-local observation history and selection. Multiple pages share one experiment and controls; there is no multi-user coordination. Abrupt process termination loses the run; Ctrl+C is the supported clean shutdown. Firefox, WebKit, macOS, throughput, and assistive-technology behavior need separate verification.

## Continuity and contribution

[AGENTS.md](AGENTS.md) holds the short contributor guide; [MODEL.md](MODEL.md) states intended behavior. Code and execution show actual behavior. Investigate disagreement. Pierre controls direction; the lead reviews bounded tasks. These maintained documents supersede planning attachments; Git and task/PR discussions hold routine history. **`master` is now the active integration and default branch.** Use issues, task branches, and PRs into master; do not continue new work against the retired restart integration branch.

Historical tips were verified before migration:

| Archive | Preserved commit |
|---|---|
| [archive/master-before-rust](https://github.com/ppericard/virtual-life/tree/archive/master-before-rust) | `4caa8fc5d4ce0672c579a6fb4dcd5c8929d375c3` |
| [archive/comprehensive-improvements](https://github.com/ppericard/virtual-life/tree/archive/comprehensive-improvements) | `31ab057b57fcdaa2925483b0eea7328685578c79` |
| [archive/original-all-manual](https://github.com/ppericard/virtual-life/tree/archive/original-all-manual) | `98225432ca878a064045d3505ea6f3b6e1180488` |

Original historical branch names remain available so existing links keep working. `restart/rust-v0.1` and `work/task-01-live-grid` remain as prior-work references, not competing active development lines. The one-time master migration retains old master and the Task 01 head as parents, preserving both histories without a force-push. It removes inherited `.py` files and their obsolete ignore file from the active tree. [LICENSE](LICENSE) is unchanged. Repeated activation, same-tick newborn activation, and acting after expiry in the original Python were bugs; display coupling was unfinished implementation. Do not restore them as model rules.
