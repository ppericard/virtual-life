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
        description: "A moderate spread of movement weights, with more repair for faster movers.",
        bundles: [
            [34, 4, 12, 30],
            [26, 10, 12, 32],
            [18, 16, 12, 34],
            [10, 22, 12, 36],
        ],
        proportions: [1; 4],
    },
    Preset {
        id: "wide-movement-range",
        name: "Wide movement range",
        description: "A wider spread of movement weights, with more repair for faster movers.",
        bundles: [
            [34, 4, 12, 30],
            [23, 12, 12, 33],
            [12, 20, 12, 36],
            [1, 28, 12, 39],
        ],
        proportions: [1; 4],
    },
    Preset {
        id: "lower-copying",
        name: "Lower copying",
        description: "The wide movement range with some Copy weight transferred to Wait.",
        bundles: [
            [38, 4, 8, 30],
            [27, 12, 8, 33],
            [16, 20, 8, 36],
            [5, 28, 8, 39],
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
