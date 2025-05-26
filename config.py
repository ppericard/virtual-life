"""
This module defines the configuration parameters for the simulation.

It includes settings for the environment, display, and cell behavior.
These parameters can be adjusted to change the simulation's dynamics.
"""
from typing import Dict, Any

# Global dictionary holding all simulation settings
SIMULATION_CONFIG: Dict[str, Any] = {
    # Environment settings
    'environment': {
        'height': 24,  # Height of the simulation grid
        'width': 72,  # Width of the simulation grid
        'populate_probability': 0.01  # Initial probability of a cell being populated
    },
    # Display settings
    'display': {
        'frame_per_second': 20,  # Target frames per second for the simulation display
        'enable_colors': True  # Whether to use colors in the display
    },
    # Cell-specific settings
    'cell': {
        'avg_life_expectancy': 200,  # Average lifespan of a cell in simulation steps
        'std_dev_life_expectancy': 100,  # Standard deviation of cell lifespan
        'mutation_probability': 0.05,  # Probability of a cell mutating upon splitting
        'action_probabilities': {  # Probabilities for different cell actions
            'move': 1/10,  # Probability of a cell moving in a given step
            'split': 1/200  # Probability of a cell splitting in a given step
        }
    }
}