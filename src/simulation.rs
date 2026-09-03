use rand::{rngs::StdRng, RngExt, SeedableRng};

use crate::{Agent, Position, Snapshot, World};

const MOORE_OFFSETS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Parameters required to reproduce a bootstrap run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimulationConfig {
    pub width: usize,
    pub height: usize,
    pub agent_count: usize,
    pub property_count: usize,
    pub seed: u64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            width: 80,
            height: 50,
            agent_count: 400,
            property_count: 1,
            seed: 1,
        }
    }
}

/// Deliberately simple executable reference model.
///
/// Each turn uses random-sequential scheduling: the set of agents present at
/// the start of the turn is shuffled, then every agent moves exactly once to a
/// uniformly chosen Moore-neighbor tile. Stacking is allowed, so movement has
/// no collision rule. Agent properties are unchanged.
///
/// This bootstrap rule exists to validate architecture and semantics. It is not
/// claimed to be an interesting emergence experiment.
pub struct Simulation {
    world: World,
    step: u64,
    rng: StdRng,
    initial_config: SimulationConfig,
}

impl Simulation {
    pub fn new(config: SimulationConfig) -> Self {
        assert!(config.width > 0 && config.height > 0);
        let mut rng = StdRng::seed_from_u64(config.seed);
        let mut world = World::new(config.width, config.height);

        for _ in 0..config.agent_count {
            let position = Position {
                x: rng.random_range(0..config.width),
                y: rng.random_range(0..config.height),
            };
            let properties = (0..config.property_count)
                .map(|_| rng.random_range(0..=255))
                .collect();
            world.add_agent(Agent::new(properties, position));
        }

        Self {
            world,
            step: 0,
            rng,
            initial_config: config,
        }
    }

    pub fn step(&mut self) {
        let mut ids: Vec<_> = self.world.agent_ids().collect();
        fisher_yates_shuffle(&mut ids, &mut self.rng);

        for id in ids {
            let Some(position) = self.world.agent(id).map(|agent| agent.position) else {
                continue;
            };
            let (dx, dy) = MOORE_OFFSETS[self.rng.random_range(0..MOORE_OFFSETS.len())];
            let destination = self.world.offset(position, dx, dy);
            self.world.move_agent(id, destination);
        }

        self.step += 1;
    }

    pub fn run(&mut self, steps: u64) {
        for _ in 0..steps {
            self.step();
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.initial_config);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot::from_world(self.step, &self.world)
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn step_number(&self) -> u64 {
        self.step
    }
}

fn fisher_yates_shuffle<T>(values: &mut [T], rng: &mut impl rand::Rng) {
    for i in (1..values.len()).rev() {
        let j = rng.random_range(0..=i);
        values.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_seeds_produce_identical_trajectories() {
        let config = SimulationConfig::default();
        let mut left = Simulation::new(config);
        let mut right = Simulation::new(config);

        for _ in 0..100 {
            left.step();
            right.step();
            assert_eq!(left.snapshot(), right.snapshot());
        }
    }

    #[test]
    fn bootstrap_rule_preserves_agent_count_and_properties() {
        let mut simulation = Simulation::new(SimulationConfig {
            width: 8,
            height: 8,
            agent_count: 32,
            property_count: 3,
            seed: 42,
        });
        let before = simulation.snapshot();
        simulation.run(50);
        let after = simulation.snapshot();

        assert_eq!(before.agents.len(), after.agents.len());
        for (before_agent, after_agent) in before.agents.iter().zip(after.agents.iter()) {
            assert_eq!(before_agent.id, after_agent.id);
            assert_eq!(before_agent.properties, after_agent.properties);
        }
    }
}
