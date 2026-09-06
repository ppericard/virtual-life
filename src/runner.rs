//! Thread ownership, controls, pacing, and lossy observation; not model rules.
use std::sync::mpsc::{
    self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError, TrySendError,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::{
    demo,
    engine::{Agent, Events, World},
};

pub const SNAPSHOT_CAPACITY: usize = 1;
const RETRY_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    Paused,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub width: usize,
    pub height: usize,
    pub tick: u64,
    pub count: usize,
    pub totals: Events,
    pub cells: Vec<Option<Agent>>,
    pub status: Status,
}

impl Snapshot {
    fn capture(world: &World, status: Status) -> Self {
        Self {
            width: world.width(),
            height: world.height(),
            tick: world.tick(),
            count: world.count(),
            totals: world.totals(),
            cells: world.cells().to_vec(),
            status,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub ticks: u64,
    pub sample_every: u64,
    pub tick_interval: Duration,
    pub start_paused: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ticks: demo::END_TICK,
            sample_every: 1,
            tick_interval: Duration::from_millis(750),
            start_paused: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Command {
    Pause,
    Resume,
    Step,
    Stop,
}

// One queued sample plus one replaceable pending sample: no growing backlog.
// Only offer() clones cells, at the configured cadence or a control boundary.
struct Observer {
    sender: Option<SyncSender<Snapshot>>,
    pending: Option<Snapshot>,
}

impl Observer {
    fn offer(&mut self, world: &World, status: Status) {
        if self.sender.is_some() {
            self.pending = Some(Snapshot::capture(world, status));
            self.flush();
        }
    }

    fn flush(&mut self) {
        if let (Some(sender), Some(snapshot)) = (&self.sender, self.pending.take()) {
            match sender.try_send(snapshot) {
                Ok(()) => {}
                Err(TrySendError::Full(snapshot)) => self.pending = Some(snapshot),
                Err(TrySendError::Disconnected(_)) => self.sender = None,
            }
        }
    }
}

struct Run {
    world: World,
    config: Config,
    status: Status,
    observer: Observer,
    next_tick: Instant,
}

impl Run {
    fn advance(&mut self) -> Result<(), String> {
        let proposals = demo::proposals(&self.world);
        self.world.step(&proposals)?;
        if self.world.tick() == self.config.ticks {
            self.status = Status::Completed;
        }
        if self.world.tick().is_multiple_of(self.config.sample_every)
            || self.status != Status::Running
        {
            self.observer.offer(&self.world, self.status);
        }
        // This is a speed ceiling, without accumulating a catch-up backlog.
        self.next_tick = Instant::now() + self.config.tick_interval;
        Ok(())
    }

    fn control(&mut self, command: Command) -> Result<bool, String> {
        if matches!(command, Command::Stop) {
            return Ok(true);
        }
        if self.status == Status::Completed {
            return Ok(false);
        }
        match command {
            Command::Pause => self.status = Status::Paused,
            Command::Resume => {
                self.status = Status::Running;
                self.next_tick = Instant::now() + self.config.tick_interval;
            }
            Command::Step if self.status == Status::Paused => {
                self.advance()?;
                return Ok(false);
            }
            Command::Step | Command::Stop => return Ok(false),
        }
        self.observer.offer(&self.world, self.status);
        Ok(false)
    }

    fn work(
        mut self,
        commands: Receiver<Command>,
        completed: SyncSender<()>,
    ) -> Result<World, String> {
        self.observer.offer(&self.world, self.status);
        let mut announced_completion = false;
        loop {
            self.observer.flush();
            if self.status == Status::Completed {
                if !announced_completion {
                    // Computation is complete even if the viewer has read nothing.
                    let _ = completed.try_send(());
                    announced_completion = true;
                }
                if self.observer.pending.is_none() {
                    return Ok(self.world);
                }
            }

            // Commands are processed between complete ticks, before the next one.
            match commands.try_recv() {
                Ok(command) => {
                    if self.control(command)? {
                        return Ok(self.world);
                    }
                    continue;
                }
                Err(TryRecvError::Disconnected) => return Ok(self.world),
                Err(TryRecvError::Empty) => {}
            }
            if self.status == Status::Running && Instant::now() >= self.next_tick {
                self.advance()?;
                continue;
            }

            // A timeout retries a pending paused/final sample without blocking
            // on the viewer. Controls wake this wait immediately, including Stop.
            let wait = if self.status == Status::Running {
                self.next_tick
                    .saturating_duration_since(Instant::now())
                    .min(RETRY_INTERVAL)
            } else {
                RETRY_INTERVAL
            };
            match commands.recv_timeout(wait) {
                Ok(command) => {
                    if self.control(command)? {
                        return Ok(self.world);
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(self.world),
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

pub struct Worker {
    commands: Sender<Command>,
    completed: Receiver<()>,
    thread: Option<JoinHandle<Result<World, String>>>,
}

impl Worker {
    pub fn spawn(config: Config, snapshots: Option<SyncSender<Snapshot>>) -> Result<Self, String> {
        if config.sample_every == 0 {
            return Err("sample_every must be at least 1".into());
        }
        let next_tick = Instant::now()
            .checked_add(config.tick_interval)
            .ok_or("tick interval is too large")?;
        let status = if config.ticks == 0 {
            Status::Completed
        } else if config.start_paused {
            Status::Paused
        } else {
            Status::Running
        };
        let run = Run {
            world: demo::initial_world(),
            config,
            status,
            observer: Observer {
                sender: snapshots,
                pending: None,
            },
            next_tick,
        };
        let (commands, receiver) = mpsc::channel();
        let (finished, completed) = mpsc::sync_channel(1);
        // move transfers ownership into the worker. No UI shares a mutable World.
        let thread = thread::Builder::new()
            .name("virtual-life".into())
            .spawn(move || run.work(receiver, finished))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            commands,
            completed,
            thread: Some(thread),
        })
    }

    pub fn command(&self, command: Command) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| "worker has stopped".into())
    }

    /// A completion signal independent of the snapshot buffer. A timeout is a
    /// deadlock guard for callers/tests, not a simulation performance guarantee.
    pub fn wait_for_completion(&self, timeout: Duration) -> Result<(), RecvTimeoutError> {
        self.completed.recv_timeout(timeout)
    }

    pub fn is_finished(&self) -> bool {
        self.thread
            .as_ref()
            .is_none_or(|thread| thread.is_finished())
    }

    pub fn join(mut self) -> Result<World, String> {
        self.join_thread()
    }

    pub fn stop(mut self) -> Result<World, String> {
        let _ = self.commands.send(Command::Stop);
        self.join_thread()
    }

    fn join_thread(&mut self) -> Result<World, String> {
        self.thread
            .take()
            .expect("worker is joined only once")
            .join()
            .map_err(|_| "simulation worker panicked".to_owned())?
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if self.thread.is_some() {
            let _ = self.commands.send(Command::Stop);
            let _ = self.join_thread();
        }
    }
}
