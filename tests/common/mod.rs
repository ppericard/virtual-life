#[allow(dead_code)]
pub fn machine(weights: virtual_life::engine::Weights) -> virtual_life::automaton::Automaton {
    virtual_life::automaton::Automaton {
        rows: [weights; 4],
        ..Default::default()
    }
}
