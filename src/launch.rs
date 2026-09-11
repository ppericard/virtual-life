//! Shared, deliberately small command-line configuration for both executables.
use crate::{
    experiment::{ExperimentConfig, GENERATOR},
    runner::Config,
};
use std::time::Duration;

pub const OPTIONS: &str = "[--mode autonomous|demo] [--ticks N]
[--width N] [--height N] [--occupancy 0..1] [--seed N]
[--automaton-preset mixed|movement-runs|repair-cycles|copy-bursts|wait-cycles]
[--automata INITIAL[~COPY_DAMAGE_GAIN,MOVE_CROWDING_GAIN]:ROW/ROW/ROW/ROW;...]
[--proportions N,...] [--integrity N] [--upkeep N]
[--crowding-threshold 0..8] [--crowding-upkeep N] [--move-wear N] [--copy-wear N] [--repair N]
Defaults: autonomous, mixed FSMs, integrity 10, upkeep 1, crowding threshold 5,
extra crowding upkeep 1, move/copy wear 1/2, gross repair 4.
Each ROW has Wait,Move,Copy,Repair weights. Quote graphs containing semicolons.
Maintenance values are nonnegative u32 integers; integrity must be positive.
Crowding threshold 0 applies everywhere; crowding upkeep 0 disables the surcharge.";

pub struct Launch {
    pub config: Config,
    pub port: u16,
    pub help: bool,
}

pub fn parse(arguments: impl IntoIterator<Item = String>, web: bool) -> Result<Launch, String> {
    let mut config = Config::default();
    let mut experiment = ExperimentConfig::default();
    let mut mode = "autonomous".to_owned();
    let mut model_options = false;
    let mut custom_automata = false;
    let mut custom_proportions = false;
    let mut ticks = None;
    let mut interval = None;
    let mut port = 7878;
    let mut arguments = arguments.into_iter();
    while let Some(argument) = arguments.next() {
        if argument == "--help" || argument == "-h" {
            return Ok(Launch {
                config,
                port,
                help: true,
            });
        }
        if argument == "--running" && web {
            config.start_paused = false;
            continue;
        }
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value for {argument}; use --help"))?;
        let integer = || {
            value
                .parse::<u64>()
                .map_err(|_| format!("invalid nonnegative integer for {argument}"))
        };
        match argument.as_str() {
            "--mode" => mode = value,
            "--integrity"
            | "--upkeep"
            | "--crowding-threshold"
            | "--crowding-upkeep"
            | "--move-wear"
            | "--copy-wear"
            | "--repair" => {
                let amount =
                    u32::try_from(integer()?).map_err(|_| "maintenance values must fit u32")?;
                let rules = &mut experiment.maintenance;
                match argument.as_str() {
                    "--integrity" => rules.maximum = amount,
                    "--upkeep" => rules.upkeep = amount,
                    "--crowding-threshold" => rules.crowding_threshold = amount,
                    "--crowding-upkeep" => rules.crowding_upkeep = amount,
                    "--move-wear" => rules.move_wear = amount,
                    "--copy-wear" => rules.copy_wear = amount,
                    _ => rules.repair = amount,
                }
                model_options = true;
            }
            "--ticks" => ticks = Some(integer()?),
            "--width" | "--height" => {
                let dimension = usize::try_from(integer()?).map_err(|_| "dimension overflow")?;
                if argument == "--width" {
                    experiment.width = dimension;
                } else {
                    experiment.height = dimension;
                }
                model_options = true;
            }
            "--occupancy" => {
                let (whole, fraction) = value.split_once('.').unwrap_or((&value, ""));
                if !matches!(whole, "0" | "1")
                    || fraction.len() > 6
                    || !fraction.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err("occupancy must be 0..1 with up to six decimal places".into());
                }
                let scaled = if fraction.is_empty() {
                    0
                } else {
                    fraction.parse::<u32>().unwrap() * 10_u32.pow(6 - fraction.len() as u32)
                };
                experiment.occupancy = whole.parse::<u32>().unwrap() * 1_000_000 + scaled;
                model_options = true;
            }
            "--seed" => {
                experiment.seed = integer()?;
                model_options = true;
            }
            "--automata" => {
                if custom_automata {
                    return Err("provide automata only once".into());
                }
                experiment.automata = value
                    .split(';')
                    .map(crate::automaton::Automaton::parse)
                    .collect::<Result<_, _>>()?;
                model_options = true;
                custom_automata = true;
            }
            "--automaton-preset" => {
                if custom_automata {
                    return Err("provide automata only once".into());
                }
                let machines = if value == "mixed" {
                    crate::automaton::PRESETS
                        .iter()
                        .map(|preset| preset.machine)
                        .collect::<Vec<_>>()
                } else {
                    vec![
                        crate::automaton::find(&value)
                            .ok_or("unknown automaton preset")?
                            .machine,
                    ]
                };
                experiment.automata = machines;
                model_options = true;
                custom_automata = true;
            }
            "--proportions" => {
                experiment.proportions = list(&value)?;
                custom_proportions = true;
                model_options = true;
            }
            "--port" if web => port = u16::try_from(integer()?).map_err(|_| "invalid port")?,
            "--sample-every" if web => config.sample_every = integer()?,
            "--tick-ms" if web => {
                let ms = integer()?;
                if ms > 60_000 {
                    return Err("tick-ms must be between 0 and 60000".into());
                }
                interval = Some(Duration::from_millis(ms));
            }
            _ => return Err(format!("unknown argument {argument}; use --help")),
        }
    }
    if !custom_proportions {
        experiment.proportions = vec![1; experiment.automata.len()];
    }
    match mode.as_str() {
        "autonomous" => {
            experiment.initialize()?;
            config.experiment = Some(experiment);
            config.ticks = 500;
            config.tick_interval = Duration::from_millis(100);
        }
        "demo" if model_options => return Err("model options require --mode autonomous".into()),
        "demo" => {}
        _ => return Err("mode must be demo or autonomous".into()),
    }
    if let Some(ticks) = ticks {
        config.ticks = ticks;
    }
    if let Some(interval) = interval {
        config.tick_interval = interval;
    }
    if config.sample_every == 0 {
        return Err("sample-every must be positive".into());
    }
    if !web {
        config.start_paused = false;
        config.tick_interval = Duration::ZERO;
    }
    Ok(Launch {
        config,
        port,
        help: false,
    })
}

