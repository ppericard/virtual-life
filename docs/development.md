# Development reference

Start with [README](../README.md) for the project overview and quick start. This reference contains the launch options, code-reading guide, API details and complete verification commands. Run commands from the repository root. Intended model rules remain in [MODEL.md](../MODEL.md); contributor guidance is in [AGENTS.md](../AGENTS.md).

## Running options

Headless builds use only the standard-library engine, seeded policy and runner:

```sh
cargo run --locked --no-default-features --bin headless -- --ticks 0
cargo run --locked --no-default-features --bin headless -- --ticks 5
cargo run --locked --no-default-features --bin headless -- --ticks 8
```

For compatibility, the default mode is `demo`, with five ticks. Tick 0 contains three agents. At tick 5, IDs 2 and 5 have value 21 at `(2,2)` and `(1,2)`, respectively; accepted totals are two moves, two creations, three removals and two value changes. Headless requests beyond five run all-wait ticks: only the tick advances. Invalid or overflowing arguments fail with a message.

Browser launch options include:

```sh
cargo run --locked --features web --bin web -- --running --tick-ms 1000 --sample-every 2
```

`--port N` selects an IPv4 loopback port (`0` chooses a free one). `--sample-every N` must be positive; its default is 1. `--tick-ms N` accepts 0–60000, defaults to 100 in autonomous mode or 750 in demo mode, and sets a minimum interval when running. Zero removes pacing and may leave only initial/final samples visible. Pacing is a speed ceiling, not a real-time guarantee. Both binaries support `--help`.

### Autonomous configuration and reproduction

```sh
cargo run --locked --features web --bin web -- --mode autonomous --seed 1 --ticks 500
cargo run --locked --no-default-features --bin headless -- --mode autonomous --seed 1 --ticks 500
cargo run --locked --features web --bin web -- --mode autonomous --width 8 --height 6 --occupancy 0.3 --bundles "2,4,1,3;2,2,2,4;4,1,1,4;1,5,2,2" --proportions 1,1,1,1 --integrity 10 --upkeep 1 --move-wear 1 --copy-wear 2 --repair 4 --seed 1 --ticks 80 --tick-ms 100 --sample-every 1
cargo run --locked --no-default-features --bin headless -- --mode autonomous --survival random --width 8 --height 6 --seed 1 --ticks 10
```

Both shells require quoting semicolon-separated bundles. `--bundles` gives wait/move/copy/repair integer weights per bundle in the default `--survival wear-repair` mode. `--survival random` selects the original wait/move/copy/remove behaviour and its earlier default bundles. `--proportions` gives one nonnegative integer ratio per supplied bundle. Defaults are the four bundles and equal ratios in MODEL. One to eight **distinct** bundles are supported; repeated identical tuples combine ratios into the first canonical group. Grid dimensions must each be at least 3 with at most 262,144 squares. Occupancy is a fraction 0–1 with up to six decimal places, not a percentage. Total and per-group initial counts are exact after the floor/largest-remainder allocation described in MODEL. Zero weight actions, zero-ratio groups and empty/full worlds are allowed; all-zero bundle weights or all-zero ratios are errors. Autonomous-only settings in demo mode are rejected so ignored configuration cannot masquerade as a different run.

Wear/repair options are global nonnegative `u32` integers: `--integrity` sets positive maximum, initial and newborn integrity (default 10); `--upkeep` costs 1 each tick; `--move-wear` and `--copy-wear` cost an additional 1 and 2 per attempt; `--repair` restores 4 after upkeep, capped at the maximum. These overrides are rejected in random/demo modes, even if set to zero. Failure occurs before an action when upkeep leaves zero, or before a spatial claim when its extra wear would leave zero. Affordable attempts pay wear even when the destination is rejected. Repair consumes the action opportunity and has no material/energy cost. See MODEL for precise ordering and examples. Zero upkeep and costs permit indefinite persistence; there is no age limit or forced extinction. The random comparison command above retains its reference result: initial groups 4/4/3/3, final groups 2/6/0/1, 28 moves, 7 copies and 12 removals.

