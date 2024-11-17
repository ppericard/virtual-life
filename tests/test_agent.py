import pytest
from src.agent import Agent, Cell
from src.environment import Tile

def test_cell_creation():
    tile = Tile(0, 0)
    cell = Cell(tile)
    assert not cell.is_dead()
    assert cell.current_tile == tile
    assert cell.previous_tile is None

def test_cell_movement():
    # Test cell movement logic
    pass 