
from agent import *
from inanimate import *
from environment import *

import random

class MyModel:
    """
    Represents the simulation model, managing the environment and agents.
    """

    def __init__(self, env_height, env_width, populate_proba):
        """
        Initializes the simulation model with a given environment size and population probability.
        """
        self.env_height = env_height
        self.env_width = env_width
        self.env_matrix = [[Tile(i, j) for i in range(env_width)] for j in range(env_height)]
        self._assign_neighbors_to_tiles()
        #
        self.active_agents = []  # List to store active agents
        self._populate_env(populate_proba)


    def _get_neighbors(self, i, j, distance):
        """
        Retrieves unique neighbors exactly at the specified distance.
        Returns an empty list if the distance exceeds the maximum possible value.
        """
        # Calculate the maximum possible distance
        max_possible_distance = min(self.env_height // 2, self.env_width // 2)

        # Return nothing if the requested distance exceeds the maximum possible distance
        if distance > max_possible_distance:
            return []

        neighbors_set = set()
        for di in range(-distance, distance + 1):
            for dj in range(-distance, distance + 1):
                # Skip the center tile and tiles not exactly at the specified distance
                if (di == 0 and dj == 0) or (abs(di) != distance and abs(dj) != distance):
                    continue
                # Calculate toric coordinates
                neighbor_i = (i + di + self.env_height) % self.env_height
                neighbor_j = (j + dj + self.env_width) % self.env_width
                # Add the neighbor tile to the set
                neighbors_set.add(self.env_matrix[neighbor_i][neighbor_j])
        return list(neighbors_set)

    def _assign_neighbors_to_tiles(self, max_distance=3):
        for i in range(self.env_height):
            for j in range(self.env_width):
                neighbors_dict = {d: self._get_neighbors(i, j, d) for d in range(1, max_distance + 1)}
                self.env_matrix[i][j].set_neighbors(neighbors_dict)

    def agent_died(self, agent):
        if agent in self.active_agents:
            self.active_agents.remove(agent)

    def agent_born(self, new_agent):
        self.active_agents.append(new_agent)

    def _populate_env(self, populate_proba):
        """
        Populates the environment with agents based on the given probability.
        """
        for i in range(self.env_height):
            for j in range(self.env_width):
                if random.random() < populate_proba:
                    tile = self.env_matrix[i][j]
                    new_agent = Cell(tile, death_callback=self.agent_died, born_callback=self.agent_born)
                    tile.set_agent(new_agent)
                    self.active_agents.append(new_agent)  # Add agent to active list

    def get_tile_at_position(self, i, j):
        """
        Returns the tile at a specific position in the environment.
        """
        if 0 <= i < self.env_height and 0 <= j < self.env_width:
            return self.env_matrix[i][j]
        else:
            raise ValueError("Position out of environment bounds")

    def run_simulation_step(self):
        """
        Executes the next step for each agent in the simulation.
        """
        for agent in self.active_agents:
            agent.next_step()
