//! Shared, deliberately small command-line configuration for both executables.
use crate::{
    engine::Weights,
    experiment::{ExperimentConfig, GENERATOR},
    runner::Config,
};
use std::time::Duration;

pub const OPTIONS: &str = "[--mode demo|autonomous] [--survival wear-repair|random] [--ticks N] [--width N] [--height N] [--occupancy 0..1] [--bundles W,M,C,R;...] [--proportions N,...] [--seed N] [--integrity N] [--upkeep N] [--move-wear N] [--copy-wear N] [--repair N]\nAutonomous defaults to wear-repair: fourth weight is Repair (Remove in random mode). Integrity defaults to 10, upkeep 1, extra move/copy wear 1/2, gross repair 4. Maintenance settings require wear-repair; all use nonnegative u32 integers, integrity must be positive.";

pub struct Launch {
    pub config: Config,
    pub port: u16,
    pub help: bool,
}

pub fn parse(arguments: impl IntoIterator<Item = String>, web: bool) -> Result<Launch, String> {
    let mut config = Config::default();
    let mut experiment = ExperimentConfig::default();
    let mut mode = "demo".to_owned();
    let mut model_options = false;
    let mut maintenance_options = false;
    let mut custom_bundles = false;
    let mut survival = "wear-repair".to_owned();
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
            "--survival" => {
                survival = value;
                model_options = true;
            }
            "--integrity" | "--upkeep" | "--move-wear" | "--copy-wear" | "--repair" => {
                let amount =
                    u32::try_from(integer()?).map_err(|_| "maintenance values must fit u32")?;
                let rules = experiment.maintenance.as_mut().unwrap();
                match argument.as_str() {
                    "--integrity" => rules.maximum = amount,
                    "--upkeep" => rules.upkeep = amount,
                    "--move-wear" => rules.move_wear = amount,
                    "--copy-wear" => rules.copy_wear = amount,
                    _ => rules.repair = amount,
                }
                model_options = true;
                maintenance_options = true;
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
            "--bundles" => {
                experiment.bundles = value
                    .split(';')
                    .map(|bundle| {
                        let values = list(bundle)?;
                        Ok(Weights(values.try_into().map_err(
                            |_| "each bundle needs wait,move,copy and repair/remove weights",
                        )?))
                    })
                    .collect::<Result<_, String>>()?;
                model_options = true;
                custom_bundles = true;
            }
            "--proportions" => {
                experiment.proportions = list(&value)?;
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
    match mode.as_str() {
        "autonomous" => {
            match survival.as_str() {
                "wear-repair" => {}
                "random" if maintenance_options => {
                    return Err("maintenance options require wear-repair survival".into());
                }
                "random" => {
                    experiment.maintenance = None;
                    if !custom_bundles {
                        experiment.bundles = ExperimentConfig::random().bundles;
                    }
                }
                _ => return Err("survival must be wear-repair or random".into()),
            }
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
                .map_err(|_| "weights and proportions must be nonnegative u32 integers".into())
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
        println!(
            "survival={} protocol={} fourth_action={}",
            experiment.survival(),
            experiment.protocol(),
            if experiment.maintenance.is_some() {
                "repair"
            } else {
                "remove"
            }
        );
        if let Some(rules) = experiment.maintenance {
            println!(
                "integrity={} upkeep={} move_wear={} copy_wear={} repair={}; initial and newborn integrity use the maximum",
                rules.maximum, rules.upkeep, rules.move_wear, rules.copy_wear, rules.repair
            );
        }
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
                "group={} weights={:?} proportion={} initial_count={}",
                index, group.weights.0, group.proportion, group.initial_count
            );
        }
    } else {
        println!("Scripted demonstrator (not autonomous behavior).");
    }
    Ok(())
}
