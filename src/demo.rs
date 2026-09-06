//! The five scripted transitions from MODEL.md, separate from their execution.
use crate::engine::{Action, Agent, Position, Proposal, World};

pub const END_TICK: u64 = 5;

pub fn initial_world() -> World {
    World::new(
        5,
        5,
        &[
            (Position::new(0, 2), Agent { id: 1, value: 10 }),
            (Position::new(2, 2), Agent { id: 2, value: 20 }),
            (Position::new(4, 2), Agent { id: 3, value: 30 }),
        ],
    )
    .expect("the documented fixture is valid")
}

pub fn proposals(world: &World) -> Vec<Proposal> {
    use Action::{Create, Move, Remove, SetValue};
    match world.tick() {
        0 => vec![
            Proposal::new(1, Move(Position::new(1, 2))),
            Proposal::new(2, Move(Position::new(1, 2))),
            Proposal::new(3, SetValue(31)),
        ],
        1 => vec![
            Proposal::new(1, Remove),
            Proposal::new(2, Create(Position::new(3, 2))),
            Proposal::new(3, Move(Position::new(0, 2))),
        ],
        2 => vec![
            Proposal::new(2, SetValue(21)),
            Proposal::new(3, Move(Position::new(0, 2))),
            Proposal::new(4, Move(Position::new(3, 3))),
        ],
        3 => vec![Proposal::new(2, Create(Position::new(1, 2)))],
        4 => vec![Proposal::new(3, Remove), Proposal::new(4, Remove)],
        // Headless requests beyond five ticks perform empty (all-wait) ticks.
        _ => vec![],
    }
}
