"""
This module defines the `MyView` class, responsible for rendering the simulation state.

It observes the `MyModel` and updates the display in the terminal when notified.
The view uses ANSI escape codes to display agents with different colors based on their
display characters.
"""
import sys
from typing import Dict # For type hinting
from observer import Observer # MyView is an Observer
from model import MyModel # MyView observes MyModel

class MyView(Observer):
    """
    Handles the visual representation of the simulation model in the terminal.

    It displays the environment grid, with agents represented by their characters
    and styled with colors using ANSI escape codes. It also shows basic simulation
    information like grid dimensions and agent count.

    Attributes:
        model (MyModel): The simulation model instance it observes.
        frame_per_second (int): The target frame rate for display updates.
        frame_duration_in_sec (float): The reciprocal of `frame_per_second`.
        color_map (Dict[str, str]): A mapping from agent display characters
                                    to ANSI color codes.
        reset_code (str): ANSI code to reset text formatting.
    """

    # Mapping of agent display characters to ANSI color codes for terminal output
    color_map: Dict[str, str] = {
        '*': '\033[31m',  # Red
        '#': '\033[32m',  # Green
        '@': '\033[33m',  # Yellow
        '+': '\033[34m',  # Blue
        '%': '\033[35m',  # Magenta
        '&': '\033[36m',  # Cyan
        '?': '\033[37m',  # White
        '!': '\033[91m',  # Bright Red
        '$': '\033[92m',  # Bright Green
        '§': '\033[93m',  # Bright Yellow
        '~': '\033[94m',  # Bright Blue
        '=': '\033[95m',  # Bright Magenta
        '-': '\033[96m',  # Bright Cyan
        '_': '\033[97m',  # Bright White
        ':': '\033[90m',  # Dark Gray (Black)
        ';': '\033[41m',  # Background Red (example, might not be used directly for char)
        '^': '\033[42m',  # Background Green (example)
        '>': '\033[44m',  # Background Blue (example)
        '<': '\033[45m',  # Background Magenta (example)
        '|': '\033[7m',   # Invert Background and Foreground (Reverse video)
    }
    reset_code: str = '\033[0m'  # ANSI code to reset all formatting attributes

    def __init__(self, model: MyModel, frame_per_second: int):
        """
        Initializes the MyView instance.

        Args:
            model: The `MyModel` instance that this view will observe and display.
            frame_per_second: The target number of frames (display updates) per second.
        """
        # super().__init__(model) # If MyView were to register itself with the model as an observer
        self.model: MyModel = model
        self.frame_per_second: int = frame_per_second
        self.frame_duration_in_sec: float = 1.0 / frame_per_second if frame_per_second > 0 else 0

    def update(self, observable: MyModel = None, *args, **kwargs):
        """
        Called when the observed model notifies the view of changes.
        Triggers a redisplay of the simulation state.

        Args:
            observable: The model that triggered the update. Typically self.model.
            *args: Additional arguments from the observable (not used here).
            **kwargs: Additional keyword arguments from the observable (not used here).
        """
        # In a true Observer pattern, the controller might call model.notify_observers()
        # and this method would be triggered. Here, it's directly called by the controller.
        self.display()

    def display(self):
        """
        Renders the current state of the simulation model to the standard output (terminal).

        It prints the grid dimensions, then iterates through each tile of the model's
        environment matrix. For each tile, it determines whether to display an agent's
        character (with color) or the empty tile character. Finally, it prints the
        total number of active agents.
        """
        # Display environment dimensions
        print(f'Environment Dimensions: Height={self.model.env_height}, Width={self.model.env_width}')

        active_agents_count: int = 0 # Counter for active agents

        # Iterate through each row and column of the environment grid
        for r in range(self.model.env_height): # r for row
            row_display_chars: List[str] = []
            for c in range(self.model.env_width): # c for column
                tile = self.model.get_tile_at_position(r, c)
                display_char_for_tile: str
                if not tile.is_empty() and tile.agent is not None: # Check if tile has an agent
                    active_agents_count += 1
                    agent_char = tile.agent.display()
                    # Get the ANSI color code for the agent's character, default to no color
                    color_code = self.color_map.get(agent_char, '')
                    # Construct the colored character string
                    if color_code:
                        display_char_for_tile = f"{color_code}{agent_char}{self.reset_code}"
                    else:
                        display_char_for_tile = agent_char
                else:
                    # Display the tile's default character (e.g., a space for empty)
                    display_char_for_tile = tile.display()
                row_display_chars.append(display_char_for_tile)
            
            # Join characters for the row and print, ensuring a space between them
            sys.stdout.write(' '.join(row_display_chars) + '\n')

        # Display the total number of active agents
        print(f'Total Active Agents: {active_agents_count}')
        sys.stdout.flush() # Ensure output is immediately visible
