import unittest
from unittest.mock import Mock
import sys
import os

# Adjust sys.path to include the src directory
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../src')))

from environment import Tile
from agent import Agent # For type hinting and creating mock agents

class TestTile(unittest.TestCase):
    """Tests for the Tile class."""

    def test_tile_initialization(self):
        tile = Tile(i=3, j=5)
        self.assertEqual(tile.i, 3)
        self.assertEqual(tile.j, 5)
        self.assertIsNone(tile.agent)
        self.assertEqual(tile.neighbors_by_distance, {})
        self.assertEqual(tile.display_character, ' ')

    def test_set_and_get_neighbors(self):
        tile = Tile(0, 0)
        mock_neighbor1_dist1 = Mock(spec=Tile)
        mock_neighbor2_dist1 = Mock(spec=Tile)
        mock_neighbor1_dist2 = Mock(spec=Tile)

        neighbors_dict = {
            1: [mock_neighbor1_dist1, mock_neighbor2_dist1],
            2: [mock_neighbor1_dist2]
        }
        tile.set_neighbors(neighbors_dict)

        self.assertEqual(tile.neighbors_by_distance, neighbors_dict)
        
        # Test get_neighbors_at_distance
        self.assertEqual(tile.get_neighbors_at_distance(1), [mock_neighbor1_dist1, mock_neighbor2_dist1])
        self.assertEqual(tile.get_neighbors_at_distance(2), [mock_neighbor1_dist2])
        self.assertEqual(tile.get_neighbors_at_distance(3), []) # No neighbors at distance 3

        # Test get_neighbors (up to max_distance)
        self.assertCountEqual(tile.get_neighbors(max_distance=1), [mock_neighbor1_dist1, mock_neighbor2_dist1])
        all_neighbors_dist_2 = [mock_neighbor1_dist1, mock_neighbor2_dist1, mock_neighbor1_dist2]
        self.assertCountEqual(tile.get_neighbors(max_distance=2), all_neighbors_dist_2)
        self.assertCountEqual(tile.get_neighbors(max_distance=0), []) # No neighbors at distance 0 or less

    def test_set_agent(self):
        tile = Tile(0, 0)
        mock_agent = Mock(spec=Agent)
        
        tile.set_agent(mock_agent)
        self.assertEqual(tile.agent, mock_agent)

        # Test setting agent to None (should remove agent)
        tile.set_agent(None)
        self.assertIsNone(tile.agent)

        # Test ValueError if tile is already occupied
        tile.set_agent(mock_agent) # Occupy the tile
        another_mock_agent = Mock(spec=Agent)
        with self.assertRaisesRegex(ValueError, "already occupied"):
            tile.set_agent(another_mock_agent)

    def test_remove_agent(self):
        tile = Tile(0, 0)
        mock_agent = Mock(spec=Agent)
        tile.set_agent(mock_agent)
        
        tile.remove_agent()
        self.assertIsNone(tile.agent)
        
        # Test removing agent when already empty (should not fail)
        tile.remove_agent()
        self.assertIsNone(tile.agent)

    def test_is_empty(self):
        tile = Tile(0, 0)
        self.assertTrue(tile.is_empty())
        
        mock_agent = Mock(spec=Agent)
        tile.set_agent(mock_agent)
        self.assertFalse(tile.is_empty())
        
        tile.remove_agent()
        self.assertTrue(tile.is_empty())

    def test_display(self):
        tile = Tile(0, 0)
        # Test display when empty
        self.assertEqual(tile.display(), tile.display_character) # Should be ' '
        
        # Test display with an agent
        mock_agent = Mock(spec=Agent)
        mock_agent.display.return_value = 'A'
        tile.set_agent(mock_agent)
        self.assertEqual(tile.display(), 'A')
        mock_agent.display.assert_called_once()

    def test_get_neighbors_agents_and_empty_neighbors(self):
        tile = Tile(0, 0)
        
        mock_neighbor1 = Mock(spec=Tile)
        mock_agent1 = Mock(spec=Agent)
        mock_neighbor1.agent = mock_agent1
        mock_neighbor1.is_empty.return_value = False
        
        mock_neighbor2 = Mock(spec=Tile)
        mock_neighbor2.agent = None
        mock_neighbor2.is_empty.return_value = True
        
        mock_neighbor3 = Mock(spec=Tile)
        mock_agent3 = Mock(spec=Agent)
        mock_neighbor3.agent = mock_agent3
        mock_neighbor3.is_empty.return_value = False

        # Setup neighbors for the tile
        tile.set_neighbors({1: [mock_neighbor1, mock_neighbor2, mock_neighbor3]})

        # Test get_neighbors_agents
        neighbor_agents = tile.get_neighbors_agents(max_distance=1)
        self.assertCountEqual(neighbor_agents, [mock_agent1, mock_agent3])
        self.assertEqual(len(neighbor_agents), 2)

        # Test get_empty_neighbors
        empty_neighbors = tile.get_empty_neighbors(max_distance=1)
        self.assertCountEqual(empty_neighbors, [mock_neighbor2])
        self.assertEqual(len(empty_neighbors), 1)

        # Test with no neighbors
        tile.set_neighbors({1: []})
        self.assertEqual(tile.get_neighbors_agents(max_distance=1), [])
        self.assertEqual(tile.get_empty_neighbors(max_distance=1), [])

if __name__ == '__main__':
    unittest.main()
