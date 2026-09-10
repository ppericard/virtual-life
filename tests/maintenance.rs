use virtual_life::{
    engine::{
        Action, Agent, FAILURE_LIMIT, FailureReason, Maintenance, Position, Proposal, Weights,
        World,
    },
    experiment::{self, ExperimentConfig, Random},
    launch,
};

fn agent(id: u64, integrity: u32) -> Agent {
    Agent {
        id,
        integrity,
        weights: Weights([2, 4, 1, 3]),
        ..Agent::default()
    }
}
fn p(x: usize, y: usize) -> Position {
    Position::new(x, y)
}
fn world(agents: &[(Position, Agent)]) -> World {
    World::with_maintenance(5, 5, agents, Maintenance::default()).unwrap()
}
fn condition(world: &World, id: u64) -> u32 {
    world
        .cells()
        .iter()
        .flatten()
        .find(|a| a.id == id)
        .unwrap()
        .integrity
}

#[test]
fn move_move_repair_worked_example_cap_and_upkeep_precedes_repair() {
    let mut w = world(&[(p(1, 1), agent(1, 10))]);
    for (action, expected) in [
        (Action::Move(p(2, 1)), 8),
        (Action::Move(p(3, 1)), 6),
        (Action::Repair, 9),
        (Action::Repair, 10),
        (Action::Wait, 9),
    ] {
        w.step(&[Proposal::new(1, action)]).unwrap();
        assert_eq!(condition(&w, 1), expected);
    }
    assert_eq!(w.totals().repairs, 2);
    let mut dying = world(&[(p(1, 1), agent(7, 1))]);
    let events = dying.step(&[Proposal::new(7, Action::Repair)]).unwrap();
    assert_eq!(
        (events.repairs, events.removals, events.failures),
        (0, 1, 1)
    );
    let failure = dying.failures().front().unwrap();
    assert_eq!(
        (
            failure.id,
            failure.tick,
            failure.reason,
            failure.integrity_before,
            failure.upkeep,
            failure.action_wear
        ),
        (7, 1, FailureReason::Upkeep, 1, 1, 0)
    );
    assert_eq!(dying.count(), 0);
}

#[test]
fn dying_intent_makes_no_claim_and_vacated_square_remains_ineligible() {
    let mut w = world(&[
        (p(0, 2), agent(1, 2)),
        (p(2, 2), agent(2, 10)),
        (p(0, 1), agent(3, 10)),
    ]);
    let events = w
        .step(&[
            Proposal::new(1, Action::Move(p(1, 2))),
            Proposal::new(2, Action::Create(p(1, 2))),
            Proposal::new(3, Action::Move(p(0, 2))),
        ])
        .unwrap();
    assert_eq!((events.creations, events.moves, events.failures), (1, 0, 1));
    assert_eq!(w.agent_at(p(1, 2)).unwrap(), Some(agent(4, 10)));
    assert_eq!((condition(&w, 2), condition(&w, 3)), (7, 8));
    assert_eq!(w.agent_at(p(0, 2)).unwrap(), None);
    assert_eq!(w.next_id(), 5);
    assert_eq!(w.failures()[0].reason, FailureReason::MoveWear);
}

#[test]
fn affordable_mixed_claims_and_occupied_destinations_pay_attempt_wear() {
    let mut w = world(&[(p(0, 2), agent(1, 10)), (p(2, 2), agent(2, 10))]);
    let events = w
        .step(&[
            Proposal::new(1, Action::Move(p(1, 2))),
            Proposal::new(2, Action::Create(p(1, 2))),
        ])
        .unwrap();
    assert_eq!((events.moves, events.creations, events.failures), (0, 0, 0));
    assert_eq!((condition(&w, 1), condition(&w, 2), w.next_id()), (8, 7, 3));
    let mut occupied = world(&[(p(0, 2), agent(1, 10)), (p(1, 2), agent(2, 10))]);
    occupied
        .step(&[
            Proposal::new(1, Action::Create(p(1, 2))),
            Proposal::new(2, Action::Move(p(0, 2))),
        ])
        .unwrap();
    assert_eq!(
        (
            condition(&occupied, 1),
            condition(&occupied, 2),
            occupied.next_id()
        ),
        (7, 8, 3)
    );
    let mut exact = world(&[(p(0, 2), agent(1, 3))]);
    let events = exact
        .step(&[Proposal::new(1, Action::Create(p(1, 2)))])
        .unwrap();
    assert_eq!(
        (
            exact.count(),
            exact.next_id(),
            events.creations,
            events.failures
        ),
        (0, 2, 0, 1)
    );
    assert_eq!(exact.failures()[0].reason, FailureReason::CopyWear);
}

