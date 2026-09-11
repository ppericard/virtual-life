//! Inherited four-state unit-action graphs. Selection and spatial resolution
//! still use the shared experiment policy and engine, respectively.
use crate::engine::{ActionState, Weights};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "web", derive(serde::Serialize))]
pub struct Automaton {
    pub initial: ActionState,
    /// Source rows and destination columns are Wait, Move, Copy, Repair.
    pub rows: [Weights; 4],
    /// Copy multiplier is 1 + this gain * missing-integrity fraction.
    pub copy_damage_gain: u8,
    /// Each occupied neighbour adds this many tickets per base Move weight.
    pub move_crowding_gain: u8,
}

impl Default for Automaton {
    fn default() -> Self {
        Self {
            initial: ActionState::Wait,
            rows: [Weights::default(); 4],
            copy_damage_gain: 0,
            move_crowding_gain: 0,
        }
    }
}

impl Automaton {
    pub fn base_weights(self, previous: Option<ActionState>) -> Weights {
        self.rows[previous.unwrap_or(self.initial).index()]
    }

    pub fn validate(self) -> Result<(), String> {
        if self
            .rows
            .iter()
            .any(|row| row.0.iter().all(|&weight| weight == 0))
        {
            return Err("every automaton state needs at least one outgoing transition".into());
        }
        Ok(())
    }

    /// A small, lossless CLI format; even unreachable rows must be valid.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (initial, rows) = text
            .split_once(':')
            .ok_or("automaton needs initial:wait-row/move-row/copy-row/repair-row")?;
        let (initial, response) = initial
            .split_once('~')
            .map_or((initial, None), |(state, value)| (state, Some(value)));
        let [copy_damage_gain, move_crowding_gain] = match response {
            Some(value) => value
                .split(',')
                .map(|value| {
                    value
                        .parse::<u8>()
                        .map_err(|_| "response parameters must be integers 0..255".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .map_err(|_| "responses need copy-damage-gain,move-crowding-gain")?,
            None => [0, 0],
        };
        let initial = match initial {
            "wait" => ActionState::Wait,
            "move" => ActionState::Move,
            "copy" => ActionState::Copy,
            "repair" => ActionState::Repair,
            _ => return Err("initial automaton state must be wait, move, copy or repair".into()),
        };
        let rows = rows
            .split('/')
            .map(|row| {
                let values = row
                    .split(',')
                    .map(|part| {
                        part.parse::<u32>().map_err(|_| {
                            "transition weights must be nonnegative u32 integers".to_owned()
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Weights(
                    values
                        .try_into()
                        .map_err(|_| "each transition row needs four weights")?,
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let machine = Self {
            initial,
            rows: rows
                .try_into()
                .map_err(|_| "each automaton needs four rows")?,
            copy_damage_gain,
            move_crowding_gain,
        };
        machine.validate()?;
        Ok(machine)
    }

    pub fn specification(self) -> String {
        let response = if (self.copy_damage_gain, self.move_crowding_gain) == (0, 0) {
            String::new()
        } else {
            format!("~{},{}", self.copy_damage_gain, self.move_crowding_gain)
        };
        format!(
            "{}{response}:{}",
            self.initial.label().to_ascii_lowercase(),
            self.rows
                .iter()
                .map(|row| row.0.map(|w| w.to_string()).join(","))
                .collect::<Vec<_>>()
                .join("/")
        )
    }
}

pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub machine: Automaton,
}

/// Illustrative inherited graphs, not biological classes or balancing promises.
pub const PRESETS: [Preset; 4] = [
    Preset {
        id: "movement-runs",
        name: "Movement runs",
        description: "Movement can repeat and becomes more likely under crowding; Copy leads to Repair.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 0,
            move_crowding_gain: 2,
            rows: [
                Weights([0, 3, 0, 2]),
                Weights([0, 3, 0, 2]),
                Weights([0, 0, 0, 1]),
                Weights([2, 9, 1, 8]),
            ],
        },
    },
    Preset {
        id: "repair-cycles",
        name: "Repair cycles",
        description: "Wait, Move and Copy always lead back to Repair.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 0,
            move_crowding_gain: 0,
            rows: [
                Weights([0, 0, 0, 1]),
                Weights([0, 0, 0, 1]),
                Weights([0, 0, 0, 1]),
                Weights([3, 3, 2, 12]),
            ],
        },
    },
    Preset {
        id: "copy-bursts",
        name: "Copy bursts",
        description: "Copy can repeat and gains weight at low integrity, especially with empty neighbours.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 2,
            move_crowding_gain: 0,
            rows: [
                Weights([0, 0, 0, 1]),
                Weights([0, 0, 0, 1]),
                Weights([1, 0, 3, 6]),
                Weights([2, 1, 7, 10]),
            ],
        },
    },
    Preset {
        id: "wait-cycles",
        name: "Wait cycles",
        description: "Wait can repeat, with occasional movement and copying after repair.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 0,
            move_crowding_gain: 0,
            rows: [
                Weights([3, 0, 0, 2]),
                Weights([1, 0, 0, 0]),
                Weights([0, 0, 0, 1]),
                Weights([10, 1, 1, 8]),
            ],
        },
    },
];

/// Candidates for open space, movement and population turnover under default
/// maintenance. Finite seeded trials do not establish lasting coexistence.
pub const TRIAL_PRESETS: [Preset; 3] = [
    Preset {
        id: "roamers",
        name: "Roamers",
        description: "Move can repeat; Copy always leads to Repair before another movement or copying choice.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 0,
            move_crowding_gain: 0,
            rows: [
                Weights([0, 8, 6, 17]),
                Weights([0, 8, 0, 17]),
                Weights([0, 0, 0, 1]),
                Weights([0, 8, 6, 17]),
            ],
        },
    },
    Preset {
        id: "burst-copiers",
        name: "Burst copiers",
        description: "Copy can repeat and gains weight as integrity falls; movement leads back to Repair.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 2,
            move_crowding_gain: 0,
            rows: [
                Weights([0, 2, 6, 8]),
                Weights([0, 0, 0, 1]),
                Weights([0, 0, 6, 8]),
                Weights([1, 2, 6, 8]),
            ],
        },
    },
    Preset {
        id: "settlers",
        name: "Settlers",
        description: "Wait can repeat; Move is available only after Repair, and Copy always returns to Repair.",
        machine: Automaton {
            initial: ActionState::Wait,
            copy_damage_gain: 0,
            move_crowding_gain: 0,
            rows: [
                Weights([8, 0, 3, 11]),
                Weights([0, 0, 0, 1]),
                Weights([0, 0, 0, 1]),
                Weights([8, 2, 3, 11]),
            ],
        },
    },
];

pub fn catalog() -> impl Iterator<Item = &'static Preset> {
    PRESETS.iter().chain(TRIAL_PRESETS.iter())
}

pub fn find(id: &str) -> Option<&'static Preset> {
    catalog().find(|preset| preset.id == id)
}
