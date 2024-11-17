"""
Simulation configuration parameters
"""
from typing import Dict, Any

SIMULATION_CONFIG: Dict[str, Any] = {
    'environment': {
        'height': 24,
        'width': 72,
        'populate_probability': 0.01
    },
    'display': {
        'frame_per_second': 20,
        'enable_colors': True
    },
    'cell': {
        'avg_life_expectancy': 200,
        'std_dev_life_expectancy': 100,
        'mutation_probability': 0.05,
        'action_probabilities': {
            'move': 1/10,
            'split': 1/200
        }
    }
} 