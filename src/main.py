#! /usr/bin/python3

import os
import sys
import time
import random
import cProfile

from controller import MyController

def main():

    # Random seed initialization
    random.seed(1)
    # random.seed()

    # Environment parameters
    environment_height = 35
    environment_width = 60
    populate_probability = 0.01
    # View parameters
    frame_per_second = 20

    #
    c = MyController(environment_height, environment_width,
                     populate_probability, frame_per_second)

    #
    c.start()


if __name__ == '__main__':

    main()
    # cProfile.run('main()')
