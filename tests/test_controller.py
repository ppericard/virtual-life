import unittest
from unittest.mock import Mock, patch, call
import sys
import os
import time

# Adjust sys.path to include the src directory
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../src')))

from controller import MyController
from model import MyModel
from view import MyView
from config import SIMULATION_CONFIG # For default FPS if needed

class TestMyController(unittest.TestCase):
    """Tests for the MyController class."""

    @patch('controller.MyView')
    @patch('controller.MyModel')
    def setUp(self, MockMyModel, MockMyView):
        # Use config values for controller defaults
        self.env_height = SIMULATION_CONFIG['environment']['height']
        self.env_width = SIMULATION_CONFIG['environment']['width']
        self.populate_proba = SIMULATION_CONFIG['environment']['populate_probability']
        self.fps = SIMULATION_CONFIG['display']['frame_per_second']

        self.mock_model_instance = MockMyModel.return_value
        self.mock_view_instance = MockMyView.return_value
        self.mock_view_instance.frame_per_second = self.fps # Ensure view has FPS

        self.controller = MyController(
            env_height=self.env_height,
            env_width=self.env_width,
            populate_proba=self.populate_proba,
            frame_per_second=self.fps
        )
        
        # Check that Model and View were instantiated correctly by __init__
        MockMyModel.assert_called_once_with(self.env_height, self.env_width, self.populate_proba)
        MockMyView.assert_called_once_with(self.mock_model_instance, self.fps)


    def test_controller_initialization(self):
        # Setup in setUp already calls __init__ and asserts instantiation
        self.assertIsNotNone(self.controller.model)
        self.assertIsNotNone(self.controller.view)
        self.assertEqual(self.controller.model, self.mock_model_instance)
        self.assertEqual(self.controller.view, self.mock_view_instance)

    @patch('builtins.print') # To suppress print output during test
    def test_clear_screen(self, mock_print):
        self.controller.clear_screen()
        mock_print.assert_called_once_with("\033[H\033[J", end="")

    @patch.object(MyController, 'clear_screen')
    def test_update_view(self, mock_clear_screen):
        self.controller.update_view()
        mock_clear_screen.assert_called_once()
        self.mock_view_instance.update.assert_called_once()
        
    @patch('time.perf_counter')
    @patch('time.sleep')
    @patch.object(MyController, 'update_view') # Mock update_view to control its calls
    def test_run_simulation_loop_basic_flow_and_exit(self, mock_update_view, mock_sleep, mock_perf_counter):
        # Simulate a few loop iterations and then a KeyboardInterrupt

        # Setup mock return values for time.perf_counter
        # Sequence: initial_view_time, loop1_sim_start, loop1_sim_end, loop1_render_end, 
        #           loop2_sim_start, loop2_sim_end, loop2_render_end, ...
        # Frame duration is 1/fps. If fps=20, duration = 0.05s
        target_frame_duration = 1.0 / self.fps
        
        # Simulate time progression
        time_sequence = [
            0.0, # Initial call to update_view before loop
            0.1, # loop 1 - sim_time_start
            0.11, # loop 1 - sim_time_end (simulation_duration = 0.01)
            # update_view is called here
            0.12, # loop 1 - current_frame_time (actual_frame_duration from last frame = 0.12 - 0.0 = 0.12)
                  # time_to_sleep = target_frame_duration - (0.12 - 0.1) + total_sleep_deficit
                  # time_to_sleep = 0.05 - 0.02 + 0 = 0.03
                  # time_passed_this_frame (after sleep) = (0.12 - 0.1) + 0.03 = 0.05 (ideal)
                  # total_sleep_deficit = 0.05 - 0.05 = 0 (ideal)
            
            0.12 + 0.03, # loop 2 - sim_time_start (0.15)
            0.15 + 0.01, # loop 2 - sim_time_end (0.16, simulation_duration = 0.01)
            # update_view is called here
            0.16 + 0.01, # loop 2 - current_frame_time (0.17, actual_frame_duration = 0.17 - 0.12 = 0.05)
                         # time_to_sleep = 0.05 - (0.17-0.15) + 0 = 0.03
        ]
        mock_perf_counter.side_effect = time_sequence

        # Make the model's run_simulation_step raise KeyboardInterrupt after a few calls
        call_count = 0
        def side_effect_run_sim_step():
            nonlocal call_count
            call_count += 1
            if call_count > 1: # Let it run once, then raise interrupt
                raise KeyboardInterrupt
        self.mock_model_instance.run_simulation_step.side_effect = side_effect_run_sim_step

        with patch('builtins.print') as mock_print: # Suppress prints
            self.controller.run()

        # Check calls
        # update_view is now only called once for the initial display.
        mock_update_view.assert_called_once() 
        self.assertEqual(self.mock_model_instance.run_simulation_step.call_count, 2) # Runs twice before interrupt
        
        # Check that sleep was called. The exact value depends on complex calculation,
        # so we check it was called with a positive value.
        mock_sleep.assert_called_once()
        self.assertGreater(mock_sleep.call_args[0][0], 0)
        
        # Check cleanup was called
        self.controller.view.model.get_tile_at_position # Example: Check if cleanup was called (needs actual cleanup logic)
                                                        # For now, just check print statement from finally block
        self.assertIn(call("Exiting simulation. Performing cleanup..."), mock_print.call_args_list)
        self.assertIn(call("Cleanup complete."), mock_print.call_args_list)


    @patch('time.perf_counter')
    @patch('time.sleep')
    @patch.object(MyController, 'update_view')
    def test_run_simulation_handles_general_exception_in_loop(self, mock_update_view_ctrl, mock_sleep, mock_perf_counter): # Renamed mock_update_view
        # Simulate an exception during model.run_simulation_step
        mock_perf_counter.side_effect = [0.0, 0.1, 0.11, 0.12, 0.15, 0.16, 0.17] # Some time values

        run_step_call_count = 0
        def run_simulation_step_side_effect():
            nonlocal run_step_call_count
            run_step_call_count +=1
            if run_step_call_count == 1:
                raise Exception("Test error in simulation step")
            raise KeyboardInterrupt # To exit after testing the exception handling

        self.mock_model_instance.run_simulation_step.side_effect = run_simulation_step_side_effect

        with patch('builtins.print') as mock_print:
            self.controller.run()
        
        # Should print the error message and continue to KeyboardInterrupt
        error_printed = False
        for print_call in mock_print.call_args_list:
            if "Error during simulation step: Test error in simulation step" in str(print_call):
                error_printed = True
                break
        self.assertTrue(error_printed, "Error message was not printed.")
        
        self.assertEqual(self.mock_model_instance.run_simulation_step.call_count, 2) # First fails, second raises KI
        # update_view (from controller) is only called once at the start.
        # If the first model.run_simulation_step() fails, no further view updates are triggered by the controller.
        # The view might attempt an update via observer if notify_observers was called before exception,
        # but here we are testing controller's direct calls.
        mock_update_view_ctrl.assert_called_once()

    @patch('builtins.print')
    def test_cleanup_method(self, mock_print):
        # The cleanup method is very simple, just ensure it can be called.
        # If it had more logic, more detailed tests would be needed.
        self.controller.cleanup()
        mock_print.assert_called_with("Cleanup complete.")

if __name__ == '__main__':
    unittest.main()
