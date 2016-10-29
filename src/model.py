
from agent import *
from inanimate import *
from environment import *

import random

class MyModel:

    def __init__(self, env_height, env_width, populate_proba):
        #
        self.env_height = env_height
        self.env_width = env_width
        #
        self.env_matrix = list()
        self.__init_env_matrix()
        #
        self.__populate_env(populate_proba)

    def __init_env_matrix(self):
        # Fill the environment matrix with tiles, and fill a similar matrix with the tiles view
        self.env_matrix = [[Tile() for j in range(self.env_width)] for i in range(self.env_height)]
        # To quickly get all neighbor tiles
        offset_list = ((-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1))
        # For each tile, add all neighbor tiles. As in a 2 dimensional toric environment
        for i in range(self.env_height):
            for j in range(self.env_width):
                for offset in offset_list:
                    neighbor_coordinate_i = (i+offset[0]+self.env_height) % self.env_height
                    neighbor_coordinate_j = (j+offset[1]+self.env_width) % self.env_width
                    neighbor_tile = self.env_matrix[neighbor_coordinate_i][neighbor_coordinate_j]
                    self.env_matrix[i][j].add_neighbor(neighbor_tile)

    def __populate_env(self, populate_proba):
        for i in range(self.env_height):
            for j in range(self.env_width):
                if random.random() < populate_proba:
                    tile = self.env_matrix[i][j]
                    new_agent = Cell(tile)
                    tile.set_agent(new_agent)

    def get_tile_at_position(self, i, j):
        return self.env_matrix[i][j]