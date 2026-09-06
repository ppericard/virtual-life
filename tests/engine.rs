use Action::{Create, Move, Remove, SetValue, Wait};
use virtual_life::{
    demo,
    engine::{Action, Agent, Events, Position, Proposal, World},
};

fn p(x: usize, y: usize) -> Position {
    Position::new(x, y)
}
fn a(id: u64, value: i64) -> Agent {
    Agent { id, value }
}
fn action(id: u64, action: Action) -> Proposal {
    Proposal::new(id, action)
}

fn assert_agents(world: &World, expected: &[(Position, Agent)]) {
    assert_eq!(world.count(), expected.len());
    for &(position, agent) in expected {
        assert_eq!(world.agent_at(position).unwrap(), Some(agent));
    }
    let mut ids: Vec<_> = world.cells().iter().flatten().map(|a| a.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        expected.len(),
        "each ID occupies exactly one square"
    );
}

#[test]
fn complete_worked_example_matches_every_tick_and_event_total() {
    let mut world = demo::initial_world();
    let expected = [
        vec![
            (p(0, 2), a(1, 10)),
            (p(2, 2), a(2, 20)),
            (p(4, 2), a(3, 31)),
        ],
        vec![
            (p(2, 2), a(2, 20)),
            (p(3, 2), a(4, 20)),
            (p(4, 2), a(3, 31)),
        ],
        vec![
            (p(0, 2), a(3, 31)),
            (p(2, 2), a(2, 21)),
            (p(3, 3), a(4, 20)),
        ],
        vec![
            (p(0, 2), a(3, 31)),
            (p(1, 2), a(5, 21)),
            (p(2, 2), a(2, 21)),
            (p(3, 3), a(4, 20)),
        ],
        vec![(p(1, 2), a(5, 21)), (p(2, 2), a(2, 21))],
    ];
    let events = [
        Events {
            value_changes: 1,
            ..Events::default()
        },
        Events {
            creations: 1,
            removals: 1,
            ..Events::default()
        },
        Events {
            moves: 2,
            value_changes: 1,
            ..Events::default()
        },
        Events {
            creations: 1,
            ..Events::default()
        },
        Events {
            removals: 2,
            ..Events::default()
        },
    ];
    for (tick, (agents, accepted)) in expected.iter().zip(events).enumerate() {
        let proposals = demo::proposals(&world);
        let before_count = world.count();
        assert_eq!(world.step(&proposals).unwrap(), accepted);
        assert_eq!(world.tick(), tick as u64 + 1);
        assert_agents(&world, agents);
        assert_eq!(
            world.count() as u64 + accepted.removals,
            before_count as u64 + accepted.creations
        );
    }
    assert_eq!(
        world.totals(),
        Events {
            moves: 2,
            creations: 2,
            removals: 3,
            value_changes: 2
        }
    );
    assert_eq!(world.next_id(), 6);
}

#[test]
fn wrapped_neighbors_are_eight_distinct_squares_even_at_minimum_dimensions() {
    for (width, height) in [(3, 3), (3, 7), (8, 3), (5, 5)] {
        let world = World::new(width, height, &[]).unwrap();
        for y in 0..height {
            for x in 0..width {
                let neighbors = world.neighbors(p(x, y)).unwrap();
                for (i, neighbor) in neighbors.iter().enumerate() {
                    assert!(neighbor.x < width && neighbor.y < height);
                    assert_ne!(*neighbor, p(x, y));
                    assert!(!neighbors[..i].contains(neighbor));
                }
            }
        }
    }
    let mut world = World::new(5, 7, &[(p(0, 0), a(1, 10))]).unwrap();
    world.step(&[action(1, Move(p(4, 6)))]).unwrap();
    assert_agents(&world, &[(p(4, 6), a(1, 10))]);
    world.step(&[action(1, Move(p(0, 0)))]).unwrap();
    assert_agents(&world, &[(p(0, 0), a(1, 10))]);
}

