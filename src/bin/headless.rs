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
        "accepted: moves={} creations={} removals={} value_changes={} repairs={} failures={}",
        events.moves,
        events.creations,
        events.removals,
        events.value_changes,
        events.repairs,
        events.failures
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
            if let Some(rules) = world.maintenance() {
                print!(" integrity={}/{}", agent.integrity, rules.maximum);
            }
            println!();
        }
    }
    if world.maintenance().is_some() {
        println!(
            "recent_failures={} discarded_failures={} (latest 128 engine outcomes; independent of observation)",
            world.failures().len(),
            world.discarded_failures()
        );
        for failure in world.failures() {
            println!(
                "failure: id={} tick={} at=({},{}) reason={} integrity_before={} upkeep={} action_wear={}",
                failure.id,
                failure.tick,
                failure.position.x,
                failure.position.y,
                failure.reason.label(),
                failure.integrity_before,
                failure.upkeep,
                failure.action_wear
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
