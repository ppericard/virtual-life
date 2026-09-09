# Development reference

Start with [README](../README.md) for the project overview and quick start. This reference contains the launch options, code-reading guide, API details and complete verification commands. Run commands from the repository root. Intended model rules remain in [MODEL.md](../MODEL.md); contributor guidance is in [AGENTS.md](../AGENTS.md).

## Running options

Headless builds use only the standard-library engine and runner:

```sh
cargo run --locked --no-default-features --bin headless -- --ticks 0
cargo run --locked --no-default-features --bin headless -- --ticks 5
cargo run --locked --no-default-features --bin headless -- --ticks 8
```

The default is five ticks. Tick 0 contains three agents. At tick 5, IDs 2 and 5 have value 21 at `(2,2)` and `(1,2)`, respectively; accepted totals are two moves, two creations, three removals and two value changes. Headless requests beyond five run all-wait ticks: only the tick advances. Invalid or overflowing arguments fail with a message.

Browser launch options include:

```sh
cargo run --locked --features web --bin web -- --running --tick-ms 1000 --sample-every 2
```

`--port N` selects an IPv4 loopback port (`0` chooses a free one). `--sample-every N` must be positive; its default is 1. `--tick-ms N` accepts 0–60000, defaults to 750, and sets a minimum interval when running. Zero removes pacing and may leave only initial/final samples visible. Pacing is a speed ceiling, not a real-time guarantee. Both binaries support `--help`.

**Lifecycle:** the Rust process owns one experiment. Refreshing, closing, hiding or disconnecting a page neither pauses, resets nor stops it. A new page reads the current/final cache; its plot starts with the first sample actually received. Stop the server with **Ctrl+C in its terminal**. In-flight connections have a three-second deadline; shutdown then stops/joins the worker and collector. To repeat the fixture, restart the executable; retained pages recognise the new run automatically. There is no reset endpoint or persistence.

A different run clears page-local history, selection and outstanding control state, with a visible new-experiment notice. The page follows the new run's actual status without resuming, stepping or retrying old commands. Reconnecting to the same run retains its history. Reload the page after changing browser assets to load the newly compiled version.

## Algorithm and code-reading order

Start with [MODEL.md](../MODEL.md) and its worked five-tick example, then read:

1. [src/demo.rs](../src/demo.rs): initial agents and scripted proposals, separate from transition logic.
2. [src/engine.rs](../src/engine.rs), `World::step`: validate against the starting grid, resolve occupied/conflicting claims to waits, check counter/ID capacity, build the next grid and swap buffers. Creation IDs follow creators' starting squares in row-major order.
3. [src/runner.rs](../src/runner.rs): a standard thread owns the world, applies commands between ticks and publishes sampled snapshots. [src/bin/headless.rs](../src/bin/headless.rs) runs it without observation.
4. [src/web.rs](../src/web.rs) and [src/bin/web.rs](../src/bin/web.rs): cache snapshots, serve embedded assets/JSON, bind loopback and handle shutdown. [web/app.js](../web/app.js) polls and sends controls; [web/display.js](../web/display.js) draws and inspects. JavaScript never computes transitions.
5. [tests/engine.rs](../tests/engine.rs), [tests/runner.rs](../tests/runner.rs), [tests/headless.rs](../tests/headless.rs), [tests/web.rs](../tests/web.rs) and [tests/browser/](../tests/browser/): rules, boundaries, real executables/API, and real-browser behaviour.

For example, A and B both claim `(1,2)`, so both wait. A removed square was occupied at the start, so another agent enters only on the following tick. A child exists in the next grid and cannot act during its creation tick. These rules do not depend on how often the page draws.

A Rust `struct` groups fields, like a Python dataclass or C struct. `Vec<Option<Agent>>` is a contiguous array of occupied (`Some`) or empty (`None`) squares, indexed as `y * width + x`. `&World` borrows for reading; `&mut World` permits exclusive mutation. Moving the world into the worker transfers ownership, avoiding shared mutable world state. Swapping current/next buffers avoids copying the finished grid back.