`--seed` accepts an unsigned 64-bit integer, including zero. `--ticks` is a finite unsigned 64-bit limit; zero reports the initial state as completed. Autonomous defaults are 32 × 24, 0.3 occupancy, seed 1 and 500 ticks. CLI `--help` lists all options. Output records effective canonical settings, seed, SplitMix64 / VirtualLife sampling v1, crate version and actual initial counts; the browser retains those alongside the final state. Headless output also prints final group counts and every surviving individual's ID, weights and position. Save the command, source commit and Cargo.lock with any result to reproduce it; arbitrary version changes are not covered by a seed guarantee.

In the browser, group letters and colours match grid, legend and plot. The legend labels the effective fourth action and shows initial/current counts; clicking an agent shows its stable ID, actual properties and integrity/maximum in wear-repair mode. Small cells omit letters but keep colour, and inspection/legend retain the identifier. Each group keeps its series even at zero. The plot distinguishes trajectories with colour and dash pattern, labels final group counts, and has accessible text for every retained sample. A full 128-point buffer discards old observations and says so; gaps remain unconnected. Reconnecting to the same experiment preserves page-local history; a new run clears it.

Failure evidence is a separate engine-owned buffer of at most 128 records, recorded on every tick even with observation disabled. It stores ID, resulting tick, starting position, reason, starting integrity, upkeep and action wear; records are ordered by tick then starting square. Cumulative failures/repairs remain exact when older records are discarded. Inspection shows the selected individual's actual retained cause or states that the record was discarded/unavailable; it never infers a cause from frame differences. The failure panel shows the discarded count and recent records. Headless output includes the same bounded evidence and surviving integrity. This is not an unbounded event journal or complete recording.

Both policies share SplitMix64 and `VirtualLife sampling v1` initialization/bounded draws. Record the separate action protocol (`wear-repair v1` or `random v1`). Wear-repair scans starting occupied squares in row-major order, skips draws for individuals unable to survive upkeep, draws one weighted choice for every survivor, and draws one of the eight neighbours for Move/Copy even when unaffordable. Costs and repair use no random draws. Baseline random mode retains every earlier draw. Newborns first draw next tick. Effective settings and protocol appear in both outputs; retain the source commit and Cargo.lock for reproduction.

**Lifecycle:** the Rust process owns one experiment. Refreshing, closing, hiding or disconnecting a page neither pauses, resets nor stops it. A new page reads the current/final cache; its plot starts with the first sample actually received. Stop the server with **Ctrl+C in its terminal**. In-flight connections have a three-second deadline; shutdown then stops/joins the worker and collector. To repeat an experiment, restart the executable; retained pages recognise the new run automatically. There is no reset endpoint or persistence.

A different run clears page-local history, selection and outstanding control state, with a visible new-experiment notice. The page follows the new run's actual status without resuming, stepping or retrying old commands. Reconnecting to the same run retains its history. Reload the page after changing browser assets to load the newly compiled version.

## Algorithm and code-reading order

Start with [MODEL.md](../MODEL.md) and its worked five-tick example, then read:

1. [src/experiment.rs](../src/experiment.rs): exact initial quotas, seeded shuffle and weighted autonomous choices from a read-only starting world. [src/launch.rs](../src/launch.rs) validates shared CLI options; [src/demo.rs](../src/demo.rs) preserves scripted proposals.
2. [src/engine.rs](../src/engine.rs), `World::step`: validate against the starting grid, stage optional integrity costs/repair and exclude failed actors' claims, resolve occupancy/conflicts, check counter/ID capacity, build the next grid and commit bounded failure evidence. Creation IDs follow creators' starting squares in row-major order. `World::with_maintenance` validates integrity and shared maintenance; `World::new` preserves the baseline interface.
3. [src/runner.rs](../src/runner.rs): a standard thread owns the world, applies commands between ticks and publishes sampled snapshots. [src/bin/headless.rs](../src/bin/headless.rs) runs it without observation.
4. [src/web.rs](../src/web.rs) and [src/bin/web.rs](../src/bin/web.rs): cache snapshots, serve embedded assets/JSON, bind loopback and handle shutdown. [web/app.js](../web/app.js) polls and sends controls; [web/display.js](../web/display.js) draws and inspects. JavaScript never computes transitions.
5. [tests/engine.rs](../tests/engine.rs), [tests/maintenance.rs](../tests/maintenance.rs), [tests/experiment.rs](../tests/experiment.rs), [tests/runner.rs](../tests/runner.rs), [tests/headless.rs](../tests/headless.rs), [tests/web.rs](../tests/web.rs) and [tests/browser/](../tests/browser/): rules, boundaries, real executables/API, and real-browser behaviour.

