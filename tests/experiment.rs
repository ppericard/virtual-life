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
        ..ExperimentConfig::random()
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
        bundles: vec![
            Weights([1, 0, 0, 0]),
            Weights([1, 0, 0, 0]),
            Weights([2, 0, 0, 0]),
            Weights([0, 0, 0, 1]),
        ],
        proportions: vec![1, 2, 3, 0],
        seed: 1,
        maintenance: None,
    };
    let (world, _, info) = config.initialize().unwrap();
    assert_eq!(info.groups.len(), 3); // Proportional, nonidentical tuples remain distinct.
    assert_eq!(
        info.groups.iter().map(|g| g.proportion).collect::<Vec<_>>(),
        [3, 3, 0]
    );
    assert_eq!(info.counts(world.cells()), [5, 4, 0]);
    let (equivalent, _, _) = ExperimentConfig {
        bundles: vec![
            Weights([1, 0, 0, 0]),
            Weights([2, 0, 0, 0]),
            Weights([0, 0, 0, 1]),
        ],
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
            bundles: vec![],
            proportions: vec![],
            ..settings()
        },
        ExperimentConfig {
            bundles: vec![Weights([0; 4])],
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
            bundles: (1..=9).map(|n| Weights([n, 0, 0, 0])).collect(),
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
        bundles: vec![Weights([u32::MAX; 4])],
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
        vec!["--width", "8"],
    ] {
        assert!(launch::parse(arguments.into_iter().map(str::to_owned), false).is_err());
    }
}

#[test]
fn copy_inherits_weights_and_stable_ids_newborns_first_choose_next_tick() {
    let weights = Weights([0, 0, 1, 0]);
    let parent = Agent {
        id: 71,
        weights,
        ..Agent::default()
    };
    let mut world = World::new(5, 5, &[(Position::new(2, 2), parent)]).unwrap();
    let mut random = Random::new(5);
    let proposals = experiment::proposals(&world, &mut random);
    assert_eq!(proposals.len(), 1);
    world.step(&proposals).unwrap();
    assert_eq!(world.count(), 2);
    let identities: Vec<_> = world
        .cells()
        .iter()
        .flatten()
        .map(|a| (a.id, a.weights))
        .collect();
    assert!(identities.contains(&(71, weights)) && identities.contains(&(72, weights)));
    let next = experiment::proposals(&world, &mut random);
    assert_eq!(next.len(), 2);
    assert!(next.iter().any(|p| p.actor == 72));
}

#[test]
fn mixed_property_claims_fail_together_and_invalid_batches_reject_atomically() {
    let a = Agent {
        id: 1,
        weights: Weights([0, 1, 0, 0]),
        ..Agent::default()
    };
    let b = Agent {
        id: 2,
        weights: Weights([0, 0, 1, 0]),
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
fn forced_wait_remove_full_and_empty_populations_have_no_hidden_rules() {
    for (weights, expected) in [
        (Weights([1, 0, 0, 0]), 48),
        (Weights([0, 0, 0, 1]), 0),
        (Weights([0, 1, 0, 0]), 48),
        (Weights([0, 0, 1, 0]), 48),
    ] {
        let config = ExperimentConfig {
            occupancy: 1_000_000,
            bundles: vec![weights],
            proportions: vec![1],
            ..settings()
        };
        let (mut world, mut random, info) = config.initialize().unwrap();
        let initial = world.cells().to_vec();
        for _ in 0..50 {
            world
                .step(&experiment::proposals(&world, &mut random))
                .unwrap();
        }
        assert_eq!(world.count(), expected);
        assert_eq!(info.counts(world.cells()), [expected]);
        if expected > 0 {
            assert_eq!(world.cells(), initial);
        }
    }
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
            .map(|a| (a.id, a.weights))
            .collect();
        for _ in 0..60 {
            let proposals = experiment::proposals(&world, &mut random);
            assert_eq!(proposals.len(), world.count());
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
                if let Some(previous) = known.insert(agent.id, agent.weights) {
                    assert_eq!(previous, agent.weights);
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
            maintenance: Some(virtual_life::engine::Maintenance {
                crowding_upkeep: 0,
                ..Default::default()
            }),
            ..ExperimentConfig::default()
        });
        let _ = finished.send(());
    });
    completion
        .recv_timeout(Duration::from_secs(20))
        .expect("observation checks exceeded deadlock guard");
}