A channel is a bounded mailbox between threads. A receipt means the worker processed a command, not merely received it in the queue. `Arc` shares ownership of the adapter cache; `Mutex` protects a short copy/update. Hyper parses HTTP, http-body-util bounds bodies, Serde/serde_json handle JSON, and hyper-util adapts Tokio sockets. Tokio handles server I/O; the simulation remains a sequential ordinary thread. These dependencies are optional and outside the engine. Assets are embedded at compile time, without a frontend framework, bundler or CDN.

## Observation and the small API

The runner retains one queued snapshot and at most one replaceable pending snapshot. It uses `try_send`, not a blocking send, at sampling/control/completion boundaries. A collector retains the latest cache independently of any browser. HTTP handlers copy that cache before serialization or network I/O; reads never ask the engine to advance or produce a frame. There are at most 16 concurrent connections and one outstanding browser snapshot read.

The page keeps at most 128 `(tick, count)` points, labels observed ticks and sampling gaps, and does not join lines across gaps. Accepted-event totals come from transitions, not frame differences. Observation changes overhead, not fixed-tick outcomes; neither a throughput guarantee nor a complete event log/replay is implied.

| Request | Meaning |
|---|---|
| `GET /`, `/app.js`, `/display.js`, `/style.css` | Embedded local assets; no generic file serving |
| `GET /api/snapshot` | Latest cached state; never advances the world |
| `POST /api/control` | JSON `{"command":"pause"}`, `{"command":"resume"}` or `{"command":"step"}`; wait briefly for an applied receipt |

Snapshot IDs, values, ticks, counts and event totals are decimal **strings**, preserving every bit of `u64`/`i64`, including values outside JavaScript's safe-number range. Grid dimensions are small numbers. JavaScript uses `BigInt` for tick comparisons; only approximate plot coordinates become `Number`.

An applied control returns `{"applied":true,"tick":"1","status":"paused",...}`. Single-step advances exactly once while paused, never beyond completion. A non-applicable/completed/stopped request returns 409; a full 16-command queue returns 503. Unknown/duplicate fields, malformed JSON, invalid commands, oversized bodies and unsupported content types are rejected. A 504 receipt timeout or lost response means the outcome is unknown: inspect the current tick before another command. **The browser never automatically retries controls.**

Within a run, after an applied receipt the page waits for a snapshot read started after the response and at or beyond its applied tick. Controls follow that snapshot's current status, since another command can supersede the acknowledged status at the same tick. Reads spanning a command are excluded from receipt reconciliation. A run change also retires pending command callbacks, so late replies or errors cannot alter the new page state.

Every allowed HTTP response carries `X-VirtualLife-Run`, an opaque 128-bit identifier generated once for the adapter instance using the optional `getrandom` library. This avoids clock/PID reuse without changing engine state, simulation randomness or the JSON schema. Failure to obtain OS randomness fails startup. The identifier is not authentication or a persistence/replay key.

Browser controls echo that header as a precondition: a mismatched or repeated header returns 409 before queueing a command. Existing local API clients may omit it for compatibility; clients needing restart safety should echo the most recently observed identifier. A tick decrease is not used to detect restarts.

The server binds IPv4 loopback and validates Host/Origin. Controls require the matching same origin and `application/json`; local API clients must send `Origin: http://127.0.0.1:PORT` (or the matching localhost origin). Foreign origins/fetch sites are rejected; there is no wildcard CORS. Bodies are capped at 128 bytes, headers at 32 with an 8 KiB parser buffer, and connections at three seconds including slow responses. Keep-alive is disabled; excess connections close before allocating another parser. A full command queue cannot prevent explicit worker shutdown. This is a local single-user tool, not an authenticated remote service.

## Verification

