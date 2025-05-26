"""
This module defines the `Agent` and `Cell` classes for the simulation.

`Agent` is an abstract base class representing a generic agent with common behaviors
like moving, splitting, and aging. `Cell` is a concrete implementation of `Agent`
with specific characteristics and interaction logic.
"""
import random
from abc import ABC, abstractmethod
from typing import Optional, List, TYPE_CHECKING

if TYPE_CHECKING:
    from src.environment import Tile # To prevent circular import

class Agent(ABC):
    """
    Abstract base class for all agents in the simulation.

    Attributes:
        possible_characters (List[str]): List of characters that can represent the agent.
        avg_life_expectancy (int): Average lifespan of the agent.
        std_dev_life_expectancy (int): Standard deviation of the agent's lifespan.
        default_mutation_probability (float): Default probability of mutation on split.
        initial_try_strength (int): Initial strength value for some actions (unused currently).
        action_probabilities (Dict[str, float]): Probabilities for actions like 'move' or 'split'.
    """
    possible_characters = ['O']
    avg_life_expectancy = 0
    std_dev_life_expectancy = 1
    default_mutation_probability = 0
    initial_try_strength = 1

    # Probabilities for each action
    action_probabilities = {  # Base probabilities for each action
        'move': 0.5,  # Probability of moving
        'split': 0.4  # Probability of splitting
    }

    def __init__(self, tile: 'Tile', display_character: Optional[str] = None,
                 mutation_probability: Optional[float] = None,
                 death_callback: Optional[callable] = None,
                 born_callback: Optional[callable] = None):
        """
        Initializes an Agent instance.

        Args:
            tile: The initial tile where the agent is placed.
            display_character: The character used to display the agent. If None, chosen randomly.
            mutation_probability: The probability of this agent's offspring mutating.
                                  If None, uses the class default.
            death_callback: A function to call when the agent dies.
            born_callback: A function to call when the agent splits (a new agent is born).
        """
        self._current_tile: 'Tile' = tile
        self._previous_tile: Optional['Tile'] = None
        self._turns_to_live: float = self._life_expectancy()
        self._try_strength: int = self.initial_try_strength # Currently unused
        self.display_character: str = display_character if display_character else random.choice(self.possible_characters)
        # Set mutation probability, or use class's default
        self.mutation_probability: float = mutation_probability if mutation_probability is not None else self.default_mutation_probability
        self.death_callback: Optional[callable] = death_callback
        self.born_callback: Optional[callable] = born_callback

    @property
    def current_tile(self) -> 'Tile':
        """The tile the agent is currently occupying."""
        return self._current_tile

    @property
    def previous_tile(self) -> Optional['Tile']:
        """The tile the agent was occupying in the previous step."""
        return self._previous_tile

    @property
    def turns_to_live(self) -> float:
        """Remaining turns for the agent to live."""
        return self._turns_to_live

    def _life_expectancy(self) -> float:
        """
        Calculates the initial life expectancy of the agent using a Gaussian distribution.

        Returns:
            A non-negative float representing the agent's initial lifespan in turns.
        """
        return max(random.gauss(self.__class__.avg_life_expectancy, self.__class__.std_dev_life_expectancy), 0)

    def is_dead(self) -> bool:
        """
        Checks if the agent's turns to live have run out.

        Returns:
            True if the agent is dead, False otherwise.
        """
        return self._turns_to_live <= 0

    def set_new_tile(self, tile: 'Tile'):
        """
        Updates the agent's current and previous tile.

        Args:
            tile: The new tile the agent is moving to.

        Raises:
            ValueError: If the provided tile is None.
        """
        if tile is None:
            raise ValueError("Tile cannot be None")
        self._previous_tile = self._current_tile
        self._current_tile = tile

    def display(self) -> str:
        """
        Returns the character representation of the agent.

        Returns:
            The display character of the agent.
        """
        return self.display_character

    def move(self):
        """
        Moves the agent to a random empty neighboring tile.

        If no empty neighbors are available, the agent does not move.
        """
        empty_neighbor_tiles: List['Tile'] = self._current_tile.get_empty_neighbors()
        if empty_neighbor_tiles:
            next_tile: 'Tile' = random.choice(empty_neighbor_tiles)
            self._current_tile.remove_agent()  # Vacate current tile
            next_tile.set_agent(self)         # Occupy new tile
            self.set_new_tile(next_tile)

    def split(self):
        """
        Creates a new agent (offspring) in a random empty neighboring tile.

        The offspring may mutate its display character based on the agent's
        mutation probability. If no empty neighbors are available, no splitting occurs.
        The born_callback is triggered if an offspring is successfully created.
        """
        empty_neighbor_tiles: List['Tile'] = self._current_tile.get_empty_neighbors()
        if empty_neighbor_tiles:
            target_tile: 'Tile' = random.choice(empty_neighbor_tiles)
            
            # Determine if mutation occurs for the offspring
            mutated_char = self.display_character
            if random.random() < self.mutation_probability:
                # Mutate: select a new character, different from the parent's
                # Ensure there are other characters to choose from to avoid infinite loop
                possible_mutations = [char for char in self.possible_characters if char != self.display_character]
                if possible_mutations:
                    mutated_char = random.choice(possible_mutations)
            
            # Create the new agent (offspring)
            new_agent = self.__class__(target_tile, mutated_char,
                                       death_callback=self.death_callback,
                                       born_callback=self.born_callback,
                                       mutation_probability=self.mutation_probability) # Pass mutation prob to child
            target_tile.set_agent(new_agent)
            
            # Trigger the born callback
            if self.born_callback:
                self.born_callback(new_agent)

    def calculate_adjusted_split_probability(self) -> float:
        """
        Calculates the adjusted probability of splitting based on the agent's environment.

        This method provides a generic implementation that can be overridden by subclasses
        to introduce more complex splitting behaviors. The base implementation slightly
        reduces split probability if the agent is surrounded by many other agents.

        Returns:
            The adjusted probability of splitting, a float between 0.0 and 1.0.
        """
        neighbors_agents: List['Agent'] = self._current_tile.get_neighbors_agents()
        # Example generic logic: reduce split chance if crowded
        if len(neighbors_agents) > 4: # If more than 4 neighbors
            # Reduce split probability, but not below a minimum threshold (e.g., 0.01)
            return max(self.action_probabilities['split'] / 2, 0.01)
        else:
            return self.action_probabilities['split']

    def draw_action(self) -> str:
        """
        Randomly selects the next action for the agent based on adjusted probabilities.

        The probability of splitting is adjusted using `calculate_adjusted_split_probability`.
        Other action probabilities are scaled proportionally to ensure the total probability sums to 1.

        Returns:
            A string representing the chosen action ('move', 'split', or 'rest').
        """
        adjusted_split_proba: float = self.calculate_adjusted_split_probability()
        current_action_probabilities = self.action_probabilities.copy()

        # Update split probability
        original_split_proba = current_action_probabilities.get('split', 0)
        current_action_probabilities['split'] = adjusted_split_proba

        # Adjust other probabilities proportionally if split probability changed
        if original_split_proba > 0 and original_split_proba != adjusted_split_proba:
            scale_factor = (1 - adjusted_split_proba) / (1 - original_split_proba) if (1 - original_split_proba) != 0 else 0
            for action in current_action_probabilities:
                if action != 'split':
                    current_action_probabilities[action] *= scale_factor
        
        # Normalize probabilities to ensure they sum to 1 (handles potential floating point issues)
        total_proba = sum(current_action_probabilities.values())
        if total_proba == 0: # Avoid division by zero if all probabilities are zero
             return 'rest' 
        for action in current_action_probabilities:
            current_action_probabilities[action] /= total_proba

        # Decide the action based on weighted random choice
        random_val = random.random()
        cumulative_proba = 0
        for action, proba in current_action_probabilities.items():
            cumulative_proba += proba
            if random_val <= cumulative_proba:
                return action

        return 'rest'  # Fallback action

    def adjust_life_expectancy_based_on_neighbors(self):
        """
        Adjusts the agent's remaining life expectancy based on its neighboring agents.

        This method provides a generic implementation that can be overridden by subclasses.
        The base implementation reduces life expectancy if the agent is in a crowded area
        (more than 4 neighbors).
        """
        neighbors_agents: List['Agent'] = self._current_tile.get_neighbors_agents()
        # Generic logic: Reduce life expectancy if too crowded
        if len(neighbors_agents) > 4:
            self._turns_to_live -= 1  # Penalty for overcrowding

    def next_step(self):
        """
        Executes the next step in the agent's lifecycle.

        This involves:
        1. Adjusting life expectancy based on neighbors.
        2. Checking if the agent is dead; if so, remove it and trigger death callback.
        3. Drawing an action (move, split, rest).
        4. Performing the chosen action.
        5. Decrementing turns_to_live.
        """
        self.adjust_life_expectancy_based_on_neighbors()

        if self.is_dead():
            self._current_tile.remove_agent() # Vacate tile upon death
            if self.death_callback:
                self.death_callback(self)
            return

        action_to_do = self.draw_action()
        if action_to_do == 'move':
            self.move()
        elif action_to_do == 'split':
            self.split()
        # No action needed for 'rest'

        self._turns_to_live -= 1 # Age the agent

    @abstractmethod
    def interact(self, other_agent: 'Agent'):
        """
        Defines how this agent interacts with another agent.

        This is an abstract method and must be implemented by subclasses.

        Args:
            other_agent: The other agent to interact with.
        """
        pass


