
class Tile:
    """
    Represents a single tile in the environment.
    """
    display_character = ' '

    def __init__(self):
        self.neighbors_list = []
        self.agent = None

    def add_neighbor(self, neighbor_tile):
        self.neighbors_list.append(neighbor_tile)

    def add_list_neighbors(self, list_neighbors):
        self.neighbors_list.extend(list_neighbors)

    def get_neighbors(self):
        return self.neighbors_list
    
    def get_neighbors_agents(self):
        return [tile.agent for tile in self.neighbors_list if tile.agent]

    def get_empty_neighbors(self):
        return [tile for tile in self.neighbors_list if tile.is_empty()]

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
