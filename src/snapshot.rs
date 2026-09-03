use crate::{AgentId, Position, World};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSnapshot {
    pub id: AgentId,
    pub position: Position,
    pub properties: Vec<i32>,
}

/// Immutable observation of a simulation state.
///
/// Consumers may clone, render, record, or analyze snapshots. They cannot use
/// them to mutate the simulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub step: u64,
    pub width: usize,
    pub height: usize,
    pub agents: Vec<AgentSnapshot>,
}

impl Snapshot {
    pub(crate) fn from_world(step: u64, world: &World) -> Self {
        let agents = world
            .agent_ids()
            .map(|id| {
                let agent = world.agent(id).expect("agent id came from world");
                AgentSnapshot {
                    id,
                    position: agent.position,
                    properties: agent.properties.clone(),
                }
            })
            .collect();

        Self {
            step,
            width: world.width(),
            height: world.height(),
            agents,
        }
    }
}
