use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use virtual_life::{
    demo,
    engine::World,
    runner::{Command, Config, SNAPSHOT_CAPACITY, Snapshot, Status, Worker as RawWorker},
};

const GUARD: Duration = Duration::from_secs(10);

// Even an intentionally broken worker must fail within GUARD during unwinding.
// Keep blocking joins off the test thread; the Rust test process owns any failed
// detached thread and will exit. This is a deadlock guard, not timing evidence.
struct Worker(Option<RawWorker>);
impl std::ops::Deref for Worker {
    type Target = RawWorker;
    fn deref(&self) -> &RawWorker {
        self.0.as_ref().unwrap()
    }
}
fn guarded_join(
    work: impl FnOnce() -> Result<World, String> + Send + 'static,
) -> Result<World, String> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let _ = send.send(work());
    });
    receive
        .recv_timeout(GUARD)
        .expect("worker join exceeded deadlock guard")
}
impl Worker {
    fn spawn(config: Config, sender: Option<mpsc::SyncSender<Snapshot>>) -> Result<Self, String> {
        RawWorker::spawn(config, sender).map(|worker| Self(Some(worker)))
    }
    fn join(mut self) -> Result<World, String> {
        let worker = self.0.take().unwrap();
        guarded_join(move || worker.join())
    }
    fn stop(mut self) -> Result<World, String> {
        let worker = self.0.take().unwrap();
        guarded_join(move || worker.stop())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(worker) = self.0.take() {
            let (send, receive) = mpsc::channel();
            thread::spawn(move || {
                drop(worker);
                let _ = send.send(());
            });
            let result = receive.recv_timeout(GUARD);
            if !thread::panicking() {
                result.expect("worker drop exceeded deadlock guard");
            }
        }
    }
}

fn fast() -> Config {
    Config {
        ticks: 100,
        sample_every: 3,
        tick_interval: Duration::ZERO,
        start_paused: false,
        experiment: None,
    }
}

fn receive_until(receiver: &Receiver<Snapshot>, predicate: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let deadline = Instant::now() + GUARD;
    loop {
        let sample = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if predicate(&sample) {
            return sample;
        }
    }
}

fn stop_with_guard(worker: Worker) -> World {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(worker.stop()).unwrap());
    receiver
        .recv_timeout(GUARD)
        .expect("shutdown must not wait for the viewer")
        .unwrap()
}

#[test]
fn disabled_normal_and_saturated_unread_observation_have_identical_outcomes() {
    let unobserved = Worker::spawn(fast(), None).unwrap().join().unwrap();
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(fast(), Some(sender)).unwrap();
    let last = receive_until(&receiver, |s| s.status == Status::Completed);
    assert_eq!(last.tick, 100);
    let observed = worker.join().unwrap();
    let (sender, unread) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(fast(), Some(sender)).unwrap();
    // Never read the full buffer. This notification proves all ticks finished.
    let completion = worker.wait_for_completion(GUARD);
    if completion.is_err() {
        // Release a wrongly blocking sender before a failed assertion unwinds
        // through Worker::drop, so a regression fails instead of hanging tests.
        drop(unread);
    }
    completion.unwrap();
    let saturated = stop_with_guard(worker);
    assert_eq!(unobserved, observed);
    assert_eq!(unobserved, saturated);
}

#[test]
fn a_saturated_completed_run_eventually_delivers_its_actual_final_snapshot() {
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(
        Config {
            sample_every: 99,
            ..fast()
        },
        Some(sender),
    )
    .unwrap();
    worker.wait_for_completion(GUARD).unwrap();
    assert_eq!(receiver.recv_timeout(GUARD).unwrap().tick, 0);
    let final_sample = receiver.recv_timeout(GUARD).unwrap();
    assert_eq!(final_sample.tick, 100);
    assert_eq!(final_sample.status, Status::Completed);
    let world = worker.join().unwrap();
    assert_eq!(final_sample.cells, world.cells());
    assert_eq!(final_sample.totals, world.totals());
    assert_eq!(final_sample.count, 2);
}

#[test]
fn pause_resume_step_and_shutdown_are_applied_between_ticks() {
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(
        Config {
            tick_interval: Duration::from_secs(3600),
            sample_every: 99,
            ..Config::default()
        },
        Some(sender),
    )
    .unwrap();
    assert_eq!(receiver.recv_timeout(GUARD).unwrap().status, Status::Paused);
    worker.command(Command::Step).unwrap();
    let step = receive_until(&receiver, |s| s.tick == 1);
    assert_eq!(step.status, Status::Paused);
    worker.command(Command::Pause).unwrap();
    let still_paused = receiver.recv_timeout(GUARD).unwrap();
    assert_eq!(still_paused.tick, 1);
    worker.command(Command::Resume).unwrap();
    assert_eq!(
        receive_until(&receiver, |s| s.status == Status::Running).tick,
        1
    );
    worker.command(Command::Step).unwrap(); // Ignored while running.
    worker.command(Command::Pause).unwrap();
    assert_eq!(
        receive_until(&receiver, |s| s.status == Status::Paused).tick,
        1
    );
    worker.command(Command::Step).unwrap();
    assert_eq!(
        receive_until(&receiver, |s| s.tick == 2).status,
        Status::Paused
    );
    worker.command(Command::Resume).unwrap();
    receive_until(&receiver, |s| s.status == Status::Running);
    // Shutdown interrupts the hour-long pacing wait, without a throughput claim.
    assert_eq!(stop_with_guard(worker).tick(), 2);
}

