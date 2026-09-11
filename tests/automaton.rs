use virtual_life::{
    automaton::{Automaton, PRESETS},
    engine::{Action, ActionState, Agent, Maintenance, Position, Proposal, Weights, World},
    experiment::{self, ExperimentConfig},
    launch,
};

#[test]
fn a_sparse_unit_action_graph_controls_the_next_tick() {
    let launch = launch::parse(
        [
            "--mode",
            "autonomous",
            "--width",
            "3",
            "--height",
            "3",
            "--occupancy",
            "0.12",
            "--proportions",
            "1",
            "--automata",
            "wait:0,1,0,0/0,0,0,1/0,0,0,1/1,0,0,0",
        ]
        .map(str::to_owned),
        false,
    )
    .expect("a four-state transition graph should be accepted");
    let (mut world, mut random, _) = launch.config.experiment.unwrap().initialize().unwrap();
    assert_eq!(world.count(), 1);
    assert!(
        world
            .cells()
            .iter()
            .flatten()
            .next()
            .unwrap()
            .last_action
            .is_none()
    );
    for expected in [
        ActionState::Move,
        ActionState::Repair,
        ActionState::Wait,
        ActionState::Move,
        ActionState::Repair,
        ActionState::Wait,
    ] {
        world
            .step(&experiment::proposals(&world, &mut random))
            .unwrap();
        let agent = world.cells().iter().flatten().next().unwrap();
        assert_eq!(agent.last_action, Some(expected));
        assert_eq!(
            world.count(),
            1,
            "a missing Copy edge cannot create a child"
        );
    }
}

#[test]
fn damage_and_crowding_use_exact_current_context_tickets() {
    let machine = Automaton::parse("repair:0,0,0,1/0,0,0,1/0,0,0,1/0,3,1,6").unwrap();
    let agent = Agent {
        automaton: Some(machine),
        integrity: 10,
        ..Agent::default()
    };
    for (integrity, expected) in [
        (7, [0, 24000, 8000, 14400]),
        (8, [0, 24000, 8000, 9600]),
        (9, [0, 24000, 8000, 4800]),
        (10, [0, 24000, 8000, 0]),
    ] {
        assert_eq!(
            experiment::transition_weights(agent, integrity, 10, &[None; 8]),
            expected
        );
    }
    let mut neighbours = [None; 8];
    neighbours[..4].fill(Some(Agent::default()));
    assert_eq!(
        experiment::transition_weights(agent, 8, 10, &neighbours),
        [4000, 24000, 4000, 9600]
    );
    assert_eq!(
        experiment::transition_weights(
            Agent {
                last_action: Some(ActionState::Move),
                ..agent
            },
            9,
            10,
            &[None; 8]
        ),
        [0, 0, 0, 800]
    );
    assert_eq!(
        experiment::transition_weights(
            Agent {
                last_action: Some(ActionState::Move),
                ..agent
            },
            10,
            10,
            &[None; 8]
        ),
        [1, 0, 0, 0],
        "healthy Repair-only rows wait"
    );
    // No multiplication may wrap for the largest supported integrity.
    assert_eq!(
        experiment::transition_weights(agent, u32::MAX, u32::MAX, &[None; 8]),
        [0, 24000, 8000, 0]
    );
    let responsive = Agent {
        automaton: Some(Automaton {
            copy_damage_gain: 2,
            move_crowding_gain: 2,
            ..machine
        }),
        ..agent
    };
    assert_eq!(
        experiment::transition_weights(responsive, 5, 10, &neighbours),
        [8000, 48000, 8000, 24000]
    );
    assert_eq!(Automaton::parse(&machine.specification()).unwrap(), machine);
}

#[test]
fn graph_and_responses_define_inherited_identity_and_newborn_start() {
    let machine = Automaton::parse("wait:0,1,0,0/0,0,0,1/0,0,0,1/1,0,0,0").unwrap();
    let parent = Agent {
        id: 1,
        integrity: 10,
        automaton: Some(machine),
        ..Agent::default()
    };
    let mut world = World::with_maintenance(
        5,
        5,
        &[(Position::new(0, 2), parent)],
        Maintenance::default(),
    )
    .unwrap();
    world
        .step(&[Proposal::new(1, Action::Create(Position::new(1, 2)))])
        .unwrap();
    let child = world.agent_at(Position::new(1, 2)).unwrap().unwrap();
    assert_eq!(
        (
            child.id,
            child.integrity,
            child.last_action,
            child.automaton
        ),
        (2, 10, None, Some(machine))
    );
    let mut random = experiment::Random::new(1);
    let proposals = experiment::proposals(&world, &mut random);
    assert_eq!(
        proposals[0].action,
        Action::Repair,
        "parent uses the Copy source row"
    );
    assert!(
        matches!(proposals[1].action, Action::Move(_)),
        "newborn uses inherited initial Wait row"
    );
    let before = world.clone();
    assert!(world.step(&[proposals[0], proposals[0]]).is_err());
    assert_eq!(
        world, before,
        "invalid proposals preserve all graph and state data"
    );

    let a = PRESETS[0].machine;
    let b = Automaton {
        rows: [a.rows[0], Weights([0, 0, 0, 1]), a.rows[2], a.rows[3]],
        ..a
    };
    let c = Automaton {
        copy_damage_gain: 3,
        ..a
    };
    let config = ExperimentConfig {
        width: 5,
        height: 5,
        occupancy: 400_000,
        automata: Some(vec![a, b, a, c]),
        proportions: vec![1; 4],
        ..ExperimentConfig::default()
    };
    let (world, _, info) = config.initialize().unwrap();
    assert_eq!(info.groups.len(), 3);
    assert_eq!(
        info.groups.iter().map(|g| g.proportion).collect::<Vec<_>>(),
        [2, 1, 1]
    );
    assert_eq!(info.counts(world.cells()), [5, 3, 2]);
}

