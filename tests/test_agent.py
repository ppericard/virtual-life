import unittest
from unittest.mock import Mock, patch
import sys
import os
import random

# Adjust sys.path to include the src directory
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../src')))

from agent import Agent, Cell
from environment import Tile
from config import SIMULATION_CONFIG

class TestAgentBehavior(unittest.TestCase): # Renamed to avoid conflict with Agent class
    """Tests for the abstract Agent class behaviors, typically tested via Cell."""

    def setUp(self):
        self.mock_tile = Mock(spec=Tile)
        self.mock_tile.i = 0 # Mock tile coordinates
        self.mock_tile.j = 0
        self.mock_tile.agent = None # Initialize agent attribute for the mock tile
        self.mock_tile.get_empty_neighbors.return_value = []
        self.mock_tile.get_neighbors_agents.return_value = []
        self.mock_death_callback = Mock()
        self.mock_born_callback = Mock()

        # Use a concrete subclass for testing Agent's non-abstract methods
        # Many Agent tests will be through Cell instances
        self.agent = Cell(tile=self.mock_tile, 
                           death_callback=self.mock_death_callback, 
                           born_callback=self.mock_born_callback)
        # Reset class variables for Cell to use config for predictability in tests
        Cell.avg_life_expectancy = SIMULATION_CONFIG['cell']['avg_life_expectancy']
        Cell.std_dev_life_expectancy = SIMULATION_CONFIG['cell']['std_dev_life_expectancy']
        Cell.default_mutation_probability = SIMULATION_CONFIG['cell']['mutation_probability']
        Cell.action_probabilities = SIMULATION_CONFIG['cell']['action_probabilities'].copy()


    def test_agent_initialization(self):
        self.assertEqual(self.agent.current_tile, self.mock_tile)
        self.assertGreater(self.agent.turns_to_live, 0) 
        self.assertTrue(0 < self.agent.turns_to_live < Cell.avg_life_expectancy + 4 * Cell.std_dev_life_expectancy, 
                        f"turns_to_live ({self.agent.turns_to_live}) seems implausible.")
        self.assertIn(self.agent.display_character, Cell.possible_characters)
        self.assertEqual(self.agent.mutation_probability, Cell.default_mutation_probability)
        self.assertEqual(self.agent.death_callback, self.mock_death_callback)
        self.assertEqual(self.agent.born_callback, self.mock_born_callback)

        custom_mut_prob = 0.75
        agent_custom_mut = Cell(self.mock_tile, mutation_probability=custom_mut_prob)
        self.assertEqual(agent_custom_mut.mutation_probability, custom_mut_prob)

    def test_life_expectancy_generation(self):
        lifespans = [self.agent._life_expectancy() for _ in range(100)]
        for ls in lifespans:
            self.assertGreaterEqual(ls, 0) 
        
        mean_lifespan = sum(lifespans) / len(lifespans)
        expected_min = Cell.avg_life_expectancy - 3 * Cell.std_dev_life_expectancy
        expected_max = Cell.avg_life_expectancy + 3 * Cell.std_dev_life_expectancy
        self.assertTrue(expected_min < mean_lifespan < expected_max, 
                        f"Mean lifespan {mean_lifespan} is far from expected {Cell.avg_life_expectancy}")

    def test_is_dead(self):
        self.agent._turns_to_live = 10
        self.assertFalse(self.agent.is_dead())
        self.agent._turns_to_live = 0
        self.assertTrue(self.agent.is_dead())
        self.agent._turns_to_live = -1
        self.assertTrue(self.agent.is_dead())

    def test_set_new_tile(self):
        new_mock_tile = Mock(spec=Tile)
        new_mock_tile.i = 1
        new_mock_tile.j = 1
        
        original_tile = self.agent.current_tile
        self.agent.set_new_tile(new_mock_tile)
        
        self.assertEqual(self.agent.current_tile, new_mock_tile)
        self.assertEqual(self.agent.previous_tile, original_tile)
        
        with self.assertRaises(ValueError):
            self.agent.set_new_tile(None)

    def test_move_no_empty_neighbors(self):
        self.mock_tile.get_empty_neighbors.return_value = [] 
        initial_tile = self.agent.current_tile
        self.agent.move()
        self.assertEqual(self.agent.current_tile, initial_tile) 
        self.mock_tile.remove_agent.assert_not_called()

    @patch('random.choice', side_effect=lambda x: x[0]) 
    def test_move_with_empty_neighbors(self, mock_random_choice):
        mock_neighbor_tile = Mock(spec=Tile)
        mock_neighbor_tile.i = 0; mock_neighbor_tile.j = 1; 
        self.mock_tile.get_empty_neighbors.return_value = [mock_neighbor_tile]
        
        with patch.object(self.agent, 'set_new_tile', wraps=self.agent.set_new_tile) as spy_set_new_tile:
            self.agent.move()
            self.mock_tile.remove_agent.assert_called_once()
            mock_neighbor_tile.set_agent.assert_called_once_with(self.agent)
            spy_set_new_tile.assert_called_once_with(mock_neighbor_tile)
            self.assertEqual(self.agent.current_tile, mock_neighbor_tile)

    def test_split_no_empty_neighbors(self):
        self.mock_tile.get_empty_neighbors.return_value = []
        self.agent.split()
        self.mock_born_callback.assert_not_called()

    @patch('random.random') 
    @patch('random.choice', side_effect=lambda x: x[0]) 
    def test_split_no_mutation(self, mock_random_choice_tile_char, mock_random_float):
        mock_random_float.return_value = 0.5 
        self.agent.mutation_probability = 0.1 

        mock_neighbor_tile_for_split = Mock(spec=Tile)
        self.mock_tile.get_empty_neighbors.return_value = [mock_neighbor_tile_for_split]
        
        original_possible_chars = Cell.possible_characters
        Cell.possible_characters = ['A', 'B', 'C']
        self.agent.display_character = 'A' 

        self.agent.split()
        
        mock_neighbor_tile_for_split.set_agent.assert_called_once()
        new_agent_arg = mock_neighbor_tile_for_split.set_agent.call_args[0][0]
        
        self.assertIsInstance(new_agent_arg, self.agent.__class__)
        self.assertEqual(new_agent_arg.display_character, self.agent.display_character) 
        self.assertEqual(new_agent_arg.death_callback, self.mock_death_callback)
        self.assertEqual(new_agent_arg.born_callback, self.mock_born_callback)
        self.assertEqual(new_agent_arg.mutation_probability, self.agent.mutation_probability)
        self.mock_born_callback.assert_called_once_with(new_agent_arg)
        Cell.possible_characters = original_possible_chars

    @patch('random.random') 
    @patch('random.choice') 
    def test_split_with_mutation(self, mock_random_choice, mock_random_float):
        mock_random_float.return_value = 0.01 
        self.agent.mutation_probability = 0.9 

        mock_neighbor_tile_for_split = Mock(spec=Tile)
        self.mock_tile.get_empty_neighbors.return_value = [mock_neighbor_tile_for_split]
        
        original_possible_chars = Cell.possible_characters
        Cell.possible_characters = ['X', 'Y', 'Z'] 
        self.agent.display_character = 'X'
        mutated_char_options = [c for c in Cell.possible_characters if c != self.agent.display_character]
        if not mutated_char_options: self.skipTest("Not enough characters for mutation test.")
        mock_random_choice.side_effect = [mock_neighbor_tile_for_split, mutated_char_options[0]]

        self.agent.split()
        
        mock_neighbor_tile_for_split.set_agent.assert_called_once()
        new_agent_arg = mock_neighbor_tile_for_split.set_agent.call_args[0][0]
        
        self.assertIsInstance(new_agent_arg, self.agent.__class__)
        self.assertNotEqual(new_agent_arg.display_character, self.agent.display_character)
        self.assertIn(new_agent_arg.display_character, mutated_char_options)
        self.mock_born_callback.assert_called_once_with(new_agent_arg)
        Cell.possible_characters = original_possible_chars

    def test_cell_initialization_mutation(self):
        original_chars = Cell.possible_characters
        Cell.possible_characters = ['A', 'B']
        with patch('random.choice', side_effect=['A', 'B']) as mock_rand_choice:
            cell_a = Cell(self.mock_tile, display_character='A')
            self.assertEqual(cell_a.display_character, 'A')
            mock_rand_choice.reset_mock(side_effect=True) 
            mock_rand_choice.side_effect = ['B']
            cell_b = Cell(self.mock_tile)
            self.assertEqual(cell_b.display_character, 'B')
            mock_rand_choice.assert_called_once()
        Cell.possible_characters = original_chars

