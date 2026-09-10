//! The transition rules in MODEL.md. No clocks, threads, or observation here.
use std::collections::{HashMap, HashSet, VecDeque};

/// Behaviour-defining properties: wait/move/copy, then repair or random removal.
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

/// Wear-repair memory of the last selected action, independent of its success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionState {
    Wait,
    Move,
    Copy,
    Repair,
}

impl ActionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Wait => "Wait",
            Self::Move => "Move",
            Self::Copy => "Copy",
            Self::Repair => "Repair",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Agent {
    pub id: u64,
    pub value: i64,
    pub weights: Weights,
    pub integrity: u32,
    /// None before the first action, and unused by random-removal/demo worlds.
    pub last_action: Option<ActionState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Maintenance {
    pub maximum: u32,
    pub upkeep: u32,
    pub crowding_threshold: u32,
    pub crowding_upkeep: u32,
    pub move_wear: u32,
    pub copy_wear: u32,
    pub repair: u32,
}

impl Default for Maintenance {
    fn default() -> Self {
        Self {
            maximum: 10,
            upkeep: 1,
            crowding_threshold: 5,
            crowding_upkeep: 1,
            move_wear: 1,
            copy_wear: 2,
            repair: 4,
        }
    }
}

impl Maintenance {
    pub fn validate(self) -> Result<(), String> {
        if self.maximum == 0 {
            return Err("integrity maximum must be positive".into());
        }
        if self.crowding_threshold > 8 {
            return Err("crowding threshold must be between 0 and 8".into());
        }
        Ok(())
    }
}

/// A cost for the supplied, unchanged neighbourhood, before any actions or deaths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Upkeep {
    pub occupied_neighbors: u8,
    pub base_upkeep: u32,
    pub crowding_upkeep: u32,
    pub effective_upkeep: u64,
}

/// Shared by action selection, resolution and inspection of immutable snapshots.
pub fn upkeep_at(
    width: usize,
    height: usize,
    cells: &[Option<Agent>],
    position: Position,
    rules: Maintenance,
) -> Result<Upkeep, String> {
    rules.validate()?;
    if width.checked_mul(height) != Some(cells.len()) {
        return Err("cell count does not match grid dimensions".into());
    }
    let occupied_neighbors = neighbor_positions(width, height, position)?
        .iter()
        .filter(|at| cells[at.y * width + at.x].is_some())
        .count() as u8;
    let crowding_upkeep = if u32::from(occupied_neighbors) >= rules.crowding_threshold {
        rules.crowding_upkeep
    } else {
        0
    };
    Ok(Upkeep {
        occupied_neighbors,
        base_upkeep: rules.upkeep,
        crowding_upkeep,
        effective_upkeep: u64::from(rules.upkeep) + u64::from(crowding_upkeep),
    })
}

fn neighbor_positions(
    width: usize,
    height: usize,
    position: Position,
) -> Result<[Position; 8], String> {
    if width < 3 || height < 3 || position.x >= width || position.y >= height {
        return Err("neighbours require a position inside a grid of at least 3 by 3".into());
    }
    let left = if position.x == 0 {
        width - 1
    } else {
        position.x - 1
    };
    let right = if position.x == width - 1 {
        0
    } else {
        position.x + 1
    };
    let up = if position.y == 0 {
        height - 1
    } else {
        position.y - 1
    };
    let down = if position.y == height - 1 {
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

pub const FAILURE_LIMIT: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    Upkeep,
    MoveWear,
    CopyWear,
}

impl FailureReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::Upkeep => "upkeep",
            Self::MoveWear => "move wear",
            Self::CopyWear => "copy wear",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub id: u64,
    pub tick: u64,
    pub position: Position,
    pub reason: FailureReason,
    pub integrity_before: u32,
    pub occupied_neighbors: u8,
    pub base_upkeep: u32,
    pub crowding_upkeep: u32,
    pub upkeep: u64,
    pub action_wear: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Wait,
    Move(Position),
    SetValue(i64),
    Create(Position),
    Remove,
    Repair,
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
    pub repairs: u64,
    pub failures: u64,
}

