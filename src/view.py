import sys

from observer import *

class MyView(Observer):

    # Mapping of characters to ANSI color codes
    color_map = {
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
        ':': '\033[90m',  # Dark Gray
        ';': '\033[41m',  # Background Red
        '^': '\033[42m',  # Background Green
        '>': '\033[44m',  # Background Blue
        '<': '\033[45m',  # Background Magenta
        '|': '\033[7m',   # Invert Background and Foreground
    }
    reset_code = '\033[0m'  # Reset color

    def __init__(self, model, frame_per_second):
        self.model = model
        self.frame_per_second = frame_per_second
        self.frame_duration_in_sec = 1 / frame_per_second

    def update(self):
        self.display()

    def display(self):

        print('height={0}, width={1}'.format(self.model.env_height, self.model.env_width))

        agents_nb = 0

        for i in range(self.model.env_height):
            for j in range(self.model.env_width):
                tile = self.model.get_tile_at_position(i,j)
                if not tile.is_empty():
                    agents_nb += 1
                    # Get the color code for the agent's character
                    color_code = self.color_map.get(tile.agent.display(), '')
                    display_char = color_code + tile.agent.display() + self.reset_code
                else:
                    display_char = tile.display()
                sys.stdout.write(f'{display_char} ')
            sys.stdout.write('\n')

        print('agents_nb={0}'.format(agents_nb))