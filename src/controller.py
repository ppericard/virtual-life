
from observer import *
from model import MyModel
from view import MyView

import random

import sys

class MyController(Observable):

    def __init__(self, env_height, env_width, populate_proba):
        # Random seed initialization
        # random.seed(1)
        random.seed()
        #
        self.model = MyModel(env_height, env_width, populate_proba)
        self.view = MyView(self, self.model)

    def start(self):
        print('{0} {1}'.format(self.model.env_height, self.model.env_width))

        for i in range(self.model.env_height):
            for j in range(self.model.env_width):
                tile = self.model.env_matrix[i][j]
                if tile.agent:
                    sys.stdout.write('0 ')
                else:
                    sys.stdout.write('  ')
            sys.stdout.write('\n')

        return 0

