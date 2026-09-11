//! Browser restart choices. Names are labels; actual inherited data defines identity.
use crate::automaton::{self, Automaton};
use crate::experiment::ExperimentInfo;

#[derive(serde::Serialize)]
pub(super) struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub automata: Vec<Automaton>,
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
        proportions: vec![1; automata.len()],
        automata,
    }
}

pub(super) fn catalog() -> Vec<Preset> {
    let mut presets = vec![machines(
        "mixed-automata",
        "Mixed automata",
        "Four inherited graphs with distinct sequences and responses to integrity and crowding.",
        automaton::PRESETS.iter().map(|p| p.machine).collect(),
    )];
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
    catalog()
        .into_iter()
        .find(|preset| {
            info.groups.len() == preset.automata.len()
                && info.groups.iter().enumerate().all(|(i, group)| {
                    group.automaton == preset.automata[i]
                        && group.proportion == u64::from(preset.proportions[i])
                })
        })
        .map(|preset| preset.id)
}