#[test]
fn failed_batches_leave_both_buffers_integrity_ids_and_failure_evidence_unchanged() {
    let mut w = world(&[(p(0, 2), agent(1, 10)), (p(2, 2), agent(2, 2))]);
    w.step(&[Proposal::new(1, Action::Create(p(1, 2)))])
        .unwrap();
    for proposals in [
        vec![
            Proposal::new(1, Action::Repair),
            Proposal::new(99, Action::Wait),
        ],
        vec![
            Proposal::new(1, Action::Repair),
            Proposal::new(1, Action::Wait),
        ],
        vec![Proposal::new(1, Action::Move(p(4, 4)))],
        vec![Proposal::new(1, Action::Move(p(5, 0)))],
        vec![Proposal::new(1, Action::Remove)],
        vec![Proposal::new(1, Action::SetValue(4))],
    ] {
        let before = w.clone();
        assert!(w.step(&proposals).is_err());
        assert_eq!(w, before);
    }
    let mut random = World::new(5, 5, &[(p(1, 1), agent(1, 10))]).unwrap();
    let before = random.clone();
    assert!(random.step(&[Proposal::new(1, Action::Repair)]).is_err());
    assert_eq!(random, before);
}

#[test]
fn children_receive_fresh_integrity_but_inherit_properties_and_act_next_tick() {
    let parent = Agent {
        weights: Weights([0, 0, 1, 0]),
        ..agent(71, 8)
    };
    let mut w = world(&[(p(2, 2), parent)]);
    let mut random = Random::new(5);
    let chosen = experiment::proposals(&w, &mut random);
    assert_eq!(chosen.len(), 1);
    w.step(&chosen).unwrap();
    assert_eq!((condition(&w, 71), condition(&w, 72)), (5, 10));
    assert!(
        w.cells()
            .iter()
            .flatten()
            .all(|a| a.weights == parent.weights)
    );
    assert_eq!(w.totals().creations, 1);
    assert_eq!(experiment::proposals(&w, &mut random).len(), 2);
}

#[test]
fn policy_skips_upkeep_failures_but_draws_destination_before_affordability_check() {
    let mover = Agent {
        weights: Weights([0, 1, 0, 0]),
        ..agent(2, 2)
    };
    let w = world(&[(p(0, 0), agent(1, 1)), (p(2, 2), mover)]);
    let without = world(&[(p(2, 2), mover)]);
    let mut first = Random::new(19);
    let mut second = first.clone();
    assert_eq!(
        experiment::proposals(&w, &mut first),
        experiment::proposals(&without, &mut second)
    );
    assert_eq!(first, second);
    let affordable = world(&[(
        p(2, 2),
        Agent {
            integrity: 10,
            ..mover
        },
    )]);
    let mut third = Random::new(19);
    assert_eq!(experiment::proposals(&affordable, &mut third).len(), 1);
    assert_eq!(first, third);
}

#[test]
fn bounded_failure_records_retain_tick_order_and_count_discarded_records() {
    let agents: Vec<_> = (0..400)
        .map(|i| (p(i % 20, i / 20), agent(i as u64 + 1, 1)))
        .collect();
    let mut w = World::with_maintenance(20, 20, &agents, Maintenance::default()).unwrap();
    w.step(&[]).unwrap();
    assert_eq!(
        (
            w.totals().failures,
            w.failures().len(),
            w.discarded_failures()
        ),
        (400, FAILURE_LIMIT, 272)
    );
    assert_eq!(w.failures().front().unwrap().id, 273);
    assert_eq!(w.failures().back().unwrap().id, 400);
    assert!(
        w.failures()
            .iter()
            .all(|f| f.tick == 1 && f.reason == FailureReason::Upkeep)
    );
    let records = w.failures().clone();
    w.step(&[]).unwrap();
    assert_eq!(*w.failures(), records);
}

