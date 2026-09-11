mod common;
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc,
    thread,
    time::Duration,
};
use virtual_life::{
    engine::{Action, Agent, Position, Proposal, Weights, World},
    experiment::{self, ExperimentConfig, Random},
    launch,
    runner::{Config, Snapshot, Status, Worker},
};

fn settings() -> ExperimentConfig {
    ExperimentConfig {
        width: 8,
        height: 6,
        ..ExperimentConfig::default()
    }
}

#[test]
fn exact_initial_counts_row_major_ids_and_seeded_placement() {
    let config = settings();
    let (world, random, info) = config.initialize().unwrap();
    assert_eq!(world.count(), 14);
    assert_eq!(
        info.groups
            .iter()
            .map(|g| g.initial_count)
            .collect::<Vec<_>>(),
        [4, 4, 3, 3]
    );
    assert_eq!(
        world
            .cells()
            .iter()
            .flatten()
            .map(|a| a.id)
            .collect::<Vec<_>>(),
        (1..=14).collect::<Vec<_>>()
    );
    assert_eq!(info.counts(world.cells()), [4, 4, 3, 3]);
    assert_eq!(config.initialize().unwrap(), (world.clone(), random, info));
    assert_ne!(
        ExperimentConfig { seed: 2, ..config }
            .initialize()
            .unwrap()
            .0,
        world
    );
}

#[test]
fn exact_tuple_identity_aggregates_proportions_and_preserves_zero_groups() {
    let config = ExperimentConfig {
        width: 3,
        height: 3,
        occupancy: 1_000_000,
        automata: vec![
            Weights([1, 0, 0, 0]),
            Weights([1, 0, 0, 0]),
            Weights([2, 0, 0, 0]),
            Weights([0, 0, 0, 1]),
        ]
        .into_iter()
        .map(common::machine)
        .collect(),
        proportions: vec![1, 2, 3, 0],
        seed: 1,
        maintenance: Default::default(),
    };
    let (world, _, info) = config.initialize().unwrap();
    assert_eq!(info.groups.len(), 3); // Proportional, nonidentical tuples remain distinct.
    assert_eq!(
        info.groups.iter().map(|g| g.proportion).collect::<Vec<_>>(),
        [3, 3, 0]
    );
    assert_eq!(info.counts(world.cells()), [5, 4, 0]);
    let (equivalent, _, _) = ExperimentConfig {
        automata: vec![
            Weights([1, 0, 0, 0]),
            Weights([2, 0, 0, 0]),
            Weights([0, 0, 0, 1]),
        ]
        .into_iter()
        .map(common::machine)
        .collect(),
        proportions: vec![3, 3, 0],
        ..config
    }
    .initialize()
    .unwrap();
    assert_eq!(world, equivalent);
}

#[test]
fn validates_dimensions_weights_proportions_and_fraction_boundaries() {
    for config in [
        ExperimentConfig {
            width: 2,
            ..settings()
        },
        ExperimentConfig {
            width: usize::MAX,
            ..settings()
        },
        ExperimentConfig {
            occupancy: 1_000_001,
            ..settings()
        },
        ExperimentConfig {
            automata: vec![].into_iter().map(common::machine).collect(),
            proportions: vec![],
            ..settings()
        },
        ExperimentConfig {
            automata: vec![Weights([0; 4])]
                .into_iter()
                .map(common::machine)
                .collect(),
            proportions: vec![1],
            ..settings()
        },
        ExperimentConfig {
            proportions: vec![0; 4],
            ..settings()
        },
        ExperimentConfig {
            proportions: vec![1; 3],
            ..settings()
        },
    ] {
        assert!(config.initialize().is_err(), "{config:?}");
    }
    assert!(
        ExperimentConfig {
            automata: (1..=9)
                .map(|n| Weights([n, 0, 0, 0]))
                .map(common::machine)
                .collect(),
            proportions: vec![1; 9],
            ..settings()
        }
        .initialize()
        .is_err()
    );
    for occupancy in [0, 1, 999_999, 1_000_000] {
        let config = ExperimentConfig {
            occupancy,
            ..settings()
        };
        let (world, _, info) = config.initialize().unwrap();
        assert_eq!(
            world.count(),
            (48_u64 * u64::from(occupancy) / 1_000_000) as usize
        );
        assert_eq!(
            info.counts(world.cells()).iter().sum::<usize>(),
            world.count()
        );
    }
    let maximum_weights = ExperimentConfig {
        automata: vec![Weights([u32::MAX; 4])]
            .into_iter()
            .map(common::machine)
            .collect(),
        proportions: vec![u32::MAX],
        ..settings()
    };
    let (world, mut random, _) = maximum_weights.initialize().unwrap();
    assert_eq!(
        experiment::proposals(&world, &mut random).len(),
        world.count()
    );
    for arguments in [
        vec!["--mode", "autonomous", "--occupancy", "NaN"],
        vec!["--mode", "autonomous", "--occupancy", "0.1234567"],
        vec!["--mode", "autonomous", "--bundles", "1,2,3"],
        vec![
            "--mode",
            "autonomous",
            "--bundles",
            "0,0,0,0",
            "--proportions",
            "1",
        ],
        vec!["--mode", "autonomous", "--proportions", "0,0,0,0"],
        vec!["--mode", "demo", "--width", "8"],
    ] {
        assert!(launch::parse(arguments.into_iter().map(str::to_owned), false).is_err());
    }
}

