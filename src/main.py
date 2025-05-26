#! /usr/bin/python3
"""
This module serves as the main entry point for the simulation application.

It handles:
- Initialization of the random number generator.
- Setting up default simulation parameters (environment size, population density, FPS).
- Parsing command-line arguments (e.g., for enabling profiling).
- Creating and starting the main simulation controller (`MyController`).
- Optionally running the simulation under `cProfile` for performance analysis.
"""

import cProfile
import pstats
import random
import sys
from controller import MyController # The main controller for the simulation
from config import SIMULATION_CONFIG # Import global configuration

def main():
    """
    Sets up and runs the simulation.

    Initializes simulation parameters (either defaults or from a config),
    creates the `MyController`, and starts the simulation loop.
    """
    # Random seed initialization (can be made configurable for repeatable simulations)
    random.seed() # Uses current system time by default if no argument is provided.

    # Environment parameters from global config
    env_config = SIMULATION_CONFIG['environment']
    environment_height = env_config['height']
    environment_width = env_config['width']
    populate_probability = env_config['populate_probability']

    # View parameters from global config
    display_config = SIMULATION_CONFIG['display']
    frame_per_second = display_config['frame_per_second']

    # Initialize and start the controller
    # The controller orchestrates the model (simulation logic) and view (display)
    print("Initializing controller...")
    controller = MyController(
        env_height=environment_height,
        env_width=environment_width,
        populate_proba=populate_probability,
        frame_per_second=frame_per_second
    )
    print("Starting simulation run...")
    controller.run()
    print("Simulation finished.")

if __name__ == '__main__':
    # This block executes when the script is run directly.
    profiling_enabled: bool = False
    # Check for '--profile' command-line argument to enable profiling
    if '--profile' in sys.argv:
        profiling_enabled = True
        print("Profiling enabled.")

    if profiling_enabled:
        # Run the main function under cProfile
        print("Running with profiler...")
        with cProfile.Profile() as profiler:
            main()
        
        # After execution, print the profiling statistics
        print("\nProfiling Results:")
        stats = pstats.Stats(profiler)
        stats.sort_stats(pstats.SortKey.TIME) # Sort by total time spent in function
        stats.print_stats(20) # Print the top 20 time-consuming functions
    else:
        # Run the main function normally without profiling
        main()
