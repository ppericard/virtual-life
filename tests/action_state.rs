use virtual_life::engine::{Action, ActionState, Agent, Maintenance, Position, Proposal, World};
use virtual_life::experiment::transition_weights;

#[test]
fn every_state_uses_the_same_exact_property_and_starting_neighbour_tickets() {
    use virtual_life::engine::Weights;
    let maximum = u64::from(u32::MAX);
    for (weights, occupied, expected) in [
        ([1, 2, 3, 4], 0, [8, 16, 24, 32]),
        ([1, 2, 3, 4], 1, [11, 16, 21, 32]),
        ([1, 2, 3, 4], 4, [20, 16, 12, 32]),
        ([1, 2, 3, 4], 8, [32, 16, 0, 32]),
        ([0, 0, 1, 0], 8, [8, 0, 0, 0]),
        ([0, 0, 0, 1], 8, [0, 0, 0, 8]),
        ([1, 2, 0, 0], 4, [8, 16, 0, 0]),
        ([u32::MAX; 4], 0, [8 * maximum; 4]),
        (
            [u32::MAX; 4],
            8,
            [16 * maximum, 8 * maximum, 0, 8 * maximum],
        ),
    ] {
        let mut neighbours = [None; 8];
        for (index, cell) in neighbours.iter_mut().take(occupied).enumerate() {
            *cell = Some(Agent {
                id: index as u64 + 1,
                integrity: 1,
                weights: Weights([u32::MAX, 0, 0, 0]),
                ..Agent::default()
            });
        }
        let before = neighbours;
        for state in [
            None,
            Some(ActionState::Wait),
            Some(ActionState::Move),
            Some(ActionState::Copy),
            Some(ActionState::Repair),
        ] {
            for integrity_after_upkeep in [1, 7, u32::MAX] {
                assert_eq!(
                    transition_weights(
                        Agent {
                            weights: Weights(weights),
                            last_action: state,
                            ..Agent::default()
                        },
                        integrity_after_upkeep,
                        u32::MAX,
                        &neighbours
                    ),
                    expected
                );
                assert_eq!(neighbours, before);
            }
        }
    }
}

#[test]
fn selected_state_survives_rejected_claims_and_moves_but_children_start_unacted() {
    let agent = |id| Agent {
        id,
        integrity: 10,
        ..Agent::default()
    };
    let mut world = World::with_maintenance(
        5,
        5,
        &[
            (Position::new(0, 2), agent(1)),
            (Position::new(2, 2), agent(2)),
            (Position::new(4, 2), agent(3)),
        ],
        Maintenance::default(),
    )
    .unwrap();
    assert!(
        world
            .cells()
            .iter()
            .flatten()
            .all(|a| a.last_action.is_none())
    );
    let result = world
        .step(&[
            Proposal::new(1, Action::Move(Position::new(1, 2))),
            Proposal::new(2, Action::Create(Position::new(1, 2))),
            Proposal::new(3, Action::Repair),
        ])
        .unwrap();
    assert_eq!((result.moves, result.creations), (0, 0));
    for (id, integrity, state) in [
        (1, 8, ActionState::Move),
        (2, 7, ActionState::Copy),
        (3, 10, ActionState::Repair),
    ] {
        let a = world.cells().iter().flatten().find(|a| a.id == id).unwrap();
        assert_eq!((a.integrity, a.last_action), (integrity, Some(state)));
    }
    let result = world
        .step(&[
            Proposal::new(1, Action::Move(Position::new(1, 2))),
            Proposal::new(2, Action::Create(Position::new(3, 2))),
            // Missing proposal for 3 means Wait, replacing its previous Repair.
        ])
        .unwrap();
    assert_eq!((result.moves, result.creations), (1, 1));
    assert_eq!(
        world
            .agent_at(Position::new(1, 2))
            .unwrap()
            .unwrap()
            .last_action,
        Some(ActionState::Move)
    );
    let child = world.agent_at(Position::new(3, 2)).unwrap().unwrap();
    assert_eq!(
        (child.id, child.integrity, child.last_action),
        (4, 10, None)
    );
    assert_eq!(
        world
            .agent_at(Position::new(4, 2))
            .unwrap()
            .unwrap()
            .last_action,
        Some(ActionState::Wait)
    );
    world.step(&[]).unwrap();
    assert_eq!(
        world
            .agent_at(Position::new(3, 2))
            .unwrap()
            .unwrap()
            .last_action,
        Some(ActionState::Wait)
    );
}

#[test]
fn occupied_targets_record_selection_and_invalid_batches_preserve_all_memory() {
    let a = Agent {
        id: 1,
        integrity: 10,
        last_action: Some(ActionState::Repair),
        ..Agent::default()
    };
    let b = Agent { id: 2, ..a };
    let mut world = World::with_maintenance(
        5,
        5,
        &[(Position::new(1, 1), a), (Position::new(2, 1), b)],
        Maintenance::default(),
    )
    .unwrap();
    world
        .step(&[
            Proposal::new(1, Action::Move(Position::new(2, 1))),
            Proposal::new(2, Action::Create(Position::new(1, 1))),
        ])
        .unwrap();
    assert_eq!(
        world
            .agent_at(Position::new(1, 1))
            .unwrap()
            .unwrap()
            .last_action,
        Some(ActionState::Move)
    );
    assert_eq!(
        world
            .agent_at(Position::new(2, 1))
            .unwrap()
            .unwrap()
            .last_action,
        Some(ActionState::Copy)
    );
    assert_eq!((world.totals().moves, world.totals().creations), (0, 0));
    for invalid in [
        vec![
            Proposal::new(1, Action::Wait),
            Proposal::new(1, Action::Repair),
        ],
        vec![
            Proposal::new(1, Action::Repair),
            Proposal::new(99, Action::Wait),
        ],
        vec![Proposal::new(1, Action::Move(Position::new(3, 3)))],
        vec![Proposal::new(1, Action::Create(Position::new(5, 0)))],
        vec![Proposal::new(1, Action::SetValue(4))],
        vec![Proposal::new(1, Action::Remove)],
    ] {
        let before = world.clone();
        assert!(world.step(&invalid).is_err());
        assert_eq!(world, before);
    }
}

#[test]
fn upkeep_failure_has_no_selection_or_draw_and_legacy_actions_leave_memory_unset() {
    use virtual_life::experiment::{Random, proposals};
    for state in [
        None,
        Some(ActionState::Wait),
        Some(ActionState::Move),
        Some(ActionState::Copy),
        Some(ActionState::Repair),
    ] {
        let mut world = World::with_maintenance(
            3,
            3,
            &[(
                Position::new(1, 1),
                Agent {
                    id: 1,
                    integrity: 1,
                    last_action: state,
                    ..Agent::default()
                },
            )],
            Maintenance::default(),
        )
        .unwrap();
        let mut random = Random::new(42);
        let before = world.clone();
        let chosen = proposals(&world, &mut random);
        assert!(chosen.is_empty());
        assert_eq!(random, Random::new(42));
        assert_eq!(world, before);
        world.step(&chosen).unwrap();
        assert_eq!(world.count(), 0);
        assert_eq!(
            world.failures().front().unwrap().reason,
            virtual_life::engine::FailureReason::Upkeep
        );
    }
    let mut demo = virtual_life::demo::initial_world();
    for _ in 0..8 {
        demo.step(&virtual_life::demo::proposals(&demo)).unwrap();
        assert!(
            demo.cells()
                .iter()
                .flatten()
                .all(|a| a.last_action.is_none())
        );
    }
}
