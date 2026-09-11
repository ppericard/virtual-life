//! Local release timing, not an FPS or CI performance guarantee.
use std::{hint::black_box, time::Instant};
use virtual_life::experiment::{ExperimentConfig, proposals};

fn main() {
    for (width, height, repeats) in [(64, 48, 500), (512, 512, 10)] {
        let config = ExperimentConfig {
            width,
            height,
            occupancy: 1_000_000,
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
            "{} agents; FSM: median {:.3} ms/choice pass (7 trials, unchanged full grid)",
            world.count(),
            trials[3]
        );
    }
    println!(
        "Agent size: {} bytes; includes the inherited graph",
        std::mem::size_of::<virtual_life::engine::Agent>()
    );
}
