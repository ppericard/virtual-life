//! Reproducible initialisation and action choice. Resolution stays in World::step.
use crate::engine::{Action, ActionState, Agent, Maintenance, Position, Proposal, Weights, World};

pub const GENERATOR: &str = "SplitMix64 / VirtualLife sampling v1";
// Vigna's 2015 public-domain reference: https://prng.di.unimi.it/splitmix64.c
pub const OCCUPANCY_SCALE: u32 = 1_000_000;
pub const MAX_CELLS: usize = 262_144;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentConfig {
    pub width: usize,
    pub height: usize,
    pub occupancy: u32,
    pub bundles: Vec<Weights>,
    pub proportions: Vec<u32>,
    pub seed: u64,
    pub maintenance: Option<Maintenance>,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            width: 32,
            height: 24,
            occupancy: 300_000,
            bundles: vec![
                Weights([2, 4, 1, 3]),
                Weights([2, 2, 2, 4]),
                Weights([4, 1, 1, 4]),
                Weights([1, 5, 2, 2]),
            ],
            proportions: vec![1; 4],
            seed: 1,
            maintenance: Some(Maintenance::default()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub weights: Weights,
    pub proportion: u64,
    pub initial_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentInfo {
    pub config: ExperimentConfig,
    pub groups: Vec<Group>,
}

impl ExperimentInfo {
    pub fn counts(&self, cells: &[Option<Agent>]) -> Vec<usize> {
        let mut counts = vec![0; self.groups.len()];
        for agent in cells.iter().flatten() {
            let group = self
                .groups
                .iter()
                .position(|g| g.weights == agent.weights)
                .expect("autonomous agents retain a configured property bundle");
            counts[group] += 1;
        }
        counts
    }
}

/// The generator and all draws belong to one simulation, never to its observer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    // Reject the short remainder, then reduce: every result has equal mass.
    fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0);
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let draw = self.next();
            if draw >= threshold {
                return draw % bound;
            }
        }
    }
}

impl ExperimentConfig {
    pub fn random() -> Self {
        Self {
            bundles: vec![
                Weights([4, 5, 1, 1]),
                Weights([4, 3, 2, 1]),
                Weights([6, 2, 1, 1]),
                Weights([2, 6, 2, 1]),
            ],
            maintenance: None,
            ..Self::default()
        }
    }

    pub fn survival(&self) -> &'static str {
        if self.maintenance.is_some() {
            "wear-repair"
        } else {
            "random"
        }
    }
    pub fn protocol(&self) -> &'static str {
        if self.maintenance.is_some() {
            "wear-repair crowding v3"
        } else {
            "random v1"
        }
    }

    pub fn initialize(&self) -> Result<(World, Random, ExperimentInfo), String> {
        if let Some(rules) = self.maintenance {
            rules.validate()?;
        }
        let size = self
            .width
            .checked_mul(self.height)
            .ok_or("grid dimensions overflow")?;
        if self.width < 3 || self.height < 3 || size > MAX_CELLS {
            return Err(format!(
                "dimensions must each be at least 3, with at most {MAX_CELLS} squares"
            ));
        }
        if self.occupancy > OCCUPANCY_SCALE {
            return Err("occupancy must be between 0 and 1".into());
        }
        if self.bundles.is_empty() || self.bundles.len() != self.proportions.len() {
            return Err("provide one proportion per bundle and at least one bundle".into());
        }
        let mut groups: Vec<Group> = Vec::new();
        for (&weights, &proportion) in self.bundles.iter().zip(&self.proportions) {
            if weights.0.iter().all(|&weight| weight == 0) {
                return Err("each bundle must have a positive total weight".into());
            }
            if let Some(group) = groups.iter_mut().find(|g| g.weights == weights) {
                group.proportion = group
                    .proportion
                    .checked_add(u64::from(proportion))
                    .ok_or("proportion total overflow")?;
            } else {
                groups.push(Group {
                    weights,
                    proportion: u64::from(proportion),
                    initial_count: 0,
                });
            }
        }
        let total = groups
            .iter()
            .try_fold(0_u64, |sum, g| sum.checked_add(g.proportion))
            .ok_or("proportion total overflow")?;
        if groups.len() > 8 {
            return Err(
                "at most eight distinct bundles are supported by the viewer palette".into(),
            );
        }
        if total == 0 {
            return Err("proportions must have a positive total".into());
        }
        let population = size as u64 * u64::from(self.occupancy) / u64::from(OCCUPANCY_SCALE);
        // Exact counts: floor each quota, then award remaining seats by largest
        // remainder (first canonical group wins ties). No randomness here.
        let mut remainders = Vec::new();
        for (index, group) in groups.iter_mut().enumerate() {
            let quota = u128::from(population) * u128::from(group.proportion);
            group.initial_count = (quota / u128::from(total)) as usize;
            remainders.push((index, quota % u128::from(total)));
        }
        remainders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let assigned: usize = groups.iter().map(|g| g.initial_count).sum();
        for &(index, _) in remainders.iter().take(population as usize - assigned) {
            groups[index].initial_count += 1;
        }
        // Shuffle all square indices; fill successive slots with the group quotas.
        let mut random = Random::new(self.seed);
        let mut squares: Vec<_> = (0..size).collect();
        for index in (1..size).rev() {
            let other = random.below(index as u64 + 1) as usize;
            squares.swap(index, other);
        }
        let mut occupied = Vec::new();
        let mut slots = squares.into_iter();
        for group in &groups {
            for _ in 0..group.initial_count {
                occupied.push((slots.next().unwrap(), group.weights));
            }
        }
        occupied.sort_by_key(|&(square, _)| square);
        let agents: Vec<_> = occupied
            .into_iter()
            .enumerate()
            .map(|(index, (square, weights))| {
                (
                    Position::new(square % self.width, square / self.width),
                    Agent {
                        id: index as u64 + 1,
                        value: 0,
                        weights,
                        integrity: self.maintenance.map_or(0, |rules| rules.maximum),
                        last_action: None,
                    },
                )
            })
            .collect();
        Ok((
            match self.maintenance {
                Some(rules) => World::with_maintenance(self.width, self.height, &agents, rules)?,
                None => World::new(self.width, self.height, &agents)?,
            },
            random,
            ExperimentInfo {
                config: self.clone(),
                groups,
            },
        ))
    }
}

