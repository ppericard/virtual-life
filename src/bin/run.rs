use std::time::Instant;

use virtual_life::{Simulation, SimulationConfig};

fn main() {
    let steps = std::env::args()
        .nth(1)
        .map(|value| {
            value
                .parse::<u64>()
                .expect("steps must be a positive integer")
        })
        .unwrap_or(100_000);

    let mut simulation = Simulation::new(SimulationConfig::default());
    let started = Instant::now();
    simulation.run(steps);
    let elapsed = started.elapsed();
    let steps_per_second = steps as f64 / elapsed.as_secs_f64();

    println!("step={}", simulation.step_number());
    println!("agents={}", simulation.world().agent_count());
    println!("elapsed={elapsed:?}");
    println!("steps_per_second={steps_per_second:.0}");
}
