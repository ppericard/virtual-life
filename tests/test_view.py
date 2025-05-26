import unittest
from unittest.mock import Mock, patch, call
import sys
import os

# Adjust sys.path to include the src directory
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../src')))

from view import MyView
from model import MyModel # MyView observes MyModel
from agent import Agent # For creating mock agents
from environment import Tile # For creating mock tiles
from config import SIMULATION_CONFIG

class TestMyView(unittest.TestCase):
    """Tests for the MyView class."""

    def setUp(self):
        self.mock_model = Mock(spec=MyModel)
        self.mock_model.env_height = 3 # Small grid for testing display
        self.mock_model.env_width = 2
        
        self.fps = SIMULATION_CONFIG['display']['frame_per_second']
        self.view = MyView(self.mock_model, self.fps)

    def test_view_initialization(self):
        self.assertEqual(self.view.model, self.mock_model)
        self.assertEqual(self.view.frame_per_second, self.fps)
        self.assertAlmostEqual(self.view.frame_duration_in_sec, 1.0 / self.fps if self.fps > 0 else 0)

    def test_update_calls_display(self):
        with patch.object(self.view, 'display') as mock_display:
            self.view.update() # Directly calling update, assuming it's triggered
            mock_display.assert_called_once()

        # Test with arguments (as per Observer pattern)
        with patch.object(self.view, 'display') as mock_display:
            self.view.update(self.mock_model, "arg1", kwarg1="kwval1")
            mock_display.assert_called_once()


    @patch('sys.stdout.write')
    @patch('builtins.print') # To capture print calls for dimensions and agent count
    def test_display_empty_grid(self, mock_print, mock_stdout_write):
        # Mock get_tile_at_position to return empty tiles
        empty_tile_char = Tile.display_character # Usually ' '
        
        def mock_get_tile(r, c):
            tile = Mock(spec=Tile)
            tile.is_empty.return_value = True
            tile.agent = None # Ensure agent is None
            tile.display.return_value = empty_tile_char
            return tile
        
        self.mock_model.get_tile_at_position.side_effect = mock_get_tile
        
        self.view.display()

        # Check print calls for dimensions and agent count
        expected_dimension_print = call(f'Environment Dimensions: Height={self.mock_model.env_height}, Width={self.mock_model.env_width}')
        expected_agent_count_print = call(f'Total Active Agents: {0}')
        
        mock_print.assert_any_call(f'Environment Dimensions: Height={self.mock_model.env_height}, Width={self.mock_model.env_width}')
        mock_print.assert_any_call(f'Total Active Agents: {0}')

        # Check stdout.write calls for the grid
        # For 3x2 grid, 3 rows. Each row: char space char newline.
        # Example for one row: '  ' + '\n' (if default char is ' ')
        expected_row_calls = []
        for r in range(self.mock_model.env_height):
            # expected_row_str = (empty_tile_char + ' ') * (self.mock_model.env_width -1) + empty_tile_char
            expected_row_str = (empty_tile_char + ' ') * (self.mock_model.env_width-1) + empty_tile_char if self.mock_model.env_width >0 else ""
            expected_row_calls.append(call(expected_row_str.strip() + '\n')) # strip to handle potential leading/trailing space from join logic

        # The actual calls might be slightly different depending on how ' '.join works with a list of single chars
        # Let's check the content of calls to stdout_write
        # Expected calls: one per row for content, one per row for newline
        # print(mock_stdout_write.call_args_list)
        
        # For a 3x2 grid, we expect 3 rows of "char char"
        # Example content: "  " (2 chars + space)
        # The current implementation joins with ' ' and then adds '\n'
        # So for a row of [' ', ' '], it becomes '  ' + '\n'
        
        calls = mock_stdout_write.call_args_list
        self.assertEqual(len(calls), self.mock_model.env_height) # One write per row
        
        for r in range(self.mock_model.env_height):
            expected_content_for_row = ' '.join([empty_tile_char] * self.mock_model.env_width) + '\n'
            self.assertEqual(calls[r][0][0], expected_content_for_row)


    @patch('sys.stdout.write')
    @patch('builtins.print')
    def test_display_grid_with_agents(self, mock_print, mock_stdout_write):
        # Mock tiles and agents
        mock_agent1 = Mock(spec=Agent)
        mock_agent1.display.return_value = 'A'
        
        mock_agent2 = Mock(spec=Agent)
        mock_agent2.display.return_value = '*' # A character in color_map

        # (0,0) has agent1, (1,1) has agent2, others empty
        # Grid:
        # A .
        # . *
        # . . 
        # (assuming default empty Tile char is '.')
        
        empty_char = Tile.display_character
        reset_code = MyView.reset_code
        color_for_star = MyView.color_map.get('*', '')


        def mock_get_tile_with_agents(r, c):
            tile = Mock(spec=Tile)
            tile.agent = None
            tile.is_empty.return_value = True
            tile.display.return_value = empty_char

            if r == 0 and c == 0:
                tile.agent = mock_agent1
                tile.is_empty.return_value = False
                # tile.display.return_value = 'A' # This is not called if agent exists
            elif r == 1 and c == 1:
                tile.agent = mock_agent2
                tile.is_empty.return_value = False
                # tile.display.return_value = '*' # Not called
            return tile

        self.mock_model.get_tile_at_position.side_effect = mock_get_tile_with_agents
        
        self.view.display()

        mock_print.assert_any_call(f'Environment Dimensions: Height={self.mock_model.env_height}, Width={self.mock_model.env_width}')
        mock_print.assert_any_call(f'Total Active Agents: {2}')

        # Expected output to stdout.write (3 rows for 3x2 grid)
        # Row 0: "A  " (Agent A, then empty tile)
        # Row 1: ". \033[31m*\033[0m" (Empty tile, then Agent *)
        # Row 2: ". ." (Empty, Empty)
        
        calls = mock_stdout_write.call_args_list
        self.assertEqual(len(calls), self.mock_model.env_height) # 3 calls for 3 rows

        # Row 0: Agent 'A' (no color by default) + space + empty_char
        self.assertEqual(calls[0][0][0], f"A {empty_char}\n")
        # Row 1: empty_char + space + Agent '*' (with color)
        self.assertEqual(calls[1][0][0], f"{empty_char} {color_for_star}*{reset_code}\n")
        # Row 2: empty_char + space + empty_char
        self.assertEqual(calls[2][0][0], f"{empty_char} {empty_char}\n")
        
    def test_color_map_usage(self):
        # This is implicitly tested in test_display_grid_with_agents
        # We can add a more direct check if a character is in color_map
        char_in_map = '*'
        char_not_in_map = 'Z'
        
        self.assertIn(char_in_map, self.view.color_map)
        # If char_not_in_map is not in color_map, get returns '' (empty string)
        self.assertEqual(self.view.color_map.get(char_not_in_map, ''), '')


if __name__ == '__main__':
    unittest.main()