impl Events {
    fn checked_add(self, other: Self) -> Result<Self, String> {
        let add = |a: u64, b: u64| a.checked_add(b).ok_or("event counter exhausted".to_owned());
        Ok(Self {
            moves: add(self.moves, other.moves)?,
            creations: add(self.creations, other.creations)?,
            removals: add(self.removals, other.removals)?,
            value_changes: add(self.value_changes, other.value_changes)?,
            repairs: add(self.repairs, other.repairs)?,
            failures: add(self.failures, other.failures)?,
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
    maintenance: Option<Maintenance>,
    failures: VecDeque<Failure>,
    discarded_failures: u64,
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
            maintenance: None,
            failures: VecDeque::new(),
            discarded_failures: 0,
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

    pub fn with_maintenance(
        width: usize,
        height: usize,
        agents: &[(Position, Agent)],
        maintenance: Maintenance,
    ) -> Result<Self, String> {
        maintenance.validate()?;
        let mut world = Self::new(width, height, agents)?;
        if world
            .current
            .iter()
            .flatten()
            .any(|agent| agent.integrity == 0 || agent.integrity > maintenance.maximum)
        {
            return Err("initial integrity must be within 1..=maximum".into());
        }
        world.maintenance = Some(maintenance);
        Ok(world)
    }

    pub fn maintenance(&self) -> Option<Maintenance> {
        self.maintenance
    }
    pub fn failures(&self) -> &VecDeque<Failure> {
        &self.failures
    }
    pub fn discarded_failures(&self) -> u64 {
        self.discarded_failures
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
        neighbor_positions(self.width, self.height, position)
    }

    /// Validate everything, resolve claims, then commit one synchronous tick.
    /// An error leaves both buffers, tick, counters, and ID allocation unchanged.
    pub fn step(&mut self, proposals: &[Proposal]) -> Result<Events, String> {
        let mut actions = vec![None; self.current.len()];

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
                self.index(target)?;
                let origin = Position::new(source % self.width, source / self.width);
                if !self.neighbors(origin)?.contains(&target) {
                    return Err("target must be one of the eight neighbors".into());
                }
            }
            if matches!(proposal.action, Action::Repair) && self.maintenance.is_none() {
                return Err("repair requires wear-repair survival".into());
            }
            if matches!(proposal.action, Action::Remove | Action::SetValue(_))
                && self.maintenance.is_some()
            {
                return Err("wear-repair actions are wait, move, copy or repair".into());
            }
            actions[source] = Some(proposal.action);
        }

        let tick = self.tick.checked_add(1).ok_or("tick counter exhausted")?;
        let mut accepted = Events::default();
        let mut states: Vec<_> = self
            .current
            .iter()
            .map(|cell| cell.map(|a| (a.integrity, a.last_action)))
            .collect();
        let mut failures = Vec::new();
        if let Some(rules) = self.maintenance {
            rules.validate()?;
            for (source, cell) in self.current.iter().enumerate() {
                let Some(agent) = cell else {
                    continue;
                };
                if agent.integrity == 0 || agent.integrity > rules.maximum {
                    return Err("integrity outside 1..=maximum".into());
                }
                let action = actions[source].unwrap_or(Action::Wait);
                let (wear, reason) = match action {
                    Action::Move(_) => (rules.move_wear, FailureReason::MoveWear),
                    Action::Create(_) => (rules.copy_wear, FailureReason::CopyWear),
                    _ => (0, FailureReason::Upkeep),
                };
                let position = Position::new(source % self.width, source / self.width);
                let upkeep = upkeep_at(self.width, self.height, &self.current, position, rules)?;
                let failed = if u64::from(agent.integrity) <= upkeep.effective_upkeep {
                    Some((FailureReason::Upkeep, 0))
                } else if u64::from(agent.integrity) - upkeep.effective_upkeep <= u64::from(wear) {
                    Some((reason, wear))
                } else {
                    None
                };
                if let Some((reason, action_wear)) = failed {
                    actions[source] = Some(Action::Remove);
                    accepted.failures += 1;
                    failures.push(Failure {
                        id: agent.id,
                        tick,
                        position,
                        reason,
                        integrity_before: agent.integrity,
                        occupied_neighbors: upkeep.occupied_neighbors,
                        base_upkeep: upkeep.base_upkeep,
                        crowding_upkeep: upkeep.crowding_upkeep,
                        upkeep: upkeep.effective_upkeep,
                        action_wear,
                    });
                } else {
                    // Survival proves this difference is positive and fits u32.
                    let after_upkeep =
                        (u64::from(agent.integrity) - upkeep.effective_upkeep) as u32;
                    let remaining = if action == Action::Repair {
                        let repaired = (u64::from(after_upkeep) + u64::from(rules.repair))
                            .min(u64::from(rules.maximum))
                            as u32;
                        accepted.repairs += u64::from(repaired > after_upkeep);
                        repaired
                    } else {
                        after_upkeep - wear
                    };
                    // Record the selection before destination resolution can
                    // replace a rejected Move/Copy with a physical wait.
                    let selected = match action {
                        Action::Wait => ActionState::Wait,
                        Action::Move(_) => ActionState::Move,
                        Action::Create(_) => ActionState::Copy,
                        Action::Repair => ActionState::Repair,
                        _ => unreachable!("validated wear-repair action"),
                    };
                    states[source] = Some((remaining, Some(selected)));
                }
            }
        }

        // Only affordable spatial actions claim destinations. Failed attempts still
        // paid wear, but dying actors cannot block another start-empty claim.
        let mut claims = vec![0_usize; self.current.len()];
        for action in actions.iter().flatten() {
            if let Action::Move(target) | Action::Create(target) = action {
                let destination = target.y * self.width + target.x;
                if self.current[destination].is_none() {
                    claims[destination] += 1;
                }
            }
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
        let next_id = self
            .next_id
            .checked_add(accepted.creations)
            .ok_or("agent IDs exhausted")?;
        let discarded = self
            .failures
            .len()
            .saturating_add(failures.len())
            .saturating_sub(FAILURE_LIMIT);
        let discarded_failures = self
            .discarded_failures
            .checked_add(discarded as u64)
            .ok_or("discarded failure counter exhausted")?;

        // All possible input errors have been checked. Build using only old agents.
        self.next.clone_from(&self.current);
        let mut child_id = self.next_id;
        for (source, action) in actions.iter().enumerate() {
            let Some(mut agent) = self.current[source] else {
                continue;
            };
            (agent.integrity, agent.last_action) = states[source].unwrap();
            self.next[source] = Some(agent);
            match action.unwrap_or(Action::Wait) {
                Action::Wait | Action::Repair => {}
                Action::Move(target) => {
                    self.next[source] = None;
                    self.next[target.y * self.width + target.x] = Some(agent);
                }
                Action::Create(target) => {
                    self.next[target.y * self.width + target.x] = Some(Agent {
                        id: child_id,
                        last_action: None,
                        integrity: self
                            .maintenance
                            .map_or(agent.integrity, |rules| rules.maximum),
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
        for failure in failures {
            if self.failures.len() == FAILURE_LIMIT {
                self.failures.pop_front();
            }
            self.failures.push_back(failure);
        }
        self.discarded_failures = discarded_failures;
        Ok(accepted)
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn selected_memory_and_integrity_roll_back_at_every_wear_counter_and_id_limit() {
        for exhausted in 0..7 {
            let action = match exhausted {
                1 => Action::Move(Position::new(2, 1)),
                2 | 3 => Action::Create(Position::new(2, 1)),
                4 => Action::Repair,
                _ => Action::Wait,
            };
            let agent = Agent {
                id: 1,
                integrity: 10,
                last_action: Some(if action == Action::Repair {
                    ActionState::Move
                } else {
                    ActionState::Repair
                }),
                ..Agent::default()
            };
            let mut world = World::with_maintenance(
                5,
                5,
                &[
                    (Position::new(1, 1), agent),
                    (
                        Position::new(4, 4),
                        Agent {
                            id: 2,
                            integrity: 1,
                            ..agent
                        },
                    ),
                ],
                Maintenance::default(),
            )
            .unwrap();
            match exhausted {
                0 => world.tick = u64::MAX,
                1 => world.totals.moves = u64::MAX,
                2 => world.totals.creations = u64::MAX,
                3 => world.next_id = u64::MAX,
                4 => world.totals.repairs = u64::MAX,
                5 => world.totals.removals = u64::MAX,
                _ => world.totals.failures = u64::MAX,
            }
            let before = world.clone();
            assert!(world.step(&[Proposal::new(1, action)]).is_err());
            assert_eq!(world, before, "exhaustion case {exhausted}");
        }
    }

    #[test]
    fn wear_events_and_record_eviction_reject_atomically_at_counter_limits() {
        let rules = Maintenance::default();
        for repairs in [false, true] {
            let agent = Agent {
                id: 1,
                integrity: if repairs { 10 } else { 1 },
                ..Agent::default()
            };
            let mut world =
                World::with_maintenance(3, 3, &[(Position::new(1, 1), agent)], rules).unwrap();
            if repairs {
                world.totals.repairs = u64::MAX;
            } else {
                world.totals.failures = u64::MAX;
            }
            let before = world.clone();
            assert!(world.step(&[Proposal::new(1, Action::Repair)]).is_err());
            assert_eq!(world, before);
            if repairs {
                world.totals.repairs = u64::MAX - 1;
            } else {
                world.totals.failures = u64::MAX - 1;
            }
            world.step(&[Proposal::new(1, Action::Repair)]).unwrap();
            assert_eq!(
                if repairs {
                    world.totals.repairs
                } else {
                    world.totals.failures
                },
                u64::MAX
            );
        }
        let mut world = World::with_maintenance(
            3,
            3,
            &[(
                Position::new(1, 1),
                Agent {
                    id: 1,
                    integrity: 1,
                    ..Agent::default()
                },
            )],
            rules,
        )
        .unwrap();
        let old = Failure {
            id: 100,
            tick: 0,
            position: Position::new(0, 0),
            reason: FailureReason::Upkeep,
            integrity_before: 1,
            occupied_neighbors: 0,
            base_upkeep: 1,
            crowding_upkeep: 0,
            upkeep: 1,
            action_wear: 0,
        };
        world.failures = std::iter::repeat_n(old, FAILURE_LIMIT).collect();
        world.discarded_failures = u64::MAX;
        let before = world.clone();
        assert!(world.step(&[]).is_err());
        assert_eq!(world, before);
    }

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
