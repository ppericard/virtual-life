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
    environment_height = 35
    environment_width = 60
    populate_probability = 0.01
    # View parameters
    frame_per_second = 20

    # Controller initialization
    c = MyController(environment_height, environment_width,
                     populate_probability, frame_per_second)
    
    try:
        c.start()  # Assuming this method contains the while True loop
    except KeyboardInterrupt:
        print("\nSimulation interrupted by user.")


if __name__ == '__main__':
    # Set this to True to enable profiling by default (overridden by command-line switch)
    profiling_enabled = False

    # Check for command line argument for profiling
    if '--profile' in sys.argv:
        profiling_enabled = True # Enable profiling

    if profiling_enabled:
        profiler = cProfile.Profile()
        profiler.enable()

        main()

        profiler.disable()
        stats = pstats.Stats(profiler).sort_stats('time')
        stats.print_stats()
    else:
        main()
