#! /usr/bin/python3

import os
import sys
import time
import random

from controller import MyController

if __name__ == '__main__':

    # Random seed initialization
    random.seed(1)
    # random.seed()

    # Environment parameters
    environment_height = 35
    environment_width = 60
    populate_probability = 0.01
    # Agents parameters
    try_strength = 3
    # View parameters
    frame_per_second = 10

    #
    c = MyController(environment_height, environment_width, 
                     populate_probability, try_strength,
                     frame_per_second)

    #
    c.start()
