import unittest
from unittest.mock import Mock, patch, call
import sys
import os
import random

# Adjust sys.path to include the src directory
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../src')))

from model import MyModel
from environment import Tile
from agent import Agent, Cell # Cell is imported here for spec in mocks
from config import SIMULATION_CONFIG

class TestMyModel(unittest.TestCase):
    """Tests for the MyModel class."""

    def setUp(self):
        self.env_height = 5
        self.env_width = 5
        self.populate_proba = 0.0 

    def test_model_initialization(self):
        with patch.object(MyModel, '_assign_neighbors_to_tiles') as mock_assign_neighbors, \
             patch.object(MyModel, '_populate_env') as mock_populate_env:
            
            model = MyModel(self.env_height, self.env_width, self.populate_proba)

            self.assertEqual(model.env_height, self.env_height)
            self.assertEqual(model.env_width, self.env_width)
            self.assertEqual(len(model.env_matrix), self.env_height)
            self.assertEqual(len(model.env_matrix[0]), self.env_width)
            self.assertIsInstance(model.env_matrix[0][0], Tile)
            
            mock_assign_neighbors.assert_called_once()
            mock_populate_env.assert_called_once_with(self.populate_proba)
            self.assertEqual(model.active_agents, [])

    def test_get_neighbors_internal_method(self):
        model = MyModel(3, 3, 0.0) 
        neighbors_dist1 = model._get_neighbors(r=1, c=1, distance=1)
        expected_coords_dist1 = [(0,1), (1,0), (1,2), (2,1)]
        self.assertEqual(len(neighbors_dist1), 4)
        for tile in neighbors_dist1: self.assertIn((tile.i, tile.j), expected_coords_dist1)

        neighbors_corner_dist1 = model._get_neighbors(r=0, c=0, distance=1)
        expected_coords_corner_dist1 = [(0,1), (1,0), (2,0), (0,2)]
        self.assertEqual(len(neighbors_corner_dist1), 4)
        for tile in neighbors_corner_dist1: self.assertIn((tile.i, tile.j), expected_coords_corner_dist1)
            
        neighbors_dist2 = model._get_neighbors(r=1, c=1, distance=2)
        self.assertEqual(len(neighbors_dist2), 8) 
        self.assertEqual(model._get_neighbors(1, 1, 0), [])
        
    def test_assign_neighbors_to_tiles_integration(self):
        model = MyModel(3, 3, 0.0) 
        center_tile = model.env_matrix[1][1]
        self.assertEqual(len(center_tile.get_neighbors_at_distance(1)), 4)
        self.assertEqual(len(center_tile.get_neighbors_at_distance(2)), 8)
        corner_tile = model.env_matrix[0][0]
        self.assertEqual(len(corner_tile.get_neighbors_at_distance(1)), 4)

    @patch('model.random.random') 
    @patch('model.Cell', autospec=True) # Patch Cell in model's namespace
    def test_populate_env(self, MockCell_in_model_scope, mock_random_in_model_scope):
        # Scenario 1: populate_proba = 0.0 (tested via MyModel init)
        mock_random_in_model_scope.return_value = 0.9 
        model = MyModel(self.env_height, self.env_width, populate_proba=0.0) 
        self.assertEqual(len(model.active_agents), 0)
        MockCell_in_model_scope.assert_not_called()

        # Scenario 2: Test with populate_proba = 1.0 by calling _populate_env directly
        mock_random_in_model_scope.return_value = 0.0 
        
        mock_cell_instances = []
        def mock_cell_constructor_side_effect(tile, death_callback, born_callback):
            # This side_effect is for the constructor call on the mocked class.
            # It should return the mock *instance*.
            mock_instance = Mock(spec=Cell) # Create a generic Mock for the instance
            mock_instance.current_tile = tile 
            # Manually assign the callbacks to the mock instance, as the real Cell.__init__ would.
            mock_instance.death_callback = death_callback
            mock_instance.born_callback = born_callback
            mock_cell_instances.append(mock_instance)
            return mock_instance
        
        MockCell_in_model_scope.reset_mock() 
        MockCell_in_model_scope.side_effect = mock_cell_constructor_side_effect

        model.active_agents.clear()
        for r_idx_clear in range(model.env_height):
            for c_idx_clear in range(model.env_width):
                if model.env_matrix[r_idx_clear][c_idx_clear].agent is not None:
                    model.env_matrix[r_idx_clear][c_idx_clear].remove_agent()

        model._populate_env(1.0) 
        
        total_tiles = self.env_height * self.env_width
        self.assertEqual(len(model.active_agents), total_tiles)
        self.assertEqual(MockCell_in_model_scope.call_count, total_tiles)
        
        self.assertEqual(len(mock_cell_instances), total_tiles)
        for r in range(self.env_height):
            for c in range(self.env_width):
                tile = model.env_matrix[r][c]
                self.assertIsNotNone(tile.agent)
                self.assertIn(tile.agent, mock_cell_instances)
                self.assertIn(tile.agent, model.active_agents)
                # Check that the mock instance on the tile had its callbacks set.
                self.assertEqual(tile.agent.death_callback, model.agent_died)
                self.assertEqual(tile.agent.born_callback, model.agent_born)

    def test_agent_died_and_born(self):
        model = MyModel(self.env_height, self.env_width, 0.0)
        mock_agent1 = Mock(spec=Agent)
        mock_agent2 = Mock(spec=Agent)

        model.agent_born(mock_agent1)
        self.assertIn(mock_agent1, model.active_agents)
        model.agent_born(mock_agent2)
        self.assertIn(mock_agent2, model.active_agents)
        self.assertEqual(len(model.active_agents), 2)
        model.agent_born(mock_agent1) # Duplicate
        self.assertEqual(len(model.active_agents), 2)

        model.agent_died(mock_agent1)
        self.assertNotIn(mock_agent1, model.active_agents)
        self.assertIn(mock_agent2, model.active_agents)
        self.assertEqual(len(model.active_agents), 1)
        model.agent_died(mock_agent1) # Not in list
        self.assertEqual(len(model.active_agents), 1)

    def test_get_tile_at_position(self):
        model = MyModel(self.env_height, self.env_width, 0.0)
        tile = model.get_tile_at_position(2, 3)
        self.assertIsInstance(tile, Tile)
        self.assertEqual(tile.i, 2)
        self.assertEqual(tile.j, 3)
        with self.assertRaisesRegex(ValueError, "out of environment bounds"):
            model.get_tile_at_position(-1, 0)
        with self.assertRaisesRegex(ValueError, "out of environment bounds"):
            model.get_tile_at_position(0, self.env_width)

    def test_run_simulation_step(self):
        model = MyModel(self.env_height, self.env_width, 0.0)
        mock_agent1 = Mock(spec=Agent); mock_agent1.is_dead.return_value = False
        mock_agent2 = Mock(spec=Agent); mock_agent2.is_dead.return_value = False
        model.active_agents = [mock_agent1, mock_agent2]
        
        with patch.object(model, 'notify_observers') as mock_notify:
            model.run_simulation_step()
            mock_notify.assert_called_once() # Check if observers are notified

        mock_agent1.next_step.assert_called_once()
        mock_agent2.next_step.assert_called_once()

        # Test robustness if agent dies and is removed during step
        mock_agent1.reset_mock()
        mock_agent2.reset_mock()
        # Reset notify_observers mock for the next part of the test if model instance is reused,
        # or ensure a fresh model for different scenarios.
        # For this test, model is the same, so mock_notify needs to be re-patched or managed if called again.
        # Let's refine this to be specific for each call to run_simulation_step if necessary.
        # The above with statement handles mock_notify for one call.

        # To test notify_observers in the context of agent death/removal:
        def agent1_dies_effect():
            # Simulate that the agent's next_step might lead to its removal from active_agents
            # This could be due to a death_callback being invoked.
            if mock_agent1 in model.active_agents: # Check before removing
                 model.active_agents.remove(mock_agent1) 
            mock_agent1.is_dead.return_value = True # Agent is now dead
        
        mock_agent1.next_step.side_effect = agent1_dies_effect
        model.active_agents = [mock_agent1, mock_agent2] # Agent1 will "die"

        with patch.object(model, 'notify_observers') as mock_notify_death_scenario:
            model.run_simulation_step()
            mock_notify_death_scenario.assert_called_once() # Check notification even with agent death

        mock_agent1.next_step.assert_called_once() # Agent1's next_step was called
        mock_agent2.next_step.assert_called_once() # Agent2's next_step should still be called


if __name__ == '__main__':
    unittest.main()
