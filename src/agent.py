
import random

class Agent:

    display_character = 'O'
    avg_life_expectancy = 0
    std_dev_life_expectancy = 1
    initial_try_strength = 1

    def __init__(self, tile):
        self.current_tile = tile
        self.previous_tile = None
        self.turns_to_live = self.life_expectancy()
        self.try_strength = self.initial_try_strength

    def life_expectancy(self):
        turns_to_live = random.gauss(self.avg_life_expectancy, self.std_dev_life_expectancy)
        if (turns_to_live > 0):
            return turns_to_live
        else:
            return 0

    def is_dead(self):
        if self.turns_to_live > 0:
            return False
        return True

    def set_new_tile(self, tile):
        self.previous_tile = self.current_tile
        self.current_tile = tile

    def display(self):
        return self.display_character

    def move(self):
        empty_neighbor_tiles = self.current_tile.get_empty_neighbors()
        if empty_neighbor_tiles:
            next_tile = random.choice(empty_neighbor_tiles)
            self.current_tile.remove_agent()
            next_tile.set_agent(self)
            self.set_new_tile(next_tile)

    def split(self):
        empty_neighbor_tiles = self.current_tile.get_empty_neighbors()
        if empty_neighbor_tiles:
            target_tile = random.choice(empty_neighbor_tiles)
            new_agent = self.__class__(target_tile)
            target_tile.set_agent(new_agent)

    def draw_action(self):
        random_proba = random.random()
        if random_proba <= 0.5:
            return 'move'
        elif random_proba <= 0.9:
            return 'split'
        else:
            return 'rest'

    def next_step(self):
        action_to_do = self.draw_action()
        if action_to_do == 'move':
            self.move()
        elif action_to_do == 'split':
            self.split()
        elif action_to_do == 'rest':
            pass
        self.turns_to_live -= 1
        if self.turns_to_live <= 0:
            self.current_tile.remove_agent()


class Cell(Agent):

    display_character = '0'
    avg_life_expectancy = 300
    std_dev_life_expectancy = 200
    initial_try_strength = 3

    def draw_action(self):
        random_proba = random.random()
        if random_proba <= 0.2:
            return 'move'
        elif random_proba <= 0.203:
            return 'split'
        else:
            return 'rest'


