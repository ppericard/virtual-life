
import random

class Agent:

    display_character = 'O'

    def __init__(self, tile):
        self.current_tile = tile
        self.previous_tile = None

    def set_new_tile(self, tile):
        self.previous_tile = self.current_tile
        self.current_tile = tile

    def display(self):
        return self.display_character

    def next_step(self, try_strength):
        neighbor_tiles = self.current_tile.get_neighbors()
        tries_nb = 0
        max_tries = len(neighbor_tiles) * try_strength
        found_empty_tile = False
        while(not found_empty_tile and (tries_nb <= max_tries)):
            next_tile = random.choice(neighbor_tiles)
            if next_tile.is_empty():
                found_empty_tile = True
                self.current_tile.remove_agent()
                next_tile.set_agent(self)
                self.set_new_tile(next_tile)
            tries_nb += 1


class Cell(Agent):

    display_character = '0'



