VirtualLife
============

VirtualLife is a life simulator. 

It does NOT aim to perform a realistic and highly detailled simulation of the natural world. This simulator is rather oriented toward the computation of generations/turns given a virtual environment, some starting individuals/cells and a set of parameters and rules. 

It would allow to reproduce something as simple as Conway's Game of Life, to observe the emergence of ecological behaviors (predator–prey, mutualism, ...), or even to identify never observed non-natural patterns and behaviors.

## Development

We propose an iterative development with continuous delivery.

This project can be started with the simplest model and the naiviest implementation, to be later improved with additional functionnality, more complex models and computationnaly efficient implementations.

## Programming language

This project will be developped in object-oriented high-level programming language like Python3 (+ potentially the PyPy interpreter). Some computationaly intensive parts of the project could also be written in C. However the priority should first be given to the algorithmic optimization rather than the implementation one.

## Roadmap

This is a proposed roadmap, with ideas more or less structured and/or detailled.

1. Propose a model for an environment (a grid, for starters) and instances of a basic "cell" with minimal functionnality (move, copy itself, die)
2. Implement this simplist model as a finite-state automaton
3. Introduce the notion of "species", which are cells instanciated with different parameters, and that will have different behaviors
4. Add some ressources to the environment, and relevant functionnalities in the cells (ressources consumption, transformation, ...)

* Add some "social" functionnalities between the cells (interaction, ressource exchange, ...)
* Add a predator-prey interaction
* Add the sexual reproduction functionnality ()
