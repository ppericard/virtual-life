import sys

from observer import *

class MyView(Observer):

    def __init__(self, model, frame_per_second):
        self.model = model
        self.frame_per_second = frame_per_second
        self.frame_duration_in_sec = 1 / frame_per_second

    def update(self):
        self.display()

    def display(self):

        print('height={0}, width={1}'.format(self.model.env_height, self.model.env_width))

        agents_nb = 0

        for i in range(self.model.env_height):
            for j in range(self.model.env_width):
                tile = self.model.get_tile_at_position(i,j)
                if not tile.is_empty():
                    agents_nb += 1
                sys.stdout.write('{0} '.format(tile.display()))
            sys.stdout.write('\n')

        print('agents_nb={0}'.format(agents_nb))