#[test]
fn full_neighborhood_transfers_copy_to_wait_before_neighbors_fail_upkeep() {
    let agents: Vec<_> = (0..9)
        .map(|index| {
            (
                p(index % 3, index / 3),
                Agent {
                    weights: Weights([0, 0, 1, 0]),
                    ..agent(index as u64 + 1, if index == 0 { 3 } else { 1 })
                },
            )
        })
        .collect();
    let mut w = World::with_maintenance(3, 3, &agents, Maintenance::default()).unwrap();
    let mut random = Random::new(1);
    let chosen = experiment::proposals(&w, &mut random);
    assert_eq!(chosen, [Proposal::new(1, Action::Wait)]);
    let events = w.step(&chosen).unwrap();
    assert_eq!((events.failures, events.creations, w.count()), (8, 0, 1));
    assert_eq!(condition(&w, 1), 2); // No Copy choice, so no unaffordable Copy wear.
    assert_eq!(
        w.agent_at(p(0, 0)).unwrap().unwrap().weights,
        Weights([0, 0, 1, 0])
    );
    assert_eq!(experiment::proposals(&w, &mut random).len(), 1);
}

#[test]
fn crowded_copy_keeps_all_eight_targets_and_draws_before_affordability() {
    let copy = Weights([0, 0, 1, 0]);
    let mut agents = vec![(
        p(0, 0),
        Agent {
            weights: copy,
            ..agent(1, 10)
        },
    )];
    for index in 1..8 {
        agents.push((
            p(index % 3, index / 3),
            Agent {
                weights: copy,
                ..agent(index as u64 + 1, 1)
            },
        ));
    }
    // Only (2,2) is empty; all seven other neighbours fail upkeep this tick.
    let affordable = World::with_maintenance(3, 3, &agents, Maintenance::default()).unwrap();
    agents[0].1.integrity = 3;
    let unaffordable = World::with_maintenance(3, 3, &agents, Maintenance::default()).unwrap();
    let mut targets = std::collections::HashSet::new();
    for seed in 0..1024 {
        let mut first = Random::new(seed);
        let mut second = first.clone();
        let chosen = experiment::proposals(&affordable, &mut first);
        assert_eq!(chosen, experiment::proposals(&unaffordable, &mut second));
        assert_eq!(first, second);
        if let Action::Create(target) = chosen[0].action {
            targets.insert((target.x, target.y));
            let mut succeeds = affordable.clone();
            let mut fails = unaffordable.clone();
            let accepted = succeeds.step(&chosen).unwrap();
            let rejected = fails.step(&chosen).unwrap();
            assert_eq!(accepted.creations, u64::from(target == p(2, 2)));
            assert_eq!(condition(&succeeds, 1), 7);
            assert_eq!(
                (rejected.failures, rejected.creations, fails.count()),
                (8, 0, 0)
            );
            assert_eq!(fails.failures()[0].reason, FailureReason::CopyWear);
            assert!(succeeds.cells().iter().flatten().all(|a| a.weights == copy));
        }
    }
    assert_eq!(
        targets,
        affordable
            .neighbors(p(0, 0))
            .unwrap()
            .into_iter()
            .map(|target| (target.x, target.y))
            .collect()
    );
}

