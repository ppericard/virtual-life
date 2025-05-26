"""
This module defines the `MyModel` class, which serves as the core of the simulation.

It manages the environment (grid of tiles), the agents within it, and the overall
simulation state and logic. It handles the creation of the environment, population
of agents, and the execution of simulation steps.
"""
from agent import Agent, Cell # Use specific imports for clarity
from environment import Tile
from observer import Observable # Import Observable
from typing import List, Dict # For type hinting

import random

class MyModel(Observable): # Inherit from Observable
    """
    Represents the simulation model, managing the environment grid and all agents.
    This class is an Observable, notifying registered Observers when its state changes.

    This class is responsible for initializing the simulation world, including
    creating all tiles and populating them with agents. It also contains the
    logic to advance the simulation by one time step, telling each agent to
    perform its actions.

    Attributes:
        env_height (int): The height of the simulation grid.
        env_width (int): The width of the simulation grid.
        env_matrix (List[List[Tile]]): A 2D list representing the grid of tiles.
        active_agents (List[Agent]): A list of all agents currently active in the simulation.
    """

    def __init__(self, env_height: int, env_width: int, populate_proba: float):
        """
        Initializes the simulation model.

        This involves setting up the environment grid, assigning neighbors to each tile,
        and populating the environment with initial agents based on `populate_proba`.

        Args:
            env_height: The height of the simulation environment.
            env_width: The width of the simulation environment.
            populate_proba: The probability (0.0 to 1.0) that a tile will initially
                            be populated with an agent.
        """
        super().__init__() # Initialize the Observable part
        self.env_height: int = env_height
        self.env_width: int = env_width
        # Create the grid of Tile objects
        self.env_matrix: List[List[Tile]] = [[Tile(j, i) for i in range(env_width)] for j in range(env_height)] # Corrected: Tile(row, col) -> Tile(j,i)
        self._assign_neighbors_to_tiles() # Pre-calculate neighbors for each tile

        self.active_agents: List[Agent] = []  # List to store all active agents
        self._populate_env(populate_proba) # Add initial agents to the environment


    def _get_neighbors(self, r: int, c: int, distance: int) -> List[Tile]:
        """
        Retrieves unique neighbor tiles exactly at a specified Manhattan distance
        from a given tile (r, c) in a toric (wrap-around) environment.

        Args:
            r: The row index of the central tile.
            c: The column index of the central tile.
            distance: The Manhattan distance at which to find neighbors.

        Returns:
            A list of `Tile` objects that are exactly at the specified distance.
            Returns an empty list if the distance is 0 or too large for meaningful
            distinct neighbors in a small grid (though toric nature makes this less restrictive).
        """
        if distance == 0:
            return []

        neighbors_set: set[Tile] = set()
        # Iterate over a square region and filter by Manhattan distance
        for dr in range(-distance, distance + 1):
            for dc in range(-distance, distance + 1):
                # Check for Manhattan distance
                if abs(dr) + abs(dc) == distance:
                    # Calculate toric (wrap-around) coordinates
                    neighbor_r = (r + dr + self.env_height) % self.env_height
                    neighbor_c = (c + dc + self.env_width) % self.env_width
                    neighbors_set.add(self.env_matrix[neighbor_r][neighbor_c])
        return list(neighbors_set)

    def _assign_neighbors_to_tiles(self, max_distance: int = 3):
        """
        Pre-calculates and assigns neighbors to each tile in the environment
        up to a specified maximum distance.

        This is done once during initialization to optimize neighbor lookups later.

        Args:
            max_distance: The maximum distance for which neighbors should be pre-calculated.
                          Defaults to 3.
        """
        for r in range(self.env_height): # r for row
            for c in range(self.env_width): # c for column
                # For each tile, find its neighbors at distances 1 through max_distance
                neighbors_dict: Dict[int, List[Tile]] = {}
                for d in range(1, max_distance + 1):
                    neighbors_dict[d] = self._get_neighbors(r, c, d)
                self.env_matrix[r][c].set_neighbors(neighbors_dict)

    def agent_died(self, agent: Agent):
        """
        Callback method invoked when an agent dies. Removes the agent from the
        list of active agents.

        Args:
            agent: The agent that has died.
        """
        if agent in self.active_agents:
            self.active_agents.remove(agent)
            # The agent should also be removed from its tile,
            # which is handled by Agent.next_step() or Tile.remove_agent()

    def agent_born(self, new_agent: Agent):
        """
        Callback method invoked when a new agent is born (e.g., through splitting).
        Adds the new agent to the list of active agents.

        Args:
            new_agent: The newly born agent.
        """
        if new_agent not in self.active_agents: # Ensure no duplicates
            self.active_agents.append(new_agent)

    def _populate_env(self, populate_proba: float):
        """
        Populates the environment's tiles with `Cell` agents based on a given probability.

        Each tile has a chance to spawn a new `Cell`. Callbacks for agent death
        and birth are passed to the new cells.

        Args:
            populate_proba: The probability (0.0 to 1.0) for each tile to be
                            populated with a new agent.
        """
        for r in range(self.env_height): # r for row
            for c in range(self.env_width): # c for column
                if random.random() < populate_proba:
                    tile = self.env_matrix[r][c]
                    if tile.is_empty(): # Only populate if tile is empty
                        # Create a new Cell agent
                        new_agent = Cell(tile,
                                         death_callback=self.agent_died,
                                         born_callback=self.agent_born)
                        tile.set_agent(new_agent) # Place agent on the tile
                        self.active_agents.append(new_agent) # Add to active agents list

    def get_tile_at_position(self, r: int, c: int) -> Tile:
        """
        Returns the tile at a specific (row, column) position in the environment.

        Args:
            r: The row index.
            c: The column index.

        Returns:
            The `Tile` object at the specified position.

        Raises:
            ValueError: If the position (r, c) is out of the environment's bounds.
        """
        if 0 <= r < self.env_height and 0 <= c < self.env_width:
            return self.env_matrix[r][c]
        else:
            raise ValueError(f"Position ({r}, {c}) is out of environment bounds.")

    def run_simulation_step(self):
        """
        Executes one step of the simulation.

        This involves instructing each active agent to perform its `next_step` action.
        A copy of the active agents list is used for iteration to prevent issues
        if agents are added or removed during the step (e.g., due to splitting or dying).
        """
        # Create a copy of the list of agents to process in this step.
        # This is important because agents might die or split, modifying self.active_agents.
        agents_to_process = self.active_agents[:] # Shallow copy is sufficient here
        for agent in agents_to_process:
            # Ensure agent is still alive and in the list (could have been removed by another agent's action)
            if agent in self.active_agents and not agent.is_dead():
                agent.next_step()
        
        self.notify_observers() # Notify observers that the model state has changed