#[test]
fn invalid_graphs_and_incompatible_modes_are_rejected() {
    for text in [
        "wait:0,0,0,0/1,0,0,0/1,0,0,0/1,0,0,0",
        "wait:1,0,0,0",
        "other:1,0,0,0/1,0,0,0/1,0,0,0/1,0,0,0",
        "wait@101:1,0,0,0/1,0,0,0/1,0,0,0/1,0,0,0",
        "wait@80:1,0,0,0/1,0,0,0/1,0,0,0/0,0,0,1",
    ] {
        assert!(Automaton::parse(text).is_err(), "{text}");
    }
    for args in [
        vec!["--automaton-preset", "mixed"],
        vec![
            "--mode",
            "autonomous",
            "--survival",
            "random",
            "--automaton-preset",
            "mixed",
        ],
        vec![
            "--mode",
            "autonomous",
            "--automaton-preset",
            "mixed",
            "--bundles",
            "1,1,1,1",
        ],
        vec![
            "--mode",
            "autonomous",
            "--automaton-preset",
            "mixed",
            "--automaton-preset",
            "mixed",
        ],
    ] {
        assert!(launch::parse(args.into_iter().map(str::to_owned), false).is_err());
    }
    for preset in &PRESETS {
        preset.machine.validate().unwrap();
    }
    for args in [
        vec![
            "--mode",
            "autonomous",
            "--proportions",
            "3",
            "--automaton-preset",
            "repair-cycles",
        ],
        vec![
            "--mode",
            "autonomous",
            "--automaton-preset",
            "repair-cycles",
            "--proportions",
            "3",
        ],
    ] {
        assert_eq!(
            launch::parse(args.into_iter().map(str::to_owned), false)
                .unwrap()
                .config
                .experiment
                .unwrap()
                .proportions,
            [3]
        );
    }
}

#[test]
fn extreme_parameters_stay_bounded_and_damage_response_is_monotonic() {
    let machine = Automaton {
        initial: ActionState::Wait,
        rows: [Weights([u32::MAX; 4]); 4],
        copy_damage_gain: 255,
        move_crowding_gain: 255,
    };
    let agent = Agent {
        automaton: Some(machine),
        ..Agent::default()
    };
    for occupied in 0..=8 {
        let mut neighbours = [None; 8];
        neighbours[..occupied].fill(Some(agent));
        let mut previous = 0;
        for damage in 0..=1000 {
            let tickets = experiment::transition_weights(agent, 1000 - damage, 1000, &neighbours);
            let total: u64 = tickets.iter().sum();
            assert!(total > 0 && total < (1 << 54));
            assert!(tickets[3] >= previous);
            previous = tickets[3];
            assert_eq!(tickets[2] == 0, occupied == 8);
        }
    }
    assert!(experiment::transition_weights(agent, u32::MAX - 1, u32::MAX, &[None; 8])[3] > 0);
    for preset in &PRESETS {
        assert_eq!(
            Automaton::parse(&preset.machine.specification()).unwrap(),
            preset.machine
        );
    }
}

#[test]
fn blocked_move_advances_the_graph_and_seeded_populations_preserve_invariants() {
    let machine = Automaton::parse("wait:0,1,0,0/0,0,0,1/0,0,0,1/1,0,0,0").unwrap();
    let (mut world, mut random, _) = ExperimentConfig {
        width: 3,
        height: 3,
        occupancy: 1_000_000,
        automata: Some(vec![machine]),
        proportions: vec![1],
        ..Default::default()
    }
    .initialize()
    .unwrap();
    world
        .step(&experiment::proposals(&world, &mut random))
        .unwrap();
    assert_eq!(world.totals().moves, 0);
    assert!(
        world
            .cells()
            .iter()
            .flatten()
            .all(|a| a.last_action == Some(ActionState::Move))
    );
    world
        .step(&experiment::proposals(&world, &mut random))
        .unwrap();
    assert!(
        world
            .cells()
            .iter()
            .flatten()
            .all(|a| a.last_action == Some(ActionState::Repair))
    );
    for seed in 0..16 {
        let config = ExperimentConfig {
            seed,
            width: 13,
            height: 9,
            automata: Some(PRESETS.iter().map(|p| p.machine).collect()),
            ..Default::default()
        };
        let (mut a, mut ra, info) = config.initialize().unwrap();
        let (mut b, mut rb, _) = config.initialize().unwrap();
        for _ in 0..60 {
            let pa = experiment::proposals(&a, &mut ra);
            let mut pb = experiment::proposals(&b, &mut rb);
            pb.reverse();
            a.step(&pa).unwrap();
            b.step(&pb).unwrap();
            assert_eq!(a, b);
            let agents: Vec<_> = a.cells().iter().flatten().collect();
            let ids: std::collections::HashSet<_> = agents.iter().map(|a| a.id).collect();
            assert_eq!(ids.len(), a.count());
            assert_eq!(info.counts(a.cells()).iter().sum::<usize>(), a.count());
            assert!(agents.iter().all(|a| a.integrity > 0
                && a.integrity <= 10
                && info.groups.iter().any(|g| g.matches(a))));
        }
    }
}
