//! Local release timing, not an FPS or CI performance guarantee.
use std::{hint::black_box, time::Instant};
use virtual_life::{
    automaton::PRESETS,
    experiment::{ExperimentConfig, proposals},
};

fn main() {
    for (width, height, repeats) in [(64, 48, 500), (512, 512, 10)] {
        for graph in [false, true] {
            let config = ExperimentConfig {
                width,
                height,
                occupancy: 1_000_000,
                automata: graph.then(|| PRESETS.iter().map(|p| p.machine).collect()),
                ..Default::default()
            };
            let (world, mut random, _) = config.initialize().unwrap();
            for _ in 0..10 {
                black_box(proposals(&world, &mut random));
            }
            let mut trials = Vec::new();
            for _ in 0..7 {
                let start = Instant::now();
                for _ in 0..repeats {
                    black_box(proposals(black_box(&world), &mut random));
                }
                trials.push(start.elapsed().as_secs_f64() * 1000.0 / f64::from(repeats));
            }
            trials.sort_by(f64::total_cmp);
            println!(
                "{} agents; {}: median {:.3} ms/choice pass (7 trials, unchanged full grid)",
                world.count(),
                if graph { "FSM" } else { "flat" },
                trials[3]
            );
        }
    }
    println!(
        "Agent size: {} bytes; includes optional inline graph in both cases",
        std::mem::size_of::<virtual_life::engine::Agent>()
    );
}