#[test]
fn wear_crowding_v2_seed_one_records_exact_tick_ten_world_and_failures() {
    let config = ExperimentConfig {
        width: 8,
        height: 6,
        maintenance: Some(virtual_life::engine::Maintenance {
            crowding_upkeep: 0,
            ..Default::default()
        }),
        ..ExperimentConfig::default()
    };
    assert_eq!(config.protocol(), "wear-repair crowding v3");
    let (mut world, mut random, info) = config.initialize().unwrap();
    for _ in 0..10 {
        world
            .step(&experiment::proposals(&world, &mut random))
            .unwrap();
    }
    let mut expected = vec![None; 48];
    for (id, x, y, group, integrity) in [
        (23, 3, 0, 0, 7),
        (22, 6, 0, 1, 9),
        (26, 7, 0, 1, 8),
        (5, 2, 1, 1, 4),
        (19, 0, 2, 0, 1),
        (24, 1, 2, 3, 9),
        (4, 2, 2, 0, 9),
        (16, 3, 2, 1, 10),
        (27, 4, 2, 2, 8),
        (25, 5, 2, 2, 6),
        (1, 3, 3, 0, 5),
        (15, 4, 3, 1, 10),
        (20, 5, 3, 1, 2),
        (9, 6, 3, 2, 10),
        (10, 1, 4, 2, 2),
        (8, 4, 4, 1, 8),
        (21, 6, 4, 1, 4),
        (3, 7, 4, 2, 6),
        (28, 1, 5, 2, 10),
        (6, 5, 5, 1, 7),
        (18, 7, 5, 1, 6),
    ] {
        expected[y * 8 + x] = Some(Agent {
            id,
            value: 0,
            weights: config.bundles[group],
            integrity,
        });
    }
    assert_eq!(world.cells(), expected);
    assert_eq!(world.tick(), 10);
    assert_eq!(world.next_id(), 29);
    assert_eq!(info.counts(world.cells()), [4, 10, 6, 1]);
    assert_eq!(
        world.totals(),
        virtual_life::engine::Events {
            moves: 34,
            creations: 14,
            removals: 7,
            value_changes: 0,
            repairs: 54,
            failures: 7,
        }
    );
    assert_eq!(world.discarded_failures(), 0);
    use virtual_life::engine::FailureReason::{CopyWear, MoveWear, Upkeep};
    let failures: Vec<_> = world
        .failures()
        .iter()
        .map(|f| {
            (
                f.id,
                f.tick,
                f.position.x,
                f.position.y,
                f.reason,
                f.integrity_before,
                f.upkeep,
                f.action_wear,
            )
        })
        .collect();
    assert_eq!(
        failures,
        [
            (12, 5, 7, 2, MoveWear, 2, 1, 1),
            (7, 5, 0, 3, MoveWear, 2, 1, 1),
            (11, 5, 4, 5, MoveWear, 2, 1, 1),
            (14, 7, 0, 4, Upkeep, 1, 1, 0),
            (13, 8, 0, 0, Upkeep, 1, 1, 0),
            (2, 8, 3, 5, Upkeep, 1, 1, 0),
            (17, 10, 3, 1, CopyWear, 2, 1, 2),
        ]
    );
}

#[test]
fn wear_crowding_v3_seed_one_records_exact_tick_ten_world_and_failure_costs() {
    use virtual_life::engine::FailureReason::{CopyWear, MoveWear, Upkeep};
    use virtual_life::engine::{Events, Failure};
    let config = ExperimentConfig {
        width: 8,
        height: 6,
        ..ExperimentConfig::default()
    };
    assert_eq!(config.protocol(), "wear-repair crowding v3");
    let (mut world, mut random, info) = config.initialize().unwrap();
    for _ in 0..10 {
        world
            .step(&experiment::proposals(&world, &mut random))
            .unwrap();
    }
    let mut expected = vec![None; 48];
    for (id, x, y, group, integrity) in [
        (23, 3, 0, 0, 7),
        (6, 5, 0, 1, 6),
        (5, 2, 1, 1, 6),
        (16, 3, 1, 1, 4),
        (27, 7, 1, 1, 9),
        (24, 0, 2, 3, 3),
        (20, 4, 2, 1, 10),
        (26, 0, 3, 2, 10),
        (25, 2, 3, 1, 10),
        (9, 6, 3, 2, 8),
        (10, 1, 4, 2, 10),
        (8, 3, 4, 1, 7),
        (21, 6, 4, 1, 10),
        (3, 7, 4, 2, 7),
        (22, 6, 5, 1, 4),
    ] {
        expected[y * 8 + x] = Some(Agent {
            id,
            value: 0,
            weights: config.bundles[group],
            integrity,
        });
    }
    assert_eq!(world.cells(), expected);
    assert_eq!(
        (world.tick(), world.next_id(), world.discarded_failures()),
        (10, 28, 0)
    );
    assert_eq!(info.counts(world.cells()), [1, 9, 4, 1]);
    assert_eq!(
        world.totals(),
        Events {
            moves: 31,
            creations: 13,
            removals: 12,
            value_changes: 0,
            repairs: 54,
            failures: 12
        }
    );
    let expected_failures: Vec<_> = [
        (12, 5, 7, 2, MoveWear, 2, 2, 0, 1, 1),
        (7, 5, 0, 3, MoveWear, 2, 2, 0, 1, 1),
        (11, 5, 4, 5, MoveWear, 2, 3, 0, 1, 1),
        (14, 7, 0, 4, Upkeep, 1, 2, 0, 1, 0),
        (13, 8, 0, 0, Upkeep, 1, 2, 0, 1, 0),
        (17, 8, 3, 1, Upkeep, 2, 5, 1, 2, 0),
        (2, 8, 3, 5, Upkeep, 1, 3, 0, 1, 0),
        (4, 9, 2, 2, Upkeep, 1, 5, 1, 2, 0),
        (19, 10, 1, 3, Upkeep, 1, 5, 1, 2, 0),
        (15, 10, 4, 3, CopyWear, 3, 3, 0, 1, 2),
        (1, 10, 2, 4, MoveWear, 2, 4, 0, 1, 1),
        (18, 10, 7, 5, MoveWear, 2, 3, 0, 1, 1),
    ]
    .into_iter()
    .map(
        |(
            id,
            tick,
            x,
            y,
            reason,
            integrity_before,
            occupied_neighbors,
            crowding_upkeep,
            upkeep,
            action_wear,
        )| Failure {
            id,
            tick,
            position: Position::new(x, y),
            reason,
            integrity_before,
            occupied_neighbors,
            base_upkeep: 1,
            crowding_upkeep,
            upkeep,
            action_wear,
        },
    )
    .collect();
    assert_eq!(
        world.failures().iter().copied().collect::<Vec<_>>(),
        expected_failures
    );
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
