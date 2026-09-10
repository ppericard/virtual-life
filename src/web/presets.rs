//! Browser restart choices, not simulation identities or transition rules.
use crate::experiment::ExperimentInfo;

#[derive(serde::Serialize)]
pub(super) struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub bundles: [[u32; 4]; 4],
    pub proportions: [u32; 4],
}

pub(super) const CATALOG: [Preset; 4] = [
    Preset {
        id: "original",
        name: "Original",
        description: "The original four combinations of wait, move, copy and repair weights.",
        bundles: [[2, 4, 1, 3], [2, 2, 2, 4], [4, 1, 1, 4], [1, 5, 2, 2]],
        proportions: [1; 4],
    },
    Preset {
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
    Preset {
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
    Preset {
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

pub(super) fn find(id: &str) -> Option<&'static Preset> {
    CATALOG.iter().find(|preset| preset.id == id)
}

pub(super) fn matching(info: &ExperimentInfo) -> Option<&'static str> {
    info.config.maintenance?;
    CATALOG
        .iter()
        .find(|preset| {
            info.groups.len() == preset.bundles.len()
                && info.groups.iter().enumerate().all(|(i, group)| {
                    group.weights.0 == preset.bundles[i]
                        && group.proportion == u64::from(preset.proportions[i])
                })
        })
        .map(|preset| preset.id)
}
