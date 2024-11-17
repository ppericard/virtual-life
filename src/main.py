#! /usr/bin/python3

import cProfile
import pstats
import random
import sys
from controller import MyController

def main():
    # Random seed initialization
    random.seed()

    # Environment parameters
    environment_height = 24
    environment_width = 72
    populate_probability = 0.01

    # View parameters
    frame_per_second = 20

    # Controller initialization and start
    controller = MyController(environment_height, environment_width, populate_probability, frame_per_second)
    controller.run()

if __name__ == '__main__':
    profiling_enabled = False
    if '--profile' in sys.argv:
        profiling_enabled = True

    if profiling_enabled:
        with cProfile.Profile() as profiler:
            main()
            stats = pstats.Stats(profiler).sort_stats('time')
            stats.print_stats()
    else:
        main()
