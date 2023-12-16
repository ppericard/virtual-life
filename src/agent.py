import random
from abc import ABC, abstractmethod
from typing import Optional, List

class Agent(ABC):
    """
    Abstract base class for all agents in the simulation.
    """
    possible_characters = ['O']
    avg_life_expectancy = 0
    std_dev_life_expectancy = 1
    default_mutation_probability = 0
    initial_try_strength = 1

    # Probabilities for each action
    action_probabilities = {
        'move': 0.5,
        'split': 0.4
    }

    def __init__(self, tile, display_character=None, mutation_probability=None):
        self._current_tile = tile
        self._previous_tile = None
        self._turns_to_live = self._life_expectancy()
        self._try_strength = self.initial_try_strength
        self.display_character = display_character if display_character else random.choice(self.possible_characters)
        # Set mutation probability, or use class's default
        self.mutation_probability = mutation_probability if mutation_probability is not None else self.default_mutation_probability

    @property
    def current_tile(self):
        return self._current_tile

    @property
    def previous_tile(self):
        return self._previous_tile

    @property
    def turns_to_live(self):
        return self._turns_to_live

    def _life_expectancy(self):
        return max(random.gauss(self.__class__.avg_life_expectancy, self.__class__.std_dev_life_expectancy), 0)

    def is_dead(self) -> bool:
        return self._turns_to_live <= 0

    def set_new_tile(self, tile):
        if tile is None:
            raise ValueError("Tile cannot be None")
        self._previous_tile = self._current_tile
        self._current_tile = tile

    def display(self) -> str:
        return self.display_character

    def move(self):
        """
        Moves the agent to a neighboring tile, if available.
        """
        empty_neighbor_tiles = self._current_tile.get_empty_neighbors()
        if empty_neighbor_tiles:
            next_tile = random.choice(empty_neighbor_tiles)
            self._current_tile.remove_agent()
            next_tile.set_agent(self)
            self.set_new_tile(next_tile)

    def split(self):
        """
        Splits the agent, creating a new agent in a neighboring tile, if available.
        There is a small chance for the child agent to mutate its display character.
        """
        empty_neighbor_tiles = self._current_tile.get_empty_neighbors()
        if empty_neighbor_tiles:
            target_tile = random.choice(empty_neighbor_tiles)
            # Decide if mutation occurs
            mutation_chance = 0.05  # 5% chance of mutation
            if random.random() < self.mutation_probability:
                # Mutate: select a new character, different from the parent's
                new_character = random.choice([char for char in self.possible_characters if char != self.display_character])
            else:
                # No mutation: inherit parent's character
                new_character = self.display_character

            new_agent = self.__class__(target_tile, new_character)
            target_tile.set_agent(new_agent)

    def draw_action(self) -> str:
        """
        Randomly decides the next action of the agent.
        """
        random_proba = random.random()
        cumulative_proba = 0
        for action, proba in self.action_probabilities.items():
            cumulative_proba += proba
            if random_proba <= cumulative_proba:
                return action
        return 'rest'  # Fallback in case of rounding errors

    def next_step(self):
        """
        Executes the next step in the agent's lifecycle, based on the drawn action.
        """
        if self.is_dead():
            self._current_tile.remove_agent()
            return

        action_to_do = self.draw_action()
        if action_to_do == 'move':
            self.move()
        elif action_to_do == 'split':
            self.split()
        # No action needed for 'rest'

        self._turns_to_live -= 1

    @abstractmethod
    def interact(self, other_agent):
        """
        Defines interaction with another agent.
        """
        pass



class Cell(Agent):
    """
    Cell class, derived from Agent, with specific properties and behaviors.
    """
    possible_characters = ['*', '#', '@', '+', '%', '&', '?', '!', '$', '§', '~', '=', '-', '_', ':', ';', '^', '>', '<', '|']
    avg_life_expectancy = 300
    std_dev_life_expectancy = 200
    default_mutation_probability = 0.05  # Default mutation probability for Cell
    initial_try_strength = 3

    action_probabilities = {
        'move': 1/5,
        'split': 1/300
    }

    def __init__(self, tile, display_character=None, mutation_probability=None):
        super().__init__(tile, display_character, mutation_probability)

    def interact(self, other_agent):
        # Define how a cell interacts with another agent
        pass