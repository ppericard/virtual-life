use std::fmt;

/// Stable identifier for an agent during a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AgentId(pub usize);

/// Position on the finite toroidal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

/// A generic agent: a position plus an ordered vector of integer properties.
///
/// Properties intentionally have no biological meaning in the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    pub properties: Vec<i32>,
    pub position: Position,
}

impl Agent {
    pub fn new(properties: Vec<i32>, position: Position) -> Self {
        Self {
            properties,
            position,
        }
    }
}

/// Spatial state of the simulation.
///
/// Agents are stored independently from tiles. A tile contains zero, one, or
/// many agent identifiers, so stacking is a first-class operation rather than
/// an exception in the representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    width: usize,
    height: usize,
    agents: Vec<Option<Agent>>,
    tiles: Vec<Vec<AgentId>>,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        assert!(width > 0 && height > 0, "world dimensions must be non-zero");
        Self {
            width,
            height,
            agents: Vec::new(),
            tiles: vec![Vec::new(); width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn agent_count(&self) -> usize {
        self.agents.iter().filter(|agent| agent.is_some()).count()
    }

    pub fn agent_ids(&self) -> impl Iterator<Item = AgentId> + '_ {
        self.agents
            .iter()
            .enumerate()
            .filter_map(|(index, agent)| agent.as_ref().map(|_| AgentId(index)))
    }

    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.get(id.0)?.as_ref()
    }

    pub fn agents_at(&self, position: Position) -> &[AgentId] {
        &self.tiles[self.tile_index(self.wrap(position))]
    }

    pub fn add_agent(&mut self, mut agent: Agent) -> AgentId {
        agent.position = self.wrap(agent.position);
        let position = agent.position;
        let id = AgentId(self.agents.len());
        self.agents.push(Some(agent));
        let index = self.tile_index(position);
        self.tiles[index].push(id);
        id
    }

    pub fn remove_agent(&mut self, id: AgentId) -> Option<Agent> {
        let agent = self.agents.get_mut(id.0)?.take()?;
        let index = self.tile_index(agent.position);
        if let Some(offset) = self.tiles[index].iter().position(|candidate| *candidate == id) {
            self.tiles[index].swap_remove(offset);
        }
        Some(agent)
    }

    /// Move an existing agent. Removing it from its old tile is O(k), where k
    /// is the number of agents stacked on that tile. This is intentionally the
    /// simple reference representation; optimize only if measurements justify it.
    pub fn move_agent(&mut self, id: AgentId, destination: Position) -> bool {
        let destination = self.wrap(destination);
        let Some(old_position) = self.agent(id).map(|agent| agent.position) else {
            return false;
        };

        let old_index = self.tile_index(old_position);
        if let Some(offset) = self.tiles[old_index]
            .iter()
            .position(|candidate| *candidate == id)
        {
            self.tiles[old_index].swap_remove(offset);
        }

        let new_index = self.tile_index(destination);
        self.tiles[new_index].push(id);
        self.agents[id.0].as_mut().expect("agent existed above").position = destination;
        true
    }

    pub fn wrap(&self, position: Position) -> Position {
        Position {
            x: position.x % self.width,
            y: position.y % self.height,
        }
    }

    pub fn offset(&self, position: Position, dx: isize, dy: isize) -> Position {
        Position {
            x: (position.x as isize + dx).rem_euclid(self.width as isize) as usize,
            y: (position.y as isize + dy).rem_euclid(self.height as isize) as usize,
        }
    }

    fn tile_index(&self, position: Position) -> usize {
        position.y * self.width + position.x
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_agents_can_share_a_tile() {
        let mut world = World::new(4, 3);
        let position = Position { x: 1, y: 2 };
        let first = world.add_agent(Agent::new(vec![1], position));
        let second = world.add_agent(Agent::new(vec![2], position));

        assert_eq!(world.agents_at(position), &[first, second]);
    }

    #[test]
    fn positions_wrap_on_both_axes() {
        let world = World::new(4, 3);
        let origin = Position { x: 0, y: 0 };

        assert_eq!(world.offset(origin, -1, -1), Position { x: 3, y: 2 });
        assert_eq!(world.offset(origin, 4, 3), origin);
    }

    #[test]
    fn moving_updates_both_agent_and_tile_indexes() {
        let mut world = World::new(3, 3);
        let start = Position { x: 0, y: 0 };
        let end = Position { x: 1, y: 0 };
        let id = world.add_agent(Agent::new(vec![], start));

        assert!(world.move_agent(id, end));
        assert!(world.agents_at(start).is_empty());
        assert_eq!(world.agents_at(end), &[id]);
        assert_eq!(world.agent(id).unwrap().position, end);
    }
}
