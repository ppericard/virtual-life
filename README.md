VirtualLife
============

VirtualLife is a life simulator. 

It does NOT aim to perform a realistic and highly detailled simulation of the natural world. 
This simulator is rather oriented toward the computation of generations/turns given a virtual environment, some starting individuals/cells and a set of parameters and rules. 

It would allow to reproduce something as simple as Conway's Game of Life, to observe the emergence of ecological behaviors (predator–prey, mutualism, ...), or even to identify never observed non-natural patterns and behaviors.

## Getting Started

To run the simulation, navigate to the project's root directory and execute the following command:

```bash
python3 src/main.py
```

This will start the simulation with the default parameters defined in `config.py`.

### Performance Profiling

If you want to analyze the performance of the simulation, you can use the `--profile` flag:

```bash
python3 src/main.py --profile
```
This will run the simulation with `cProfile` and print performance statistics to the console after the simulation finishes.

## Configuration

The simulation's behavior is primarily controlled by parameters defined in the `config.py` file. You can modify these parameters to experiment with different scenarios.

The main configuration settings are stored in the `SIMULATION_CONFIG` dictionary and include:

*   **`environment`**:
    *   `height`: The height of the simulation grid.
    *   `width`: The width of the simulation grid.
    *   `populate_probability`: The initial probability (0.0 to 1.0) that a tile will be populated by a cell at the start of the simulation.
*   **`display`**:
    *   `frame_per_second`: The target frame rate for the simulation display in the terminal.
    *   `enable_colors`: (Currently implicitly True, but a placeholder for future GUI/display options) Whether to use colors in the display.
*   **`cell`**:
    *   `avg_life_expectancy`: The average lifespan of a cell in simulation steps (turns).
    *   `std_dev_life_expectancy`: The standard deviation of a cell's lifespan.
    *   `mutation_probability`: The probability (0.0 to 1.0) that a cell's offspring will mutate its display character upon splitting.
    *   `action_probabilities`: A dictionary defining the base probabilities for different cell actions:
        *   `move`: Probability of a cell attempting to move to an adjacent empty tile.
        *   `split`: Probability of a cell attempting to split and create an offspring in an adjacent empty tile.

Modifying these values in `config.py` will directly impact how the simulation unfolds, allowing for a wide range of observable behaviors and complexities.

## Project Structure

The project is organized as follows:

*   `src/`: Contains all the source code for the simulation.
    *   `main.py`: The main entry point for the application. It initializes and starts the simulation controller.
    *   `config.py`: Defines the global configuration parameters for the simulation.
    *   `controller.py`: Manages the main simulation loop, connecting the model and the view, and handling timing.
    *   `model.py`: Contains the core simulation logic, including the environment grid, agent management, and rules for how agents interact and evolve.
    *   `agent.py`: Defines the `Agent` abstract base class and the concrete `Cell` class, which dictates agent behavior (movement, splitting, aging, etc.).
    *   `environment.py`: Defines the `Tile` class, representing individual units of the simulation grid.
    *   `view.py`: Responsible for rendering the simulation state to the terminal, including colored output for different agents.
    *   `observer.py`: Implements the Observer design pattern, though it's currently minimally used by `MyView` (the controller directly calls view updates).
*   `README.md`: This file, providing an overview of the project.
*   `LICENSE`: Contains the license information for the project.
*   `.gitignore`: Specifies intentionally untracked files that Git should ignore.
*   `tests/`: (Placeholder) Intended for future unit and integration tests.

## Development

We propose an iterative development with continuous delivery.

This project can be started with the simplest model and the naiviest implementation, to be later improved with additional functionnality, more complex models and computationnaly efficient implementations.

This project will be developped in object-oriented high-level programming language like Python3 (+ potentially the PyPy interpreter). 
Some computationaly intensive parts of the project could also be written in C. 
However the priority should first be given to the algorithmic optimization rather than the implementation one.

## Roadmap

This is a proposed roadmap, with ideas more or less structured and/or detailled.

1. Propose a model for an environment (a grid, for starters) and instances of a basic "cell" with minimal functionnality (move, copy itself, die)
2. Implement this simplist model as a finite-state automaton
3. Introduce the notion of "species", which are cells instanciated with different parameters, and that will have different behaviors
4. Add some ressources to the environment, and relevant functionnalities in the cells (ressources consumption, transformation, ...)

* Add some "social" functionnalities between the cells (interaction, ressource exchange, ...)
* Add a predator-prey interaction
* Add the sexual reproduction functionnality

* Once a working prototype is available and manually configurable, an automated upper layer could be added. Using machine learning approaches, this automated layer could run multiple instances of the simulation under varying starting conditions and help identify "interesting" scenarios (extended running life of the species, complex behavior ermergence, ...)

