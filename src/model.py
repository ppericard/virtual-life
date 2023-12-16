
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
        self.env_matrix = [[Tile() for _ in range(env_width)] for _ in range(env_height)]
        self.__init_env_matrix()
        #
        self.__populate_env(populate_proba)

    def __init_env_matrix(self):
        """
        Initializes the environment matrix with tiles and assigns neighbors to each tile.
        """
        # To quickly get all neighbor tiles
        offset_list = ((-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1))
        # For each tile, add all neighbor tiles. As in a 2 dimensional toric environment
        for i in range(self.env_height):
            for j in range(self.env_width):
                for offset in offset_list:
                    neighbor_i = (i+offset[0]+self.env_height) % self.env_height
                    neighbor_j = (j+offset[1]+self.env_width) % self.env_width
                    neighbor_tile = self.env_matrix[neighbor_i][neighbor_j]
                    self.env_matrix[i][j].add_neighbor(neighbor_tile)

    def __populate_env(self, populate_proba):
        """
        Populates the environment with agents based on the given probability.
        """
        for i in range(self.env_height):
            for j in range(self.env_width):
                if random.random() < populate_proba:
                    tile = self.env_matrix[i][j]
                    new_agent = Cell(tile)
                    tile.set_agent(new_agent)

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
        for row in self.env_matrix:
            for tile in row:
                if not tile.is_empty():
                    tile.agent.next_step()