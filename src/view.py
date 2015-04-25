
from observer import *
from agent_view import *
from inanimate_view import *
from environment_view import *

class MyView(Observer):

    def __init__(self, controller, model):
        self.controller = controller
        self.model = model
        #
        self.env_matrix_view = list()
        self.__init_env_matrix_view()
        #
        self.agent_list_view = list()
        self.__init_agent_list()

    def __init_env_matrix_view(self):
        self.env_matrix_view = [[TileView(self.model.get_tile_at_position(i, j)) 
                                 for j in range(self.model.env_width)] 
                                 for i in range(self.model.env_height)]

    def __init_agent_list(self):
        for agent in self.model.agent_list:
            self.agent_list_view.append(AgentView(agent))
        









