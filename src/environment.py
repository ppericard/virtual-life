
class Tile:

    def __init__(self):
        self.neighbors_list = list()
        self.agent = None

    def add_neighbor(self, neighbor_tile):
        self.neighbors_list.append(neighbor_tile)

    def add_list_neighbors(self, list_neighbors):
        self.neighbors_list.extend(list_neighbors)

    def get_neighbors(self):
        return self.neighbors_list()

    def set_agent(self, agent):
        self.agent = agent

    def remove_agent(self):
        self.agent = None