#[test]
fn invalid_initial_dimensions_positions_occupancy_and_identity_are_rejected() {
    for (width, height) in [
        (0, 5),
        (5, 0),
        (2, 3),
        (3, 2),
        (usize::MAX, 3),
        (usize::MAX / 3, 3),
    ] {
        assert!(World::new(width, height, &[]).is_err());
    }
    for agents in [
        vec![(p(5, 0), a(1, 10))],
        vec![(p(0, 5), a(1, 10))],
        vec![(p(0, 0), a(1, 10)), (p(0, 0), a(2, 20))],
        vec![(p(0, 0), a(1, 10)), (p(1, 0), a(1, 20))],
        vec![(p(0, 0), a(u64::MAX, 10))],
    ] {
        assert!(World::new(5, 5, &agents).is_err());
    }
    let world = demo::initial_world();
    assert!(world.agent_at(p(usize::MAX, 0)).is_err());
    assert!(world.neighbors(p(0, usize::MAX)).is_err());
}

#[test]
fn every_invalid_batch_is_atomic_including_scratch_buffer_and_ids() {
    let mut world = demo::initial_world();
    world.step(&[action(2, Create(p(3, 2)))]).unwrap();
    let before = world.clone();
    for invalid in [
        vec![action(2, SetValue(99)), action(999, Wait)],
        vec![action(1, Remove), action(1, Move(p(1, 2)))],
        vec![action(1, Move(p(1, 2))), action(1, Move(p(4, 2)))],
        vec![action(1, Wait), action(1, Wait)],
        vec![action(1, Move(p(5, 2)))],
        vec![action(1, Create(p(0, 5)))],
        vec![action(1, Move(p(2, 0)))],
        vec![action(1, Create(p(0, 2)))],
        vec![action(1, Create(p(1, 2))), action(5, SetValue(88))],
    ] {
        assert!(world.step(&invalid).is_err(), "{invalid:?}");
        assert_eq!(world, before);
    }
    world.step(&[action(1, Create(p(1, 2)))]).unwrap();
    assert_eq!(world.agent_at(p(1, 2)).unwrap(), Some(a(5, 10)));
}

#[test]
fn id_exhaustion_is_atomic_and_rejected_creations_spend_no_ids() {
    let mut world = World::new(5, 5, &[(p(0, 0), a(u64::MAX - 1, 10))]).unwrap();
    let before = world.clone();
    assert!(
        world
            .step(&[action(u64::MAX - 1, Create(p(1, 0)))])
            .is_err()
    );
    assert_eq!(world, before);
    world.step(&[action(u64::MAX - 1, SetValue(11))]).unwrap();
    assert_eq!(world.next_id(), u64::MAX);
}

#[test]
fn moves_do_not_repeat_and_newborns_first_act_on_the_following_tick() {
    let mut world = World::new(5, 5, &[(p(0, 0), a(1, 7))]).unwrap();
    world.step(&[action(1, Move(p(1, 0)))]).unwrap();
    assert_agents(&world, &[(p(1, 0), a(1, 7))]);
    assert_eq!(world.totals().moves, 1);
    world.step(&[action(1, Create(p(2, 0)))]).unwrap();
    assert_agents(&world, &[(p(1, 0), a(1, 7)), (p(2, 0), a(2, 7))]);
    world
        .step(&[action(2, Move(p(3, 0))), action(1, SetValue(9))])
        .unwrap();
    assert_agents(&world, &[(p(1, 0), a(1, 9)), (p(3, 0), a(2, 7))]);
    world.step(&[action(2, Remove)]).unwrap();
    world.step(&[action(1, Create(p(2, 0)))]).unwrap();
    assert_eq!(world.agent_at(p(2, 0)).unwrap(), Some(a(3, 9)));
}

#[test]
fn all_move_and_creation_claimants_lose_a_destination_conflict() {
    for left in [Move(p(1, 2)), Create(p(1, 2))] {
        for right in [Move(p(1, 2)), Create(p(1, 2))] {
            let mut world = demo::initial_world();
            let cells = world.cells().to_vec();
            assert_eq!(
                world.step(&[action(1, left), action(2, right)]).unwrap(),
                Events::default()
            );
            assert_eq!(world.cells(), cells);
            assert_eq!(world.next_id(), 4);
        }
    }
}