class TestCellSpecificBehavior(unittest.TestCase):
    def setUp(self):
        self.mock_tile = Mock(spec=Tile)
        self.mock_tile.i = 0; self.mock_tile.j = 0;
        self.mock_tile.get_empty_neighbors.return_value = []
        self.mock_tile.get_neighbors_agents.return_value = []
        
        Cell.avg_life_expectancy = SIMULATION_CONFIG['cell']['avg_life_expectancy']
        Cell.std_dev_life_expectancy = SIMULATION_CONFIG['cell']['std_dev_life_expectancy']
        Cell.default_mutation_probability = SIMULATION_CONFIG['cell']['mutation_probability']
        Cell.action_probabilities = SIMULATION_CONFIG['cell']['action_probabilities'].copy()

        self.cell = Cell(self.mock_tile)
        self.cell.display_character = 'X' 

    def test_calculate_adjusted_split_probability_generic_agent(self):
        base_split_prob = Cell.action_probabilities['split'] 
        self.mock_tile.get_neighbors_agents.return_value = []
        prob = self.cell.calculate_adjusted_split_probability()
        self.assertAlmostEqual(prob, min(max(base_split_prob * (1 + 3/20), base_split_prob / 2), base_split_prob * 2))
        self.assertTrue(0 <= prob <= 1)

        mock_agent_list = [Mock(spec=Agent, display_character=str(i)) for i in range(5)]
        self.mock_tile.get_neighbors_agents.return_value = mock_agent_list
        prob = self.cell.calculate_adjusted_split_probability()
        self.assertAlmostEqual(prob, min(max(base_split_prob * (1 - 2/20), base_split_prob / 2), base_split_prob * 2))
        self.assertTrue(0 <= prob <= 1)

        mock_agent_list_kin = [Mock(spec=Agent, display_character='X') for _ in range(2)]
        self.mock_tile.get_neighbors_agents.return_value = mock_agent_list_kin
        prob = self.cell.calculate_adjusted_split_probability()
        self.assertAlmostEqual(prob, min(max(base_split_prob * (1 + 2/20) * (1 + 1/20), base_split_prob / 2), base_split_prob * 2))
        self.assertTrue(0 <= prob <= 1)

    @patch('random.random')
    def test_draw_action(self, mock_random_float):
        with patch.object(self.cell, 'calculate_adjusted_split_probability', return_value=0.0):
            Cell.action_probabilities = {'move': 1.0, 'split': 0.0}
            mock_random_float.return_value = 0.05
            self.assertEqual(self.cell.draw_action(), 'move')

        with patch.object(self.cell, 'calculate_adjusted_split_probability', return_value=1.0):
            Cell.action_probabilities = {'move': 0.0, 'split': 1.0}
            mock_random_float.return_value = 0.05
            self.assertEqual(self.cell.draw_action(), 'split')
            
        with patch.object(self.cell, 'calculate_adjusted_split_probability', return_value=0.0):
            Cell.action_probabilities = {'move': 0.0, 'split': 0.0}
            mock_random_float.return_value = 0.5
            self.assertEqual(self.cell.draw_action(), 'rest')
            
        Cell.action_probabilities = SIMULATION_CONFIG['cell']['action_probabilities'].copy()

    @patch.object(Cell, 'move', autospec=True)
    @patch.object(Cell, 'split', autospec=True)
    @patch.object(Cell, 'draw_action', autospec=True)
    @patch.object(Cell, 'adjust_life_expectancy_based_on_neighbors', autospec=True)
    def test_next_step_actions(self, mock_adjust_life_method, mock_draw_action_method, mock_split_method, mock_move_method):
        initial_ttl = self.cell.turns_to_live
        mock_draw_action_method.return_value = 'move'
        self.cell.next_step() 
        mock_adjust_life_method.assert_called_once_with(self.cell)
        mock_move_method.assert_called_once_with(self.cell)
        mock_split_method.assert_not_called()
        self.assertEqual(self.cell.turns_to_live, initial_ttl - 1)
        
        mock_move_method.reset_mock(); mock_adjust_life_method.reset_mock(); mock_draw_action_method.reset_mock(); mock_split_method.reset_mock()
        initial_ttl = self.cell.turns_to_live 
        mock_draw_action_method.return_value = 'split'
        self.cell.next_step()
        mock_adjust_life_method.assert_called_once_with(self.cell)
        mock_split_method.assert_called_once_with(self.cell)
        mock_move_method.assert_not_called()
        self.assertEqual(self.cell.turns_to_live, initial_ttl - 1)

        mock_move_method.reset_mock(); mock_adjust_life_method.reset_mock(); mock_draw_action_method.reset_mock(); mock_split_method.reset_mock()
        initial_ttl = self.cell.turns_to_live 
        mock_draw_action_method.return_value = 'rest'
        self.cell.next_step()
        mock_adjust_life_method.assert_called_once_with(self.cell)
        mock_move_method.assert_not_called()
        mock_split_method.assert_not_called()
        self.assertEqual(self.cell.turns_to_live, initial_ttl - 1)

    @patch.object(Cell, 'adjust_life_expectancy_based_on_neighbors', autospec=True) 
    def test_next_step_death(self, mock_adjust_life_method): 
        self.cell._turns_to_live = 0 
        death_cb = Mock()
        self.cell.death_callback = death_cb
        self.cell.next_step() 
        mock_adjust_life_method.assert_called_once_with(self.cell) 
        self.assertTrue(self.cell.is_dead())
        self.mock_tile.remove_agent.assert_called_once_with() 
        death_cb.assert_called_once_with(self.cell)

    def test_adjust_life_expectancy_based_on_neighbors_cell(self):
        initial_ttl = 100
        self.cell._turns_to_live = initial_ttl
        mock_neighbors_crowded = [Mock(spec=Agent, display_character=str(i)) for i in range(5)] 
        self.mock_tile.get_neighbors_agents.return_value = mock_neighbors_crowded
        self.cell.adjust_life_expectancy_based_on_neighbors()
        self.assertEqual(self.cell.turns_to_live, initial_ttl - 2)

        self.cell._turns_to_live = initial_ttl 
        mock_neighbors_kin = [Mock(spec=Agent, display_character='X') for _ in range(2)]
        self.mock_tile.get_neighbors_agents.return_value = mock_neighbors_kin
        self.cell.adjust_life_expectancy_based_on_neighbors()
        self.assertAlmostEqual(self.cell.turns_to_live, initial_ttl + 0.1)

        self.cell._turns_to_live = initial_ttl 
        mock_neighbors_mixed = [
            Mock(spec=Agent, display_character='X'), Mock(spec=Agent, display_character='X'),
            Mock(spec=Agent, display_character='Y'), Mock(spec=Agent, display_character='Z')
        ] 
        self.mock_tile.get_neighbors_agents.return_value = mock_neighbors_mixed
        self.cell.adjust_life_expectancy_based_on_neighbors()
        self.assertAlmostEqual(self.cell.turns_to_live, initial_ttl - 1 + 0.1)

if __name__ == '__main__':
    unittest.main()
