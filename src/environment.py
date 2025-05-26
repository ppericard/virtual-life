"""
This module defines the `Tile` class, which represents a single unit or cell
in the simulation environment's grid.

Each tile knows its position, can hold an agent, and can identify its neighboring
tiles.
"""
from typing import Dict, List, Optional, Set # Added for type hinting
from src.agent import Agent # For type hinting Agent

class Tile:
    """
    Represents a single tile in the simulation environment's grid.

    A tile has coordinates (i, j), can store references to its neighboring tiles
    at various distances, and can optionally contain an agent.

    Attributes:
        display_character (str): The character used to display an empty tile.
        i (int): The row index of the tile in the grid.
        j (int): The column index of the tile in the grid.
        neighbors_by_distance (Dict[int, List['Tile']]): A dictionary mapping
            distance to a list of tiles at that distance.
        agent (Optional[Agent]): The agent currently occupying this tile, if any.
    """
    display_character: str = ' ' # Character for an empty tile

    def __init__(self, i: int, j: int):
        """
        Initializes a Tile instance.

        Args:
            i: The row index of the tile.
            j: The column index of the tile.
        """
        self.i: int = i
        self.j: int = j
        # Stores neighbors pre-calculated by the Environment, keyed by distance
        self.neighbors_by_distance: Dict[int, List['Tile']] = {}
        self.agent: Optional[Agent] = None # No agent on the tile initially

    def set_neighbors(self, neighbors_dict: Dict[int, List['Tile']]):
        """
        Sets the dictionary of neighboring tiles.

        This is typically called by the Environment class after all tiles are created
        and neighbor relationships are computed.

        Args:
            neighbors_dict: A dictionary where keys are distances (int) and
                            values are lists of `Tile` objects at that distance.
        """
        self.neighbors_by_distance = neighbors_dict

    def get_neighbors_at_distance(self, distance: int) -> List['Tile']:
        """
        Returns a list of tiles exactly at the specified distance.

        Args:
            distance: The exact distance at which to find neighbors.

        Returns:
            A list of `Tile` objects that are exactly at the given distance,
            or an empty list if no neighbors exist at that distance or the
            distance is not pre-calculated.
        """
        return self.neighbors_by_distance.get(distance, [])

    def get_neighbors(self, max_distance: int = 1) -> List['Tile']:
        """
        Returns a list of all unique neighboring tiles up to a specified maximum distance.

        Args:
            max_distance: The maximum distance to search for neighbors. Defaults to 1,
                          meaning immediately adjacent neighbors.

        Returns:
            A list of unique `Tile` objects within the `max_distance`.
        """
        all_neighbors: Set['Tile'] = set()
        for d in range(1, max_distance + 1):
            neighbors_at_d = self.neighbors_by_distance.get(d, [])
            all_neighbors.update(neighbors_at_d)
        return list(all_neighbors)
    
    def get_neighbors_agents(self, max_distance: int = 1) -> List[Agent]:
        """
        Returns a list of agents occupying neighboring tiles up to a specified maximum distance.

        Args:
            max_distance: The maximum distance to search for neighboring agents.

        Returns:
            A list of `Agent` objects found in the neighboring tiles.
        """
        neighboring_tiles: List['Tile'] = self.get_neighbors(max_distance)
        return [tile.agent for tile in neighboring_tiles if tile.agent is not None]

    def get_empty_neighbors(self, max_distance: int = 1) -> List['Tile']:
        """
        Returns a list of neighboring tiles that are empty (do not contain an agent).

        Args:
            max_distance: The maximum distance to search for empty neighboring tiles.

        Returns:
            A list of `Tile` objects that are empty and within the specified distance.
        """
        neighboring_tiles: List['Tile'] = self.get_neighbors(max_distance)
        return [tile for tile in neighboring_tiles if tile.is_empty()]

    def set_agent(self, agent: Agent):
        """
        Places an agent on this tile.

        Args:
            agent: The `Agent` to place on this tile.
                   If None, it's equivalent to calling `remove_agent()`.
        
        Raises:
            ValueError: If the tile is already occupied by another agent.
        """
        if agent is None:
            self.remove_agent()
        elif self.agent is None:
            self.agent = agent
        else:
            # Optional: Decide on behavior if tile is already occupied.
            # Could raise an error, overwrite, or ignore.
            # For now, let's assume an error or specific logic might be needed
            # depending on game rules, but the current Agent.move() handles checking for empty.
            # This explicit check here ensures Tile's state integrity.
            raise ValueError(f"Tile ({self.i}, {self.j}) is already occupied by {self.agent}.")


    def remove_agent(self):
        """Removes any agent currently on this tile, making it empty."""
        self.agent = None

    def is_empty(self) -> bool:
        """
        Checks if the tile is empty (does not contain an agent).

        Returns:
            True if the tile is empty, False otherwise.
        """
        return self.agent is None

    def display(self) -> str:
        """
        Determines the character to display for this tile.

        If an agent is on the tile, it returns the agent's display character.
        Otherwise, it returns the tile's default display character.

        Returns:
            A string character for display.
        """
        if self.agent:
            return self.agent.display()
        else:
            return self.display_character