#[test]
fn pending_paused_snapshot_is_refreshed_without_needing_another_tick() {
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(Config::default(), Some(sender)).unwrap();
    // Tick zero fills the queue; later paused samples replace the one pending slot.
    worker.command(Command::Step).unwrap();
    worker.command(Command::Step).unwrap();
    let latest = receive_until(&receiver, |s| s.tick == 2);
    assert_eq!(latest.status, Status::Paused);
    assert_eq!(latest.count, 3);
    assert_eq!(stop_with_guard(worker).tick(), 2);
}

#[test]
fn a_paused_worker_can_step_to_the_exact_finite_end() {
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(Config::default(), Some(sender)).unwrap();
    receiver.recv_timeout(GUARD).unwrap();
    for tick in 1..=5 {
        worker.command(Command::Step).unwrap();
        let sample = receive_until(&receiver, |s| s.tick == tick);
        assert_eq!(
            sample.status,
            if tick == 5 {
                Status::Completed
            } else {
                Status::Paused
            }
        );
    }
    worker.wait_for_completion(GUARD).unwrap();
    assert_eq!(worker.join().unwrap().tick(), 5);
}

#[test]
fn sampling_cadence_is_respected_and_received_snapshots_are_independent_copies() {
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(fast(), Some(sender)).unwrap();
    loop {
        let mut sample = receiver.recv_timeout(GUARD).unwrap();
        assert!(sample.tick % 3 == 0 || sample.status == Status::Completed);
        sample.cells.clear(); // This local copy cannot edit the worker's world.
        if sample.status == Status::Completed {
            break;
        }
    }
    assert_eq!(worker.join().unwrap().count(), 2);
}

#[test]
fn disconnected_viewer_and_unread_completion_signal_do_not_block_the_run() {
    let (sender, receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    drop(receiver);
    let worker = Worker::spawn(fast(), Some(sender)).unwrap();
    assert_eq!(
        worker.join().unwrap(),
        Worker::spawn(fast(), None).unwrap().join().unwrap()
    );
}

#[test]
fn dropping_a_worker_stops_and_joins_even_when_paused_with_an_unread_buffer() {
    let (sender, _receiver) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
    let worker = Worker::spawn(Config::default(), Some(sender)).unwrap();
    let (done, finished) = mpsc::channel();
    thread::spawn(move || {
        drop(worker);
        done.send(()).unwrap();
    });
    finished.recv_timeout(GUARD).unwrap();
}

#[test]
fn zero_ticks_completes_without_activation_and_invalid_config_is_rejected() {
    assert_eq!(
        Worker::spawn(
            Config {
                ticks: 0,
                ..Config::default()
            },
            None
        )
        .unwrap()
        .join()
        .unwrap(),
        demo::initial_world()
    );
    assert!(
        Worker::spawn(
            Config {
                sample_every: 0,
                ..Config::default()
            },
            None
        )
        .is_err()
    );
    assert!(
        Worker::spawn(
            Config {
                tick_interval: Duration::MAX,
                ..Config::default()
            },
            None
        )
        .is_err()
    );
}

#[test]
fn receipts_distinguish_applied_steps_from_running_and_completed_rejections() {
    let worker = Worker::spawn(
        Config {
            tick_interval: Duration::from_secs(3600),
            ..Config::default()
        },
        None,
    )
    .unwrap();
    let controls = worker.controls();
    let receipt = controls
        .request(Command::Step)
        .unwrap()
        .recv_timeout(GUARD)
        .unwrap();
    assert!(receipt.applied);
    assert_eq!(receipt.tick, 1);
    assert_eq!(receipt.status, Status::Paused);
    let resumed = controls
        .request(Command::Resume)
        .unwrap()
        .recv_timeout(GUARD)
        .unwrap();
    assert!(resumed.applied);
    assert_eq!(resumed.tick, 1);
    let rejected = controls
        .request(Command::Step)
        .unwrap()
        .recv_timeout(GUARD)
        .unwrap();
    assert!(!rejected.applied);
    assert_eq!(rejected.tick, 1);
    controls
        .request(Command::Pause)
        .unwrap()
        .recv_timeout(GUARD)
        .unwrap();
    for tick in 2..=5 {
        let receipt = controls
            .request(Command::Step)
            .unwrap()
            .recv_timeout(GUARD)
            .unwrap();
        assert!(receipt.applied);
        assert_eq!(receipt.tick, tick);
    }
    // Completion can race a queued request: rejection OR closed receiver, never a sixth step.
    if let Ok(receiver) = controls.request(Command::Step)
        && let Ok(receipt) = receiver.recv_timeout(GUARD)
    {
        assert!(!receipt.applied);
    }
    assert_eq!(worker.join().unwrap().tick(), 5);
}