fn list(value: &str) -> Result<Vec<u32>, String> {
    value
        .split(',')
        .map(|part| {
            part.parse()
                .map_err(|_| "proportions must be nonnegative u32 integers".into())
        })
        .collect()
}

pub fn describe(config: &Config) -> Result<(), String> {
    if let Some(experiment) = &config.experiment {
        let (world, _, info) = experiment.initialize()?;
        println!(
            "Autonomous experiment: seed={} generator={GENERATOR}; crate={}",
            experiment.seed,
            env!("CARGO_PKG_VERSION")
        );
        println!("protocol=unit-action automaton v1");
        let rules = experiment.maintenance;
        println!(
            "integrity={} upkeep={} crowding_threshold={} crowding_upkeep={} move_wear={} copy_wear={} repair={}; initial and newborn integrity use the maximum",
            rules.maximum,
            rules.upkeep,
            rules.crowding_threshold,
            rules.crowding_upkeep,
            rules.move_wear,
            rules.copy_wear,
            rules.repair
        );
        println!(
            "width={} height={} occupancy={}.{:06} ticks={} initial_count={}",
            experiment.width,
            experiment.height,
            experiment.occupancy / 1_000_000,
            experiment.occupancy % 1_000_000,
            config.ticks,
            world.count()
        );
        for (index, group) in info.groups.iter().enumerate() {
            println!(
                "group={index} automaton={} proportion={} initial_count={}",
                group.automaton.specification(),
                group.proportion,
                group.initial_count
            );
        }
    } else {
        println!("Scripted demonstrator (not autonomous behavior).");
    }
    Ok(())
}