/// Shared wear-repair transition weights in Wait/Move/Copy/Repair order.
/// State and positive integrity after mandatory upkeep are explicit inputs for
/// later rules; neither changes the probabilities in this increment. Neighbours
/// are immutable starting occupants, including individuals about to fail upkeep.
/// Eightfold tickets preserve the v3 draw bound: at most 32 * u32::MAX in u64.
pub fn transition_weights(
    _current: Option<ActionState>,
    properties: Weights,
    _integrity_after_upkeep: u32,
    neighbours: &[Option<Agent>; 8],
) -> [u64; 4] {
    let occupied = neighbours.iter().flatten().count() as u64;
    let [wait, movement, copy, repair] = properties.0.map(u64::from);
    [
        8 * wait + occupied * copy,
        8 * movement,
        (8 - occupied) * copy,
        8 * repair,
    ]
}

/// Each starting individual chooses once in row-major order. Wear-repair reduces
/// Copy according to starting empty neighbours and transfers its lost share to Wait.
/// Requires a validated autonomous world: every agent has a positive weight sum.
pub fn proposals(world: &World, random: &mut Random) -> Vec<Proposal> {
    world
        .cells()
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            let agent = (*cell)?;
            let position = Position::new(index % world.width(), index / world.width());
            let upkeep = world.maintenance().map(|rules| {
                crate::engine::upkeep_at(
                    world.width(),
                    world.height(),
                    world.cells(),
                    position,
                    rules,
                )
                .expect("validated autonomous world")
            });
            if upkeep.is_some_and(|cost| u64::from(agent.integrity) <= cost.effective_upkeep) {
                return None; // Upkeep failure is resolved before action choice; no draw.
            }
            let tickets = if let Some(cost) = upkeep {
                let neighbours = world
                    .neighbors(position)
                    .unwrap()
                    .map(|position| world.agent_at(position).unwrap());
                // The upkeep check above proves this difference is positive.
                transition_weights(
                    agent.last_action,
                    agent.weights,
                    (u64::from(agent.integrity) - cost.effective_upkeep) as u32,
                    &neighbours,
                )
            } else {
                agent.weights.0.map(u64::from)
            };
            let total: u64 = tickets.iter().sum();
            let mut draw = random.below(total);
            let choice = tickets
                .iter()
                .position(|&weight| {
                    if draw < weight {
                        true
                    } else {
                        draw -= weight;
                        false
                    }
                })
                .unwrap();
            let action = match choice {
                0 => Action::Wait,
                1 | 2 => {
                    let target = world.neighbors(position).unwrap()[random.below(8) as usize];
                    if choice == 1 {
                        Action::Move(target)
                    } else {
                        Action::Create(target)
                    }
                }
                _ if world.maintenance().is_some() => Action::Repair,
                _ => Action::Remove,
            };
            Some(Proposal::new(agent.id, action))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crowding_selection_uses_exact_tickets_starting_occupancy_and_one_action_draw() {
        let maximum = u64::from(u32::MAX);
        let cases = [
            (Weights([2, 4, 1, 3]), 8, [16, 32, 8, 24]),
            (Weights([2, 4, 1, 3]), 4, [20, 32, 4, 24]),
            (Weights([2, 4, 1, 3]), 0, [24, 32, 0, 24]),
            (Weights([1, 2, 3, 4]), 1, [29, 16, 3, 32]),
            (Weights([1, 2, 3, 4]), 7, [11, 16, 21, 32]),
            (Weights([1, 2, 0, 4]), 0, [8, 16, 0, 32]),
            (Weights([1, 2, 0, 4]), 7, [8, 16, 0, 32]),
            (
                Weights([u32::MAX; 4]),
                0,
                [16 * maximum, 8 * maximum, 0, 8 * maximum],
            ),
            (
                Weights([u32::MAX; 4]),
                1,
                [15 * maximum, 8 * maximum, maximum, 8 * maximum],
            ),
            (
                Weights([u32::MAX; 4]),
                7,
                [9 * maximum, 8 * maximum, 7 * maximum, 8 * maximum],
            ),
            (Weights([u32::MAX; 4]), 8, [8 * maximum; 4]),
        ];
        // Explicit coordinates include wrapping at the corner. Neighbours are
        // doomed by upkeep but still occupy their starting squares for choice.
        for (position, neighbors) in [
            (
                Position::new(2, 2),
                [
                    (1, 1),
                    (2, 1),
                    (3, 1),
                    (1, 2),
                    (3, 2),
                    (1, 3),
                    (2, 3),
                    (3, 3),
                ],
            ),
            (
                Position::new(0, 0),
                [
                    (4, 4),
                    (0, 4),
                    (1, 4),
                    (4, 0),
                    (1, 0),
                    (4, 1),
                    (0, 1),
                    (1, 1),
                ],
            ),
        ] {
            for current in [
                None,
                Some(ActionState::Wait),
                Some(ActionState::Move),
                Some(ActionState::Copy),
                Some(ActionState::Repair),
            ] {
                for (weights, empty, tickets) in cases {
                    let mut agents = vec![(
                        position,
                        Agent {
                            id: 1,
                            weights,
                            integrity: 10,
                            last_action: current,
                            ..Agent::default()
                        },
                    )];
                    for (index, &(x, y)) in neighbors.iter().take(8 - empty).enumerate() {
                        agents.push((
                            Position::new(x, y),
                            Agent {
                                id: index as u64 + 2,
                                weights,
                                integrity: 1,
                                ..Agent::default()
                            },
                        ));
                    }
                    let world =
                        World::with_maintenance(5, 5, &agents, Maintenance::default()).unwrap();
                    let before = world.clone();
                    let mut seen = [false; 4];
                    for seed in 0..1024 {
                        let mut expected_random = Random::new(seed);
                        let draw = expected_random.below(tickets.iter().sum());
                        let choice = if draw < tickets[0] {
                            0
                        } else if draw < tickets[0] + tickets[1] {
                            1
                        } else if draw < tickets[0] + tickets[1] + tickets[2] {
                            2
                        } else {
                            3
                        };
                        seen[choice] = true;
                        let expected = match choice {
                            0 => Action::Wait,
                            1 | 2 => {
                                let (x, y) = neighbors[expected_random.below(8) as usize];
                                if choice == 1 {
                                    Action::Move(Position::new(x, y))
                                } else {
                                    Action::Create(Position::new(x, y))
                                }
                            }
                            _ => Action::Repair,
                        };
                        let mut actual_random = Random::new(seed);
                        assert_eq!(
                            proposals(&world, &mut actual_random),
                            [Proposal::new(1, expected)],
                            "seed {seed}, empty {empty}, weights {weights:?}, position {position:?}"
                        );
                        assert_eq!(
                            actual_random, expected_random,
                            "only the selected spatial action draws a destination"
                        );
                    }
                    assert_eq!(seen, tickets.map(|tickets| tickets > 0));
                    assert_eq!(
                        world, before,
                        "choice cannot edit properties, integrity, state or occupancy"
                    );
                }
            }
        }
    }

    #[test]
    fn generator_v1_reference_stream() {
        let mut random = Random::new(0);
        assert_eq!(random.next(), 0xe220a8397b1dcdaf);
        assert_eq!(random.next(), 0x6e789e6aa1b965f4);
        assert_eq!(random.next(), 0x06c45d188009454f);
    }

    #[test]
    fn bounded_draws_skip_the_short_remainder_and_handle_boundaries() {
        let bound = (1_u64 << 63) + 1;
        let mut expected_stream = Random::new(0);
        let mut actual = expected_stream.clone();
        let mut rejections = 0;
        for _ in 0..100 {
            let expected = loop {
                let value = expected_stream.next();
                if value >= bound.wrapping_neg() % bound {
                    break value % bound;
                }
                rejections += 1;
            };
            assert_eq!(actual.below(bound), expected);
        }
        assert!(rejections > 0, "exercise actual rejection, not only modulo");
        for bound in [1, 8, u64::MAX] {
            for _ in 0..100 {
                assert!(actual.below(bound) < bound);
            }
        }
    }
}
