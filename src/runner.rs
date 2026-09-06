//! Thread ownership, controls, pacing, and lossy observation; not model rules.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::{
    demo,
    engine::{Agent, Events, World},
};

pub const SNAPSHOT_CAPACITY: usize = 1;
pub const COMMAND_CAPACITY: usize = 16;
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

/// A receipt describes an action already processed between ticks, not enqueueing.
#[derive(Clone, Debug)]
pub struct Receipt {
    pub applied: bool,
    pub tick: u64,
    pub status: Status,
}

struct Request {
    command: Command,
    reply: Option<SyncSender<Receipt>>,
}

#[derive(Clone)]
pub struct Controls {
    sender: SyncSender<Request>,
    stopping: Arc<AtomicBool>,
}

impl Controls {
    fn send(&self, command: Command, reply: Option<SyncSender<Receipt>>) -> Result<(), String> {
        if matches!(command, Command::Stop) {
            // Shutdown cannot be lost behind a full command queue.
            self.stopping.store(true, Ordering::Relaxed);
            return Ok(());
        }
        self.sender
            .try_send(Request { command, reply })
            .map_err(|error| match error {
                TrySendError::Full(_) => "control queue is full".into(),
                TrySendError::Disconnected(_) => "worker has completed or stopped".into(),
            })
    }

    pub fn request(&self, command: Command) -> Result<Receiver<Receipt>, String> {
        if matches!(command, Command::Stop) {
            return Err("use worker shutdown, not a control receipt, to stop".into());
        }
        let (reply, receiver) = mpsc::sync_channel(1);
        self.send(command, Some(reply))?;
        Ok(receiver)
    }
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
    stopping: Arc<AtomicBool>,
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

    fn apply_request(&mut self, request: Request) -> Result<bool, String> {
        let applied = self.status != Status::Completed
            && (!matches!(request.command, Command::Step) || self.status == Status::Paused);
        let stop = self.control(request.command)?;
        if let Some(reply) = request.reply {
            let _ = reply.try_send(Receipt {
                applied,
                tick: self.world.tick(),
                status: self.status,
            });
        }
        Ok(stop)
    }

    fn work(
        mut self,
        commands: Receiver<Request>,
        completed: SyncSender<()>,
    ) -> Result<World, String> {
        self.observer.offer(&self.world, self.status);
        let mut announced_completion = false;
        loop {
            if self.stopping.load(Ordering::Relaxed) {
                return Ok(self.world);
            }
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
                    if self.apply_request(command)? {
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
                    if self.apply_request(command)? {
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
    commands: Controls,
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
        let stopping = Arc::new(AtomicBool::new(false));
        let run = Run {
            world: demo::initial_world(),
            config,
            status,
            observer: Observer {
                sender: snapshots,
                pending: None,
            },
            next_tick,
            stopping: stopping.clone(),
        };
        let (sender, receiver) = mpsc::sync_channel(COMMAND_CAPACITY);
        let commands = Controls { sender, stopping };
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
        self.commands.send(command, None)
    }

    pub fn controls(&self) -> Controls {
        self.commands.clone()
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
        let _ = self.command(Command::Stop);
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
            let _ = self.command(Command::Stop);
            let _ = self.join_thread();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_buffer_is_bounded_and_shutdown_bypasses_a_full_queue() {
        let (sender, _unread) = mpsc::sync_channel(COMMAND_CAPACITY);
        let controls = Controls {
            sender,
            stopping: Arc::new(AtomicBool::new(false)),
        };
        let mut receipts = Vec::new();
        for _ in 0..COMMAND_CAPACITY {
            receipts.push(controls.request(Command::Step).unwrap());
        }
        assert_eq!(
            controls.request(Command::Step).unwrap_err(),
            "control queue is full"
        );
        assert!(
            receipts
                .iter()
                .all(|reply| matches!(reply.try_recv(), Err(TryRecvError::Empty)))
        );
        controls.send(Command::Stop, None).unwrap();
        assert!(controls.stopping.load(Ordering::Relaxed));
    }
}
