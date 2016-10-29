
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

    def set_new_tile(self, tile):
        self.previous_tile = self.current_tile
        self.current_tile = tile

    def display(self):
        return self.display_character

    def move(self):
        neighbor_tiles = self.current_tile.get_neighbors()
        tries_nb = 0
        max_tries = len(neighbor_tiles) * self.try_strength
        found_empty_tile = False
        while(not found_empty_tile and (tries_nb <= max_tries)):
            next_tile = random.choice(neighbor_tiles)
            if next_tile.is_empty():
                found_empty_tile = True
                self.current_tile.remove_agent()
                next_tile.set_agent(self)
                self.set_new_tile(next_tile)
            tries_nb += 1

    def draw_action(self):
        random_proba = random.random()
        if random_proba <= 0.5:
            return 'move'
        else:
            return 'rest'

    def next_step(self):
        action_to_do = self.draw_action()
        if action_to_do == 'move':
            self.move()
        elif action_to_do == 'rest':
            pass
        self.turns_to_live -= 1



class Cell(Agent):

    display_character = '0'
    avg_life_expectancy = 600
    std_dev_life_expectancy = 100
    initial_try_strength = 3



