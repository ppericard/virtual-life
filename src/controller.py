
from agent import * 
from viewer import *

class Controller:

    def __init__(self, env_height, env_width, populate_proba):
        #
        self.env_height = env_height
        self.env_width = env_width
        #
        self.env_matrix = list()
        self.env_matrix_view = list()
        self.init_env_matrix()
        #
        self.agent_list = list()
        self.agent_list_view = list()
        self.populate_env(populate_proba)

    def init_env_matrix(self):
        
        # Fill the environment matrix with tiles, and fill a similar matrix with the tiles view
        self.env_matrix = [[Tile() for j in range(self.env_width)] for i in range(self.env_height)]
        self.env_matrix_view = [[TileViewer(self.env_matrix[i][j]) for j in range(self.env_width)] 
                                for i in range(self.env_height)]

        # To quickly get all neighbor tiles
        offset_list = ((-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1))
        # For each tile, add all neighbor tiles. As in a 2 dimensional toric environment
        for i in range self.env_height:
            for j in range self.env_width:
                for offset in offset_list:
                    neighbor_coordinate_i = (i+offset[0]+self.env_height) % self.env_height 
                    neighbor_coordinate_j = (j+offset[1]+self.env_width) % self.env_width
                    neighbor_tile = self.env_matrix[neighbor_coordinate_i][neighbor_coordinate_j]
                    self.env_matrix.add_neighbor(neighbor_tile)


    def populate_env(self, populate_proba):




    def run(self):

