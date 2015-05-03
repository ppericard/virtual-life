
from observer import *
from model import MyModel
from view import MyView

import random

class MyController():

    def __init__(self, env_height, env_width, populate_proba):
        # Random seed initialization
        # random.seed(1)
        random.seed()
        #
        self.model = MyModel(env_height, env_width, populate_proba)
        self.view = MyView(self.model)

    def update_view(self):
        self.view.update()

    def start(self):
        self.update_view()


