
from observer import *
from model import MyModel
from view import MyView

import os
import platform
import random
import time

class MyController():

    def __init__(self, env_height, env_width,
                 populate_proba, frame_per_second):
        self.model = MyModel(env_height, env_width, populate_proba)
        self.view = MyView(self.model, frame_per_second)
        self.clear_command = 'cls' if platform.system() == "Windows" else 'clear'

    def update_view(self):
        os.system(self.clear_command)
        self.view.update()

    def start(self):
        frame_time_start = time.perf_counter()
        self.update_view()
        while (True):
            sim_time_start = time.perf_counter()
            self.model.run_simulation_step()
            sim_running_time = time.perf_counter() - sim_time_start

            self.update_view()
            frame_time = time.perf_counter() - frame_time_start
            frame_time_start = time.perf_counter()

            print("sim_time={0} sec".format(sim_running_time))
            print("FPS={0}".format(self.view.frame_per_second))
            print("frame_duration_in_sec={0}".format(self.view.frame_duration_in_sec))
            print("frame_time={0} sec".format(frame_time))

            time_left_to_next_frame = self.view.frame_duration_in_sec - (time.perf_counter() - sim_time_start)
            if (time_left_to_next_frame > 0):
                time.sleep(time_left_to_next_frame)