class Cell(Agent):
    """
    Represents a cell, a specific type of Agent in the simulation.

    Cells have their own set of possible display characters, life expectancy parameters,
    mutation probabilities, and action probabilities. They also implement custom logic
    for adjusting life expectancy and split probability based on their neighbors.
    """
    possible_characters = ['*', '#', '@', '+', '%', '&', '?', '!', '$', '§', '~', '=', '-', '_', ':', ';', '^', '>', '<', '|']
    possible_characters = ['*', '#', '@', '+', '%', '&', '?', '!', '$', '§', '~', '=', '-', '_', ':', ';', '^', '>', '<', '|']
    avg_life_expectancy = 200  # Average lifespan in simulation steps
    std_dev_life_expectancy = 100 # Standard deviation of lifespan
    default_mutation_probability = 0.05  # Default mutation probability for Cell offspring
    initial_try_strength = 3 # Currently unused

    action_probabilities = { # Base action probabilities for Cells
        'move': 1/10,
        'split': 1/200
    }

    def __init__(self, tile: 'Tile', display_character: Optional[str] = None,
                 mutation_probability: Optional[float] = None,
                 death_callback: Optional[callable] = None,
                 born_callback: Optional[callable] = None):
        """
        Initializes a Cell instance.

        Args:
            tile: The initial tile where the cell is placed.
            display_character: The character used to display the cell. If None, chosen randomly
                               from `Cell.possible_characters`.
            mutation_probability: The probability of this cell's offspring mutating.
                                  If None, uses `Cell.default_mutation_probability`.
            death_callback: A function to call when the cell dies.
            born_callback: A function to call when the cell splits.
        """
        super().__init__(tile, display_character, mutation_probability, death_callback, born_callback)

    def interact(self, other_agent: 'Agent'):
        """
        Defines how a cell interacts with another agent.
        Currently, interactions are not implemented for Cells.

        Args:
            other_agent: The other agent to interact with.
        """
        # TODO: Implement cell-specific interaction logic if needed.
        pass

    def adjust_life_expectancy_based_on_neighbors(self):
        """
        Adjusts the cell's life expectancy based on its neighbors within a 3-tile radius.

        - Crowding: Life expectancy decreases if there are more than 3 neighbors.
                  The penalty is `max(0, num_neighbors - 3)`.
        - Kinship: Life expectancy slightly increases (by 0.1 per step) if there are 2 or more
                   neighbors with the same display character.
        """
        neighbors_agents: List['Agent'] = self._current_tile.get_neighbors_agents(max_distance=3)

        # Reduce life expectancy if too crowded
        # Penalty is the number of neighbors exceeding 3
        crowding_penalty = max(0, len(neighbors_agents) - 3)
        self._turns_to_live -= crowding_penalty

        # Increase life expectancy for more same-character neighbors
        same_character_neighbors_count = len([a for a in neighbors_agents if a.display_character == self.display_character])
        if same_character_neighbors_count >= 2:
            self._turns_to_live += 0.1 # Small bonus for being near similar cells
    
    def calculate_adjusted_split_probability(self) -> float:
        """
        Calculates the cell-specific adjusted probability of splitting.

        This considers neighbors within a 3-tile radius and adjusts the base split
        probability based on:
        - Kinship: Increases if neighbors have the same display character.
                   Factor: `1 + (num_same_char_neighbors / 20)`
        - Crowding: Increases if there are fewer than 3 neighbors (more space),
                    decreases if there are more than 3 neighbors (less space).
                   Factor: `1 + (3 - num_total_neighbors) / 20`

        The final probability is clamped between half and double the base split probability
        defined in `Cell.action_probabilities['split']`, and also between 0 and 1.

        Returns:
            The adjusted probability of splitting for the cell.
        """
        neighbors_agents: List['Agent'] = self._current_tile.get_neighbors_agents(max_distance=3)
        neighbors_count: int = len(neighbors_agents)

        adjusted_split_proba: float = self.action_probabilities['split']

        # Adjust for same-character neighbors (kinship bonus)
        same_char_neighbors_count: int = sum(a.display_character == self.display_character for a in neighbors_agents)
        # Increase split probability if more same-character neighbors are present
        adjusted_split_proba *= (1 + (same_char_neighbors_count / 20))

        # Adjust for crowding (more space might encourage splitting, less space might discourage)
        # If neighbors_count < 3, factor is > 1 (encourages splitting)
        # If neighbors_count > 3, factor is < 1 (discourages splitting)
        adjusted_split_proba *= (1 + (3 - neighbors_count) / 20)

        # Ensure probability is within general bounds [0, 1]
        adjusted_split_proba = min(max(adjusted_split_proba, 0.0), 1.0)

        # Limit split probability to a defined range around the base probability
        # e.g., not more than double, not less than half of the original probability.
        max_split_proba: float = self.action_probabilities['split'] * 2
        min_split_proba: float = self.action_probabilities['split'] / 2

        return min(max(adjusted_split_proba, min_split_proba), max_split_proba)
