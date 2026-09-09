//! The transition rules in MODEL.md. No clocks, threads, or observation here.
use std::collections::{HashMap, HashSet};

/// Behaviour-defining properties, in wait/move/copy/remove order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Weights(pub [u32; 4]);

impl Default for Weights {
    fn default() -> Self {
        Self([1, 0, 0, 0])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Agent {
    pub id: u64,
    pub value: i64,
    pub weights: Weights,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Wait,
    Move(Position),
    SetValue(i64),
    Create(Position),
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Proposal {
    pub actor: u64,
    pub action: Action,
}

impl Proposal {
    pub const fn new(actor: u64, action: Action) -> Self {
        Self { actor, action }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    pub moves: u64,
    pub creations: u64,
    pub removals: u64,
    pub value_changes: u64,
}

impl Events {
    fn checked_add(self, other: Self) -> Result<Self, String> {
        let add = |a: u64, b: u64| a.checked_add(b).ok_or("event counter exhausted".to_owned());
        Ok(Self {
            moves: add(self.moves, other.moves)?,
            creations: add(self.creations, other.creations)?,
            removals: add(self.removals, other.removals)?,
            value_changes: add(self.value_changes, other.value_changes)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    width: usize,
    height: usize,
    // Vec is a growable contiguous array. Each square is Some(agent) or None.
    current: Vec<Option<Agent>>,
    next: Vec<Option<Agent>>,
    tick: u64,
    totals: Events,
    next_id: u64,
}

impl World {
    pub fn new(width: usize, height: usize, agents: &[(Position, Agent)]) -> Result<Self, String> {
        if width < 3 || height < 3 {
            return Err("width and height must both be at least 3".into());
        }
        let size = width
            .checked_mul(height)
            .ok_or("grid dimensions overflow")?;
        let mut current = Vec::new();
        current
            .try_reserve_exact(size)
            .map_err(|_| "grid is too large to allocate")?;
        current.resize(size, None);
        let mut next = Vec::new();
        next.try_reserve_exact(size)
            .map_err(|_| "grid is too large to allocate")?;
        next.resize(size, None);
        let mut world = Self {
            width,
            height,
            current,
            next,
            tick: 0,
            totals: Events::default(),
            next_id: 1,
        };
        let mut ids = HashSet::new();
        for &(position, agent) in agents {
            let index = world.index(position)?;
            if world.current[index].is_some() {
                return Err("initial square is already occupied".into());
            }
            if !ids.insert(agent.id) {
                return Err("initial agent IDs must be unique".into());
            }
            world.next_id = world
                .next_id
                .max(agent.id.checked_add(1).ok_or("agent IDs exhausted")?);
            world.current[index] = Some(agent);
        }
        Ok(world)
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn totals(&self) -> Events {
        self.totals
    }
    pub fn count(&self) -> usize {
        self.current.iter().flatten().count()
    }
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    // & borrows the array for reading. Callers cannot mutate the engine's grid.
    pub fn cells(&self) -> &[Option<Agent>] {
        &self.current
    }

    pub fn agent_at(&self, position: Position) -> Result<Option<Agent>, String> {
        Ok(self.current[self.index(position)?])
    }

    fn index(&self, position: Position) -> Result<usize, String> {
        if position.x >= self.width || position.y >= self.height {
            return Err(format!(
                "position ({},{}) is outside the grid",
                position.x, position.y
            ));
        }
        Ok(position.y * self.width + position.x)
    }

    pub fn neighbors(&self, position: Position) -> Result<[Position; 8], String> {
        self.index(position)?;
        let left = if position.x == 0 {
            self.width - 1
        } else {
            position.x - 1
        };
        let right = if position.x == self.width - 1 {
            0
        } else {
            position.x + 1
        };
        let up = if position.y == 0 {
            self.height - 1
        } else {
            position.y - 1
        };
        let down = if position.y == self.height - 1 {
            0
        } else {
            position.y + 1
        };
        Ok([
            Position::new(left, up),
            Position::new(position.x, up),
            Position::new(right, up),
            Position::new(left, position.y),
            Position::new(right, position.y),
            Position::new(left, down),
            Position::new(position.x, down),
            Position::new(right, down),
        ])
    }

    /// Validate everything, resolve claims, then commit one synchronous tick.
    /// An error leaves both buffers, tick, counters, and ID allocation unchanged.
    pub fn step(&mut self, proposals: &[Proposal]) -> Result<Events, String> {
        let mut actions = vec![None; self.current.len()];
        let mut claims = vec![0_usize; self.current.len()];

        // Each proposal is attached to its actor's STARTING square.
        // Lookup only: map iteration never determines actions or child IDs.
        let sources: HashMap<_, _> = self
            .current
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| cell.map(|agent| (agent.id, index)))
            .collect();
        for proposal in proposals {
            let source = *sources
                .get(&proposal.actor)
                .ok_or_else(|| format!("unknown starting actor {}", proposal.actor))?;
            if actions[source].is_some() {
                return Err(format!(
                    "more than one proposal for actor {}",
                    proposal.actor
                ));
            }
            if let Action::Move(target) | Action::Create(target) = proposal.action {
                let destination = self.index(target)?;
                let origin = Position::new(source % self.width, source / self.width);
                if !self.neighbors(origin)?.contains(&target) {
                    return Err("target must be one of the eight neighbors".into());
                }
                if self.current[destination].is_none() {
                    claims[destination] += 1;
                }
            }
            actions[source] = Some(proposal.action);
        }

        // Occupied targets and competing claims are valid, but rejected actions.
        // Resolve them to waits before counting or writing anything.
        for action in actions.iter_mut().flatten() {
            if let Action::Move(target) | Action::Create(target) = *action {
                let destination = target.y * self.width + target.x;
                if self.current[destination].is_some() || claims[destination] != 1 {
                    *action = Action::Wait;
                }
            }
        }
        let mut accepted = Events::default();
        for (source, action) in actions.iter().enumerate() {
            match action {
                Some(Action::Move(_)) => accepted.moves += 1,
                Some(Action::Create(_)) => accepted.creations += 1,
                Some(Action::Remove) => accepted.removals += 1,
                Some(Action::SetValue(value)) if self.current[source].unwrap().value != *value => {
                    accepted.value_changes += 1
                }
                _ => {}
            }
        }
        let totals = self.totals.checked_add(accepted)?;
        let tick = self.tick.checked_add(1).ok_or("tick counter exhausted")?;
        let next_id = self
            .next_id
            .checked_add(accepted.creations)
            .ok_or("agent IDs exhausted")?;

        // All possible input errors have been checked. Build using only old agents.
        self.next.clone_from(&self.current);
        let mut child_id = self.next_id;
        for (source, action) in actions.iter().enumerate() {
            let Some(agent) = self.current[source] else {
                continue;
            };
            match action.unwrap_or(Action::Wait) {
                Action::Wait => {}
                Action::Move(target) => {
                    self.next[source] = None;
                    self.next[target.y * self.width + target.x] = Some(agent);
                }
                Action::Create(target) => {
                    self.next[target.y * self.width + target.x] = Some(Agent {
                        id: child_id,
                        ..agent
                    });
                    child_id += 1;
                }
                Action::SetValue(value) => self.next[source] = Some(Agent { value, ..agent }),
                Action::Remove => self.next[source] = None,
            }
        }
        // Swap the two arrays, rather than copying the finished grid back.
        // Accepted children get IDs in row-major order of their creators' old squares.
        std::mem::swap(&mut self.current, &mut self.next);
        self.tick = tick;
        self.totals = totals;
        self.next_id = next_id;
        Ok(accepted)
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn tick_exhaustion_leaves_both_buffers_and_all_counters_unchanged() {
        let mut world = crate::demo::initial_world();
        world.tick = u64::MAX - 1;
        world
            .step(&[Proposal::new(1, Action::SetValue(i64::MIN))])
            .unwrap();
        assert_eq!(world.tick, u64::MAX);
        let before = world.clone();
        assert!(
            world
                .step(&[
                    Proposal::new(1, Action::Remove),
                    Proposal::new(2, Action::Create(Position::new(3, 2)))
                ])
                .is_err()
        );
        assert_eq!(world, before);
    }

    #[test]
    fn every_event_counter_can_reach_its_limit_then_rejects_atomically() {
        for intent in [
            Action::Move(Position::new(1, 2)),
            Action::Create(Position::new(1, 2)),
            Action::Remove,
            Action::SetValue(i64::MAX),
        ] {
            let mut world = crate::demo::initial_world();
            let counter = match intent {
                Action::Move(_) => &mut world.totals.moves,
                Action::Create(_) => &mut world.totals.creations,
                Action::Remove => &mut world.totals.removals,
                Action::SetValue(_) => &mut world.totals.value_changes,
                _ => unreachable!(),
            };
            *counter = u64::MAX;
            let before = world.clone();
            assert!(
                world
                    .step(&[
                        Proposal::new(1, intent),
                        Proposal::new(3, Action::SetValue(99))
                    ])
                    .is_err()
            );
            assert_eq!(world, before, "{intent:?}");
            let counter = match intent {
                Action::Move(_) => &mut world.totals.moves,
                Action::Create(_) => &mut world.totals.creations,
                Action::Remove => &mut world.totals.removals,
                Action::SetValue(_) => &mut world.totals.value_changes,
                _ => unreachable!(),
            };
            *counter = u64::MAX - 1;
            world.step(&[Proposal::new(1, intent)]).unwrap();
            let totals = world.totals;
            assert!(
                [
                    totals.moves,
                    totals.creations,
                    totals.removals,
                    totals.value_changes
                ]
                .contains(&u64::MAX)
            );
        }
    }
}
