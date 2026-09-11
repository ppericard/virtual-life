//! Browser restart choices. Names are labels; actual inherited data defines identity.
use crate::automaton::{self, Automaton};
use crate::experiment::ExperimentInfo;

#[derive(serde::Serialize)]
struct FlatPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub bundles: [[u32; 4]; 4],
    pub proportions: [u32; 4],
}

const FLAT_CATALOG: [FlatPreset; 4] = [
    FlatPreset {
        id: "original",
        name: "Original",
        description: "The original four combinations of wait, move, copy and repair weights.",
        bundles: [[2, 4, 1, 3], [2, 2, 2, 4], [4, 1, 1, 4], [1, 5, 2, 2]],
        proportions: [1; 4],
    },
    FlatPreset {
        id: "moderate-movement",
        name: "Moderate movement",
        description: "5% base Copy chance, with a moderate spread of movement weights.",
        bundles: [
            [42, 4, 4, 30],
            [34, 10, 4, 32],
            [26, 16, 4, 34],
            [18, 22, 4, 36],
        ],
        proportions: [1; 4],
    },
    FlatPreset {
        id: "wide-movement-range",
        name: "Wide movement range",
        description: "5% base Copy chance, with a wider spread of movement weights.",
        bundles: [
            [42, 4, 4, 30],
            [31, 12, 4, 33],
            [20, 20, 4, 36],
            [9, 28, 4, 39],
        ],
        proportions: [1; 4],
    },
    FlatPreset {
        id: "lower-copying",
        name: "Lower copying",
        description: "2.5% base Copy chance, with the same wide movement range.",
        bundles: [
            [44, 4, 2, 30],
            [33, 12, 2, 33],
            [22, 20, 2, 36],
            [11, 28, 2, 39],
        ],
        proportions: [1; 4],
    },
];

#[derive(serde::Serialize)]
pub(super) struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub bundles: Vec<[u32; 4]>,
    pub automata: Option<Vec<Automaton>>,
    pub proportions: Vec<u32>,
}

fn machines(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    automata: Vec<Automaton>,
) -> Preset {
    Preset {
        id,
        name,
        description,
        bundles: automata.iter().map(|m| m.base_weights(None).0).collect(),
        proportions: vec![1; automata.len()],
        automata: Some(automata),
    }
}

pub(super) fn catalog() -> Vec<Preset> {
    let mut presets: Vec<_> = FLAT_CATALOG
        .iter()
        .map(|p| Preset {
            id: p.id,
            name: p.name,
            description: p.description,
            bundles: p.bundles.to_vec(),
            proportions: p.proportions.to_vec(),
            automata: None,
        })
        .collect();
    presets.push(machines(
        "mixed-automata",
        "Mixed automata",
        "Four inherited graphs with distinct sequences and responses to integrity and crowding.",
        automaton::PRESETS.iter().map(|p| p.machine).collect(),
    ));
    presets.extend(
        automaton::PRESETS
            .iter()
            .map(|p| machines(p.id, p.name, p.description, vec![p.machine])),
    );
    presets
}

pub(super) fn find(id: &str) -> Option<Preset> {
    catalog().into_iter().find(|preset| preset.id == id)
}

pub(super) fn matching(info: &ExperimentInfo) -> Option<&'static str> {
    info.config.maintenance?;
    catalog()
        .into_iter()
        .find(|preset| {
            info.groups.len() == preset.bundles.len()
                && info.groups.iter().enumerate().all(|(i, group)| {
                    group.weights.0 == preset.bundles[i]
                        && group.automaton == preset.automata.as_ref().map(|machines| machines[i])
                        && group.proportion == u64::from(preset.proportions[i])
                })
        })
        .map(|preset| preset.id)
}
