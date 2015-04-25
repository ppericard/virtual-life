#! /usr/bin/python3

import os
import sys
import time

# import pudb

from controller import MyController

if __name__ == '__main__':

    # pu.db

    # Parameters
    environment_height = 35
    environment_width = 60
    populate_probability = 0.01

    #
    c = MyController(environment_height, environment_width, populate_probability)

    #
    c.start()
