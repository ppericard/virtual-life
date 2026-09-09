use std::process::ExitCode;
use virtual_life::{demo, launch, runner::Worker};

fn run() -> Result<(), String> {
    let launch = launch::parse(std::env::args().skip(1), false)?;
    if launch.help {
        println!(
            "Usage: headless {}\nDemo defaults to 5 ticks; autonomous defaults to 500. Quote bundles containing semicolons.",
            launch::OPTIONS
        );
        return Ok(());
    }
    launch::describe(&launch.config)?;
    let info = launch
        .config
        .experiment
        .as_ref()
        .map(|config| config.initialize().map(|(_, _, info)| info))
        .transpose()?;
    let ticks = launch.config.ticks;
    let world = Worker::spawn(launch.config, None)?.join()?;
    if info.is_none() && ticks > demo::END_TICK {
        println!("Ticks after 5 were all-wait transitions.");
    }
    println!("tick={} count={}", world.tick(), world.count());
    let events = world.totals();
    println!(
        "accepted: moves={} creations={} removals={} value_changes={}",
        events.moves, events.creations, events.removals, events.value_changes
    );
    if let Some(info) = &info {
        for (index, count) in info.counts(world.cells()).iter().enumerate() {
            println!("group={index} count={count}");
        }
    }
    for (index, cell) in world.cells().iter().enumerate() {
        if let Some(agent) = cell {
            print!(
                "id={} value={} at=({},{})",
                agent.id,
                agent.value,
                index % world.width(),
                index / world.width()
            );
            if info.is_some() {
                print!(" weights={:?}", agent.weights.0);
            }
            println!();
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
