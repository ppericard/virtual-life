
class Tile:
    """
    Represents a single tile in the environment.
    """
    display_character = ' '

    def __init__(self, i, j):
        self.i = i
        self.j = j
        self.neighbors_by_distance = {}  # Dict to store neighbors by distance
        self.agent = None

    def set_neighbors(self, neighbors_dict):
        self.neighbors_by_distance = neighbors_dict

    def get_neighbors_at_distance(self, distance):
        """
        Returns a list of tiles exactly at the specified distance.
        """
        return self.neighbors_by_distance.get(distance, [])

    def get_neighbors(self, max_distance=1):
        """
        Returns a list of all neighboring tiles up to the specified maximum distance.
        """
        all_neighbors = set()
        for distance in range(1, max_distance + 1):
            neighbors_at_distance = self.neighbors_by_distance.get(distance, [])
            all_neighbors.update(neighbors_at_distance)
        return list(all_neighbors)
    
    def get_neighbors_agents(self, max_distance=1):
        return [tile.agent for tile in self.get_neighbors(max_distance) if tile.agent]

    def get_empty_neighbors(self, max_distance=1):
        return [tile for tile in self.get_neighbors(max_distance) if tile.is_empty()]

    def set_agent(self, agent):
        self.agent = agent

    def remove_agent(self):
        self.agent = None

    def is_empty(self):
        if self.agent:
            return False
        else:
            return True

    def display(self):
        if self.agent:
            return self.agent.display()
        else:
            return self.display_character
