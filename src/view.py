
import sys

from observer import *
from agent_view import *
from inanimate_view import *
from environment_view import *

class MyView(Observer):

    def __init__(self, model):
        self.model = model

    def update(self):
        self.display()

    def display(self):

        print('height={0}, width={1}'.format(self.model.env_height, self.model.env_width))

        for i in range(self.model.env_height):
            for j in range(self.model.env_width):
                tile = self.model.env_matrix[i][j]
                if tile.agent:
                    sys.stdout.write('{0} '.format(tile.agent.display_character))
                else:
                    sys.stdout.write('{0} '.format(tile.display_character))
            sys.stdout.write('\n')
        
        print('agents_nb={0}'.format(len()))