#[test]
fn zero_costs_and_repair_can_sustain_life_and_large_restoration_cannot_overflow() {
    for rules in [
        Maintenance {
            maximum: 1,
            upkeep: 0,
            move_wear: 0,
            copy_wear: 0,
            repair: 0,
        },
        Maintenance {
            maximum: u32::MAX,
            repair: u32::MAX,
            ..Maintenance::default()
        },
    ] {
        let mut w =
            World::with_maintenance(5, 5, &[(p(1, 1), agent(1, rules.maximum))], rules).unwrap();
        for _ in 0..600 {
            w.step(&[Proposal::new(1, Action::Repair)]).unwrap();
        }
        assert_eq!(
            (w.count(), condition(&w, 1), w.totals().failures),
            (1, rules.maximum, 0)
        );
        assert_eq!(w.totals().repairs, if rules.upkeep == 0 { 0 } else { 600 });
    }
    let mut zero_repair = World::with_maintenance(
        5,
        5,
        &[(p(1, 1), agent(1, 10))],
        Maintenance {
            repair: 0,
            ..Maintenance::default()
        },
    )
    .unwrap();
    zero_repair
        .step(&[Proposal::new(1, Action::Repair)])
        .unwrap();
    assert_eq!(
        (condition(&zero_repair, 1), zero_repair.totals().repairs),
        (9, 0)
    );
}

#[test]
fn configuration_rejects_invalid_survival_settings_and_grouping_ignores_integrity() {
    for args in [
        vec!["--mode", "autonomous", "--integrity", "0"],
        vec!["--mode", "autonomous", "--upkeep", "4294967296"],
        vec!["--mode", "autonomous", "--repair", "-1"],
        vec![
            "--mode",
            "autonomous",
            "--survival",
            "random",
            "--repair",
            "0",
        ],
        vec!["--mode", "demo", "--survival", "random"],
        vec!["--mode", "autonomous", "--survival", "unknown"],
    ] {
        assert!(launch::parse(args.into_iter().map(str::to_owned), false).is_err());
    }
    let config = ExperimentConfig {
        width: 3,
        height: 3,
        bundles: vec![Weights([1, 0, 0, 0]); 2],
        proportions: vec![1, 1],
        ..ExperimentConfig::default()
    };
    let (mut w, _, info) = config.initialize().unwrap();
    assert_eq!(info.groups.len(), 1);
    w.step(&[]).unwrap();
    assert_eq!(info.counts(w.cells()), [2]);
    assert!(w.cells().iter().flatten().all(|a| a.integrity == 9));
    for integrity in [0, 11] {
        assert!(
            World::with_maintenance(
                5,
                5,
                &[(p(1, 1), agent(1, integrity))],
                Maintenance::default()
            )
            .is_err()
        );
    }
    assert!(
        World::with_maintenance(
            5,
            5,
            &[],
            Maintenance {
                maximum: 0,
                ..Maintenance::default()
            }
        )
        .is_err()
    );
}

#[test]
fn generated_wear_runs_keep_identities_positive_integrity_and_order_independent_outcomes() {
    use std::collections::HashMap;
    for seed in 0..20 {
        let config = ExperimentConfig {
            width: 8,
            height: 6,
            seed,
            ..ExperimentConfig::default()
        };
        let (mut w, mut random, info) = config.initialize().unwrap();
        let initial = (w.clone(), random.clone());
        assert_eq!(
            config.initialize().map(|(w, r, _)| (w, r)).unwrap(),
            initial
        );
        let mut known = HashMap::new();
        for _ in 0..100 {
            let proposals = experiment::proposals(&w, &mut random);
            let mut reverse = proposals.clone();
            reverse.reverse();
            let mut other = w.clone();
            other.step(&reverse).unwrap();
            let previous = w.count();
            let events = w.step(&proposals).unwrap();
            assert_eq!(w, other);
            assert_eq!(
                w.count() as u64,
                previous as u64 + events.creations - events.removals
            );
            assert_eq!(info.counts(w.cells()).iter().sum::<usize>(), w.count());
            assert_eq!(
                w.totals().failures,
                w.failures().len() as u64 + w.discarded_failures()
            );
            let mut ids = std::collections::HashSet::new();
            for agent in w.cells().iter().flatten() {
                assert!(ids.insert(agent.id));
                assert!((1..=10).contains(&agent.integrity));
                if let Some(previous) = known.insert(agent.id, agent.weights) {
                    assert_eq!(previous, agent.weights);
                }
            }
        }
    }
}