For example, A and B both claim `(1,2)`, so both wait. A removed square was occupied at the start, so another agent enters only on the following tick. A child exists in the next grid and cannot act during its creation tick. These rules do not depend on how often the page draws.

A Rust `struct` groups fields, like a Python dataclass or C struct. `Vec<Option<Agent>>` is a contiguous array of occupied (`Some`) or empty (`None`) squares, indexed as `y * width + x`. `&World` borrows for reading; `&mut World` permits exclusive mutation. Moving the world into the worker transfers ownership, avoiding shared mutable world state. Swapping current/next buffers avoids copying the finished grid back.

A channel is a bounded mailbox between threads. A receipt means the worker processed a command, not merely received it in the queue. `Arc` shares ownership of the adapter cache; `Mutex` protects a short copy/update. Hyper parses HTTP, http-body-util bounds bodies, Serde/serde_json handle JSON, and hyper-util adapts Tokio sockets. Tokio handles server I/O; the simulation remains a sequential ordinary thread. These dependencies are optional and outside the engine. Assets are embedded at compile time, without a frontend framework, bundler or CDN.

## Observation and the small API

The runner retains one queued snapshot and at most one replaceable pending snapshot. It uses `try_send`, not a blocking send, at sampling/control/completion boundaries. A collector retains the latest cache independently of any browser. HTTP handlers copy that cache before serialization or network I/O; reads never ask the engine to advance or produce a frame. There are at most 16 concurrent connections and one outstanding browser snapshot read.

The page keeps at most 128 `(tick, total, per-group counts)` points and shows a compact tick range, sample count and sampling-gap count below the plot. Lines do not join across gaps. Accepted-event totals come from transitions, not frame differences. Observation changes overhead, not fixed-tick outcomes; neither a throughput guarantee nor a complete event log/replay is implied.

| Request | Meaning |
|---|---|
| `GET /`, `/app.js`, `/display.js`, `/style.css` | Embedded local assets; no generic file serving |
| `GET /api/snapshot` | Latest cached state; never advances the world |
| `POST /api/control` | JSON `{"command":"pause"}`, `{"command":"resume"}` or `{"command":"step"}`; wait briefly for an applied receipt |

Snapshot IDs, values, seeds, ticks, counts, proportions and event totals are decimal **strings**, preserving every bit of `u64`/`i64`, including values outside JavaScript's safe-number range. Grid dimensions, group indices, u32 weights and integrity/cost settings are JSON numbers. Autonomous snapshots also include `end_tick`, per-cell weights/group and an `experiment` record with seed, generator, action protocol, survival mode, crate version, occupancy and canonical groups. Wear-repair cells add `integrity`, and `experiment.maintenance` holds the five shared settings. Top-level `failure_history` holds `limit`, decimal-string `discarded` and bounded `records`; `totals` includes decimal-string `repairs` and `failures`. Demo cell records retain ID/value. JavaScript uses `BigInt` for tick comparisons; only approximate plot coordinates become `Number`.

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