#[test]
fn occupied_and_just_vacated_targets_are_unavailable_until_the_next_tick() {
    for leaving in [Remove, Move(p(2, 0))] {
        for arriving in [Move(p(1, 0)), Create(p(1, 0))] {
            let mut world = World::new(5, 5, &[(p(0, 0), a(1, 10)), (p(1, 0), a(2, 20))]).unwrap();
            world
                .step(&[action(2, leaving), action(1, arriving)])
                .unwrap();
            assert_eq!(world.agent_at(p(0, 0)).unwrap(), Some(a(1, 10)));
            assert_eq!(world.agent_at(p(1, 0)).unwrap(), None);
            assert_eq!(world.next_id(), 3);
            world.step(&[action(1, arriving)]).unwrap();
            assert!(world.agent_at(p(1, 0)).unwrap().is_some());
        }
    }
    let mut world = World::new(5, 5, &[(p(0, 0), a(1, 10)), (p(1, 0), a(2, 20))]).unwrap();
    let cells = world.cells().to_vec();
    world
        .step(&[action(1, Move(p(1, 0))), action(2, Move(p(0, 0)))])
        .unwrap();
    assert_eq!(world.cells(), cells);
    assert_eq!(world.totals(), Events::default());
}

fn permutations(items: &mut [Proposal], start: usize, visit: &mut impl FnMut(&[Proposal])) {
    if start == items.len() {
        visit(items);
        return;
    }
    for i in start..items.len() {
        items.swap(start, i);
        permutations(items, start + 1, visit);
        items.swap(start, i);
    }
}

#[test]
fn proposal_order_never_changes_complete_state_or_row_major_creation_ids() {
    let world = World::new(
        5,
        5,
        &[
            (p(0, 0), a(9, 90)),
            (p(2, 0), a(2, 20)),
            (p(4, 0), a(3, 30)),
            (p(0, 3), a(1, 10)),
            (p(2, 3), a(4, 40)),
        ],
    )
    .unwrap();
    let mut proposals = vec![
        action(9, Create(p(0, 1))),
        action(2, Move(p(3, 0))),
        action(3, Create(p(3, 0))),
        action(1, Create(p(0, 4))),
        action(4, Remove),
    ];
    let mut expected = world.clone();
    expected.step(&proposals).unwrap();
    assert_eq!(expected.agent_at(p(0, 1)).unwrap(), Some(a(10, 90)));
    assert_eq!(expected.agent_at(p(0, 4)).unwrap(), Some(a(11, 10)));
    assert_eq!(expected.next_id(), 12);
    permutations(&mut proposals, 0, &mut |order| {
        let mut actual = world.clone();
        actual.step(order).unwrap();
        assert_eq!(actual, expected);
    });
}

#[test]
fn inactivity_never_removes_agents_and_extra_demo_ticks_only_advance_time() {
    let mut world = demo::initial_world();
    world
        .step(&[action(1, Wait), action(2, SetValue(20))])
        .unwrap();
    assert_eq!(world.totals(), Events::default());
    assert_eq!(world.count(), 3);
    let mut world = demo::initial_world();
    for _ in 0..5 {
        world.step(&demo::proposals(&world)).unwrap();
    }
    let cells = world.cells().to_vec();
    let totals = world.totals();
    let next_id = world.next_id();
    for _ in 0..10 {
        world.step(&demo::proposals(&world)).unwrap();
    }
    assert_eq!(world.tick(), 15);
    assert_eq!(world.cells(), cells);
    assert_eq!(world.totals(), totals);
    assert_eq!(world.next_id(), next_id);
    let mut empty = World::new(3, 3, &[]).unwrap();
    empty.step(&[]).unwrap();
    assert_eq!(empty.tick(), 1);
    assert_eq!(empty.count(), 0);
}
