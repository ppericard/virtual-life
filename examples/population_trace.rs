//! CSV observations of real autonomous transitions for comparing preset candidates.
//! Uses the normal CLI options, with no pacing; samples every 10 ticks and at the end.
use std::io::{self, Write};
use virtual_life::{experiment::proposals, launch};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let launch = launch::parse(std::env::args().skip(1), false)?;
    if launch.help {
        println!("Population trace: {}", launch::OPTIONS);
        return Ok(());
    }
    let experiment = launch
        .config
        .experiment
        .ok_or("population_trace requires autonomous mode")?;
    let (mut world, mut random, info) = experiment.initialize()?;
    let mut output = io::BufWriter::new(io::stdout().lock());
    write!(
        output,
        "tick,population,agent_ticks,moves,births,deaths,repairs"
    )?;
    for group in 0..info.groups.len() {
        write!(output, ",group_{}", group + 1)?;
    }
    writeln!(output)?;
    // Count starting occupants, including agents that fail upkeep in this tick.
    // Event deltas / exposure deltas give per-agent-per-tick rates for any window.
    let mut agent_ticks = 0_u128;
    loop {
        if world.tick().is_multiple_of(10) || world.tick() == launch.config.ticks {
            let events = world.totals();
            write!(
                output,
                "{},{},{agent_ticks},{},{},{},{}",
                world.tick(),
                world.count(),
                events.moves,
                events.creations,
                events.removals,
                events.repairs
            )?;
            for count in info.counts(world.cells()) {
                write!(output, ",{count}")?;
            }
            writeln!(output)?;
        }
        if world.tick() == launch.config.ticks {
            break;
        }
        agent_ticks += world.count() as u128;
        let actions = proposals(&world, &mut random);
        world.step(&actions)?;
    }
    output.flush()?;
    Ok(())
}