The existing [CI workflow](../.github/workflows/ci.yml) runs on PRs into `master` and pushes to `master`. Node **24.19.0**, Playwright **1.63.0**, and node-pty **1.1.0** are development/test tools, not application services. Use the following local equivalents for affected code and supported modes:

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

Tests cover the worked states, proposal order, generated short sequences, identity/occupancy/count invariants, atomic rejection and integer exhaustion; runner tests exercise controls and disabled, saturated or disconnected observation. Real API and deterministic backpressure tests check that slow/absent readers do not prevent completion or final-cache publication. Cleanup uses bounded deadlock guards, not sleeps as proof of correctness.

Each Playwright scenario starts a **fresh Rust process**, not just a fresh browser context. Tests cover actual grid pixels/clicks, inspection, controls/completion races, same-tick superseded receipts, refresh/reconnect, absent pages, network errors, lost replies, gaps, narrow layout and graceful shutdown. Restart tests replace the real process at the same address and exercise retained history/receipts, equal ticks, same-run reconnect and delayed old commands/replies. Synthetic extreme-value display checks are supplemental, not substitutes for the real-fixture tests. Node tests check bounded history and exact values. Documentation tests are invoked, but there are currently no executable doc examples.

On Windows the fixture uses node-pty's private terminal to send real Ctrl+C, rather than Node's forceful `child.kill('SIGINT')`. Linux uses its ordinary process/SIGINT path. Both must exit cleanly within the guard; forced cleanup fails. node-pty requires Windows 10 version 1809 or newer. Where no packaged native binary is available, `npm ci` needs native build prerequisites; see [node-pty's prerequisites](https://github.com/microsoft/node-pty#dependencies). These requirements apply to testing, not running VirtualLife.

### CI and evidence

The workflow covers Linux quality checks, Linux/Windows Rust tests and builds, Linux/Windows Chromium, and dependency auditing. It uses read-only permissions, full-SHA action pins, timeouts, superseded-run cancellation and separately named 14-day browser artifacts. It has no arbitrary coverage or performance gate.

Keep tested commit/run links and execution results in the relevant PR. CI success, source review, visual inspection and Pierre's acceptance are distinct. The preserved [paused](../docs/screenshots/paused.png), [completed](../docs/screenshots/completed.png), [sampled gaps](../docs/screenshots/sampled-gaps.png) and [narrow layout](../docs/screenshots/narrow-paused.png) screenshots are historical browser-parity evidence, not automatically updated visual baselines.

Firefox/WebKit, macOS, manual keyboard/assistive-technology behaviour and throughput remain unverified.

## Implementation limits

Agents have only an ID and integer value. There are no autonomous rules, simulation randomness, automatic expiry, resource classes, generic property framework, world editing, persistence, replay, exports, WebAssembly engine, cloud deployment or analysis framework. Multiple occupancy, variable sizes and continuous space remain possible future decisions, not implemented abstractions.

Linear actor lookup costs O(grid squares × proposals), plus O(grid squares) passes. This is a tiny-fixture simplicity trade-off, not a scaling claim; measure a real experimental workload before optimising. Page history and selection are lost on refresh, multiple pages share one experiment/control surface, and abrupt process termination loses the run. A new run deliberately discards the old page-local observations; they are not saved.

## Historical references

The native viewer and browser migration are recorded in [PR #2](https://github.com/ppericard/virtual-life/pull/2) and [PR #4](https://github.com/ppericard/virtual-life/pull/4). Detailed execution history and preserved branch tips remain in the [pre-cleanup README](https://github.com/ppericard/virtual-life/blob/9ca715a77965d9a8d4dee4a79f2a0d5a3fef5d4c/README.md#continuity-and-contribution) and Git history, not a rolling log here.

Historical branches are references, not development bases. Retiring the native viewer did not establish that its old build problem was fixed. Repeated activation, same-tick newborn activation and acting after expiry in the original Python were bugs; display coupling was unfinished implementation. Do not restore them as model rules.
