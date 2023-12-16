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

    def __init__(self, tile, display_character=None, mutation_probability=None, death_callback=None, born_callback=None):
        self._current_tile = tile
        self._previous_tile = None
        self._turns_to_live = self._life_expectancy()
        self._try_strength = self.initial_try_strength
        self.display_character = display_character if display_character else random.choice(self.possible_characters)
        # Set mutation probability, or use class's default
        self.mutation_probability = mutation_probability if mutation_probability is not None else self.default_mutation_probability
        self.death_callback = death_callback
        self.born_callback = born_callback

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

            new_agent = self.__class__(target_tile, new_character, 
                                       death_callback=self.death_callback, 
                                       born_callback=self.born_callback)
            target_tile.set_agent(new_agent)
            if self.born_callback:
                self.born_callback(new_agent)

    def calculate_adjusted_split_probability(self):
        """
        Calculates the adjusted probability of splitting based on the neighbors.
        This method provides a generic implementation, which can be overridden by subclasses.
        """
        neighbors_agents = self._current_tile.get_neighbors_agents()
        # Example generic logic (can be adjusted or replaced in subclasses)
        if len(neighbors_agents) > 4:
            return max(self.action_probabilities['split'] / 2, 0.01)
        else:
            return self.action_probabilities['split']

    def draw_action(self) -> str:
        """
        Randomly decides the next action of the agent, with adjusted probabilities.
        """
        adjusted_split_proba = self.calculate_adjusted_split_probability()

        # Decide the action
        random_proba = random.random()
        cumulative_proba = 0

        for action, proba in self.action_probabilities.items():
            if action == 'split':
                cumulative_proba += adjusted_split_proba
            else:
                # Adjust the probability of other actions to maintain the total probability of 1
                cumulative_proba += proba / (1 - self.action_probabilities['split']) * (1 - adjusted_split_proba)

            if random_proba <= cumulative_proba:
                return action

        return 'rest'  # Fallback in case of rounding errors

    def adjust_life_expectancy_based_on_neighbors(self):
        """
        Adjusts the agent's life expectancy based on neighboring agents.
        This method provides a generic implementation, which can be overridden by subclasses.
        """
        neighbors_agents = self._current_tile.get_neighbors_agents()
        # Implement generic logic here. For example:
        # Reduce life expectancy if too crowded
        if len(neighbors_agents) > 4:
            self._turns_to_live -= 1

    def next_step(self):
        """
        Executes the next step in the agent's lifecycle, based on the drawn action.
        """
        self.adjust_life_expectancy_based_on_neighbors()  # Adjust life expectancy

        if self.is_dead():
            self._current_tile.remove_agent()
            if self.death_callback:
                self.death_callback(self)
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
    avg_life_expectancy = 200
    std_dev_life_expectancy = 100
    default_mutation_probability = 0.05  # Default mutation probability for Cell
    initial_try_strength = 3

    action_probabilities = {
        'move': 1/5,
        'split': 1/200
    }

    def __init__(self, tile, display_character=None, mutation_probability=None, death_callback=None, born_callback=None):
        super().__init__(tile, display_character, mutation_probability, death_callback, born_callback)

    def interact(self, other_agent):
        # Define how a cell interacts with another agent
        pass

    def adjust_life_expectancy_based_on_neighbors(self):
        """
        Cell-specific logic for adjusting life expectancy based on neighbors.
        """
        neighbors_agents = self._current_tile.get_neighbors_agents(max_distance=3)

        # Reduce life expectancy if too crowded
        self._turns_to_live -= max(0, len(neighbors_agents) - 3)

        #Increase life expectancy for more same-character neighbors
        same_character_neighbors_count = len([a for a in neighbors_agents if a.display_character == self.display_character])
        if same_character_neighbors_count >= 2:
            self._turns_to_live += 0.1
    
    def calculate_adjusted_split_probability(self):
        """
        Cell-specific logic for adjusting split probability based on neighbors.
        """
        neighbors_agents = self._current_tile.get_neighbors_agents(max_distance=3)
        neighbors_count = len(neighbors_agents)

        adjusted_split_proba = self.action_probabilities['split']

        # Adjust for same-character neighbors
        same_char_neighbors_count = sum(a.display_character == self.display_character for a in neighbors_agents)
        adjusted_split_proba *= 1 + (same_char_neighbors_count / 20)

        # Adjust for crowding
        adjusted_split_proba *= 1 + (3 - neighbors_count) / 20

        # Ensure probability is within bounds [0, 1]
        adjusted_split_proba = min(max(adjusted_split_proba, 0), 1)

        # Limit split probability to min and max values
        max_split_proba = self.action_probabilities['split'] * 2
        min_split_proba = self.action_probabilities['split'] / 2

        return min(max(adjusted_split_proba, min_split_proba), max_split_proba)
