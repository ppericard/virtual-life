use std::{process::ExitCode, time::Duration};
use virtual_life::{
    demo,
    runner::{Config, Worker},
};

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let ticks = match arguments.as_slice() {
        [] => demo::END_TICK,
        [help] if help == "--help" || help == "-h" => {
            println!(
                "Usage: headless [--ticks N]\nDefaults to 5. After tick 5, all agents wait; only the tick advances."
            );
            return Ok(());
        }
        [flag, value] if flag == "--ticks" => value
            .parse::<u64>()
            .map_err(|_| "ticks must be a nonnegative integer")?,
        _ => return Err("Usage: headless [--ticks N]".into()),
    };
    let world = Worker::spawn(
        Config {
            ticks,
            tick_interval: Duration::ZERO,
            start_paused: false,
            ..Config::default()
        },
        None,
    )?
    .join()?;
    println!("Scripted demonstrator: requested run complete (not autonomous behavior).");
    if ticks > demo::END_TICK {
        println!("Ticks after 5 were all-wait transitions.");
    }
    println!("tick={} count={}", world.tick(), world.count());
    let events = world.totals();
    println!(
        "accepted: moves={} creations={} removals={} value_changes={}",
        events.moves, events.creations, events.removals, events.value_changes
    );
    for (index, cell) in world.cells().iter().enumerate() {
        if let Some(agent) = cell {
            println!(
                "id={} value={} at=({},{})",
                agent.id,
                agent.value,
                index % world.width(),
                index / world.width()
            );
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