#[test]
fn copy_inherits_graph_and_stable_ids_newborns_first_choose_next_tick() {
    let weights = Weights([0, 0, 1, 0]);
    let parent = Agent {
        id: 71,
        integrity: 10,
        automaton: common::machine(weights),
        ..Agent::default()
    };
    let mut world =
        World::with_maintenance(5, 5, &[(Position::new(2, 2), parent)], Default::default())
            .unwrap();
    let mut random = Random::new(5);
    let proposals = experiment::proposals(&world, &mut random);
    assert_eq!(proposals.len(), 1);
    world.step(&proposals).unwrap();
    assert_eq!(world.count(), 2);
    let identities: Vec<_> = world
        .cells()
        .iter()
        .flatten()
        .map(|a| (a.id, a.automaton))
        .collect();
    assert!(
        identities.contains(&(71, common::machine(weights)))
            && identities.contains(&(72, common::machine(weights)))
    );
    let next = experiment::proposals(&world, &mut random);
    assert_eq!(next.len(), 2);
    assert!(next.iter().any(|p| p.actor == 72));
}

#[test]
fn mixed_property_claims_fail_together_and_invalid_batches_reject_atomically() {
    let a = Agent {
        id: 1,
        automaton: common::machine(Weights([0, 1, 0, 0])),
        ..Agent::default()
    };
    let b = Agent {
        id: 2,
        automaton: common::machine(Weights([0, 0, 1, 0])),
        ..Agent::default()
    };
    let mut world =
        World::new(5, 5, &[(Position::new(0, 2), a), (Position::new(2, 2), b)]).unwrap();
    let cells = world.cells().to_vec();
    let events = world
        .step(&[
            Proposal::new(1, Action::Move(Position::new(1, 2))),
            Proposal::new(2, Action::Create(Position::new(1, 2))),
        ])
        .unwrap();
    assert_eq!(events, Default::default());
    assert_eq!(world.cells(), cells);
    let before = world.clone();
    assert!(
        world
            .step(&[
                Proposal::new(1, Action::Remove),
                Proposal::new(99, Action::Wait)
            ])
            .is_err()
    );
    assert_eq!(before, world);
}

#[test]
fn empty_population_stays_empty_without_hidden_births() {
    let (mut world, mut random, info) = ExperimentConfig {
        occupancy: 0,
        ..settings()
    }
    .initialize()
    .unwrap();
    for _ in 0..501 {
        world
            .step(&experiment::proposals(&world, &mut random))
            .unwrap();
    }
    assert_eq!(world.count(), 0);
    assert_eq!(world.tick(), 501);
    assert_eq!(info.counts(world.cells()), [0; 4]);
}

#[test]
fn autonomous_zero_tick_request_returns_the_exact_initial_world() {
    let (done, finished) = mpsc::channel();
    thread::spawn(move || {
        let configuration = settings();
        let initial = configuration.initialize().unwrap().0;
        let actual = Worker::spawn(
            Config {
                ticks: 0,
                experiment: Some(configuration),
                ..Config::default()
            },
            None,
        )
        .unwrap()
        .join()
        .unwrap();
        assert_eq!(actual, initial);
        done.send(()).unwrap();
    });
    finished
        .recv_timeout(Duration::from_secs(5))
        .expect("zero-tick lifecycle exceeded deadlock guard");
}

#[test]
fn generated_runs_preserve_identity_inheritance_counts_and_permutation_independence() {
    for seed in 0..20 {
        let (mut world, mut random, info) = ExperimentConfig { seed, ..settings() }
            .initialize()
            .unwrap();
        let mut known: HashMap<_, _> = world
            .cells()
            .iter()
            .flatten()
            .map(|a| (a.id, a.automaton))
            .collect();
        for _ in 0..60 {
            let proposals = experiment::proposals(&world, &mut random);
            let survivors = world
                .cells()
                .iter()
                .enumerate()
                .filter(|(_, cell)| cell.is_some())
                .filter(|(index, cell)| {
                    let cost = virtual_life::engine::upkeep_at(
                        world.width(),
                        world.height(),
                        world.cells(),
                        Position::new(index % world.width(), index / world.width()),
                        world.maintenance().unwrap(),
                    )
                    .unwrap();
                    u64::from(cell.unwrap().integrity) > cost.effective_upkeep
                })
                .count();
            assert_eq!(proposals.len(), survivors);
            let mut reverse = proposals.clone();
            reverse.reverse();
            let mut other = world.clone();
            other.step(&reverse).unwrap();
            let before = world.count();
            let events = world.step(&proposals).unwrap();
            assert_eq!(world, other);
            assert_eq!(
                world.count() as i64,
                before as i64 + events.creations as i64 - events.removals as i64
            );
            let mut ids = HashSet::new();
            for agent in world.cells().iter().flatten() {
                assert!(ids.insert(agent.id));
                if let Some(previous) = known.insert(agent.id, agent.automaton) {
                    assert_eq!(previous, agent.automaton);
                }
            }
            assert_eq!(
                info.counts(world.cells()).iter().sum::<usize>(),
                world.count()
            );
        }
    }
}