Wear/repair checks include exact-zero upkeep/action failure, repair caps and zero restoration, rejected attempts paying wear, unaffordable intents excluded from conflicts, fresh child integrity, draw skipping, coalesced properties, generated positive-integrity/identity invariants, bounded failure eviction and counter exhaustion. Observation checks run both wear-repair and the original random mode with multiple sampling rates, saturated and disconnected viewers, comparing final worlds and failure evidence. Fresh-server browser scenarios inspect recovery, selected failure causes, sparse-sampling evidence, discard labels, restart, narrow layout and long completion; earlier random/demo regressions remain explicit.

Each Playwright scenario starts a **fresh Rust process**, not just a fresh browser context. Tests cover actual grid pixels/clicks, inspection, controls/completion races, same-tick superseded receipts, refresh/reconnect, absent pages, network errors, lost replies, gaps, narrow layout and graceful shutdown. Restart tests replace the real process at the same address and exercise retained history/receipts, equal ticks, same-run reconnect and delayed old commands/replies. Synthetic extreme-value display checks are supplemental, not substitutes for the real-fixture tests. Node tests check bounded history and exact values. Documentation tests are invoked, but there are currently no executable doc examples.

On Windows the fixture uses node-pty's private terminal to send real Ctrl+C, rather than Node's forceful `child.kill('SIGINT')`. Linux uses its ordinary process/SIGINT path. Both must exit cleanly within the guard; forced cleanup fails. node-pty requires Windows 10 version 1809 or newer. Where no packaged native binary is available, `npm ci` needs native build prerequisites; see [node-pty's prerequisites](https://github.com/microsoft/node-pty#dependencies). These requirements apply to testing, not running VirtualLife.

### CI and evidence

The workflow covers Linux quality checks, Linux/Windows Rust tests and builds, Linux/Windows Chromium, and dependency auditing. It uses read-only permissions, full-SHA action pins, timeouts, superseded-run cancellation and separately named 14-day browser artifacts. It has no arbitrary coverage or performance gate.

Keep tested commit/run links and execution results in the relevant PR. CI success, source review, visual inspection and Pierre's acceptance are distinct. The preserved [paused](../docs/screenshots/paused.png), [completed](../docs/screenshots/completed.png), [sampled gaps](../docs/screenshots/sampled-gaps.png) and [narrow layout](../docs/screenshots/narrow-paused.png) screenshots are historical browser-parity evidence, not automatically updated visual baselines.

WebKit, macOS, manual keyboard/assistive-technology behaviour and throughput remain unverified. Firefox is an optional focused check when installed; required CI remains Chromium on Linux and Windows.

## Implementation limits

Agents have stable IDs and fixed weight tuples; wear-repair adds mutable integrity and the scripted fixture uses an integer value. There is no age-based mortality, energy/resource layer, mutation, neighbour-property-dependent choice, generic property framework, world editing, persistence, replay, exports, WebAssembly engine, cloud deployment or analysis framework. Repair currently costs only time/action opportunity. Multiple occupancy, variable sizes and continuous space remain possible future decisions, not implemented abstractions.

Actor lookup uses an ordinary ID-to-starting-square hash map. The map is only queried, never iterated to decide outcomes; the engine performs O(grid squares + proposals) expected work, preserving row-major decisions and creation IDs. Group counting costs O(occupied squares × configured groups), with at most eight groups. This is not a throughput guarantee. Page history and selection are lost on refresh, multiple pages share one experiment/control surface, and abrupt process termination loses the run. A new run deliberately discards the old page-local observations; they are not saved.

## Historical references

The native viewer and browser migration are recorded in [PR #2](https://github.com/ppericard/virtual-life/pull/2) and [PR #4](https://github.com/ppericard/virtual-life/pull/4). Detailed execution history and preserved branch tips remain in the [pre-cleanup README](https://github.com/ppericard/virtual-life/blob/9ca715a77965d9a8d4dee4a79f2a0d5a3fef5d4c/README.md#continuity-and-contribution) and Git history, not a rolling log here.

Historical branches are references, not development bases. Retiring the native viewer did not establish that its old build problem was fixed. Repeated activation, same-tick newborn activation and acting after expiry in the original Python were bugs; display coupling was unfinished implementation. Do not restore them as model rules.
