
from observer import *
from model import MyModel
from view import MyView

import os
import random
import time

class MyController():

    def __init__(self, env_height, env_width, 
                 populate_proba, try_strength,
                 frame_per_second):
        self.model = MyModel(env_height, env_width, populate_proba)
        self.view = MyView(self.model, frame_per_second)
        self.try_strength = try_strength

    def update_view(self):
        self.view.update()

    def run_next_step(self):
        for agent in self.model.agent_list:
            agent.next_step(self.try_strength)

    def start(self):
        frame_time_start = time.perf_counter()
        while (True):
            tmp_time = time.perf_counter()
            os.system('clear')
            self.update_view()
            frame_time = time.perf_counter() - frame_time_start
            frame_time_start = time.perf_counter()
            self.run_next_step()
            elapsed_time = time.perf_counter() - tmp_time
            print("sim_time={0} sec".format(elapsed_time))
            # print("FPS={0}".format(self.view.frame_per_second))
            # print("frame_duration_in_sec={0}".format(self.view.frame_duration_in_sec))
            # print("frame_time={0} sec".format(frame_time))
            time_left_to_next_frame = self.view.frame_duration_in_sec - elapsed_time
            if (time_left_to_next_frame > 0):
                # print("sleep_time={0}".format(time_left_to_next_frame))
                time.sleep(time_left_to_next_frame)