#[test]
fn fixed_tick_results_ignore_observation_rate_backpressure_and_disconnect() {
    // Keep joins and panic-time Worker::drop off the test thread. A broken
    // blocking publisher must fail the guard, never hang the suite.
    let (finished, completion) = mpsc::channel();
    thread::spawn(move || {
        check_observation_independence(settings());
        check_observation_independence(ExperimentConfig {
            width: 8,
            height: 6,
            ..ExperimentConfig::default()
        });
        check_observation_independence(ExperimentConfig {
            width: 8,
            height: 6,
            maintenance: virtual_life::engine::Maintenance {
                crowding_upkeep: 0,
                ..Default::default()
            },
            ..ExperimentConfig::default()
        });
        // Guaranteed survivors make the action-memory comparison non-vacuous.
        check_observation_independence(ExperimentConfig {
            width: 5,
            height: 5,
            maintenance: virtual_life::engine::Maintenance {
                upkeep: 0,
                crowding_upkeep: 0,
                move_wear: 0,
                copy_wear: 0,
                ..Default::default()
            },
            ..ExperimentConfig::default()
        });
        check_observation_independence(ExperimentConfig {
            automata: virtual_life::automaton::PRESETS
                .iter()
                .map(|p| p.machine)
                .collect(),
            maintenance: virtual_life::engine::Maintenance {
                upkeep: 0,
                crowding_upkeep: 0,
                move_wear: 0,
                copy_wear: 0,
                ..Default::default()
            },
            ..Default::default()
        });
        let _ = finished.send(());
    });
    completion
        .recv_timeout(Duration::from_secs(20))
        .expect("observation checks exceeded deadlock guard");
}

fn check_observation_independence(configuration: ExperimentConfig) {
    let base = Config {
        ticks: 240,
        start_paused: false,
        tick_interval: Duration::ZERO,
        experiment: Some(configuration),
        ..Config::default()
    };
    let expected = Worker::spawn(base.clone(), None).unwrap().join().unwrap();
    if expected
        .maintenance()
        .is_some_and(|rules| rules.upkeep == 0 && rules.crowding_upkeep == 0)
    {
        assert!(
            expected
                .cells()
                .iter()
                .flatten()
                .any(|a| a.last_action.is_some())
        );
    }
    for cadence in [1, 3, 97, 1000] {
        for connected in [true, false] {
            let (send, receive) = mpsc::sync_channel::<Snapshot>(1);
            let worker = Worker::spawn(
                Config {
                    sample_every: cadence,
                    ..base.clone()
                },
                Some(send),
            )
            .unwrap();
            let collector = if connected {
                Some(thread::spawn(move || {
                    let mut final_sample = None;
                    for sample in receive {
                        if sample.status == Status::Completed {
                            final_sample = Some(sample);
                        }
                    }
                    final_sample.unwrap()
                }))
            } else {
                // A saturated receiver reads nothing until the independent completion barrier.
                let completed = worker.wait_for_completion(Duration::from_secs(10));
                drop(receive);
                completed.unwrap();
                None
            };
            assert_eq!(worker.join().unwrap(), expected);
            if let Some(collector) = collector {
                let sample = collector.join().unwrap();
                assert_eq!(sample.tick, 240);
                assert_eq!(sample.cells, expected.cells());
                assert_eq!(sample.totals, expected.totals());
                assert_eq!(
                    sample.failures,
                    expected.failures().iter().copied().collect::<Vec<_>>()
                );
                assert_eq!(sample.discarded_failures, expected.discarded_failures());
                assert_eq!(
                    sample
                        .experiment
                        .unwrap()
                        .counts(&sample.cells)
                        .iter()
                        .sum::<usize>(),
                    sample.count
                );
            }
        }
        let (send, receive) = mpsc::sync_channel(1);
        drop(receive);
        assert_eq!(
            Worker::spawn(
                Config {
                    sample_every: cadence,
                    ..base.clone()
                },
                Some(send)
            )
            .unwrap()
            .join()
            .unwrap(),
            expected
        );
    }
}
