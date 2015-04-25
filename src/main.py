#! /usr/bin/python3

import os
import sys
import time
import random

from controller import Controller

if __name__ == '__main__':

    # Parameters
    environment_height = 35
    environment_width = 60
    populate_probability = 0.01

    #
    c = Controller(environment_height, environment_width, populate_probability)

    #
    c.run()
