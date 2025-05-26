"""
This module defines the `MyController` class, which orchestrates the simulation.

The controller initializes the model and view, manages the main simulation loop,
handles user input (like KeyboardInterrupt for exiting), and ensures the
simulation runs at the desired frame rate.
"""
from observer import * # Assuming Observer/Observable might be used, though not directly in this snippet
from model import MyModel
from view import MyView
# import os # os is imported but not used
# import platform # platform is imported but not used
import time

class MyController():
    """
    Manages the simulation flow, connecting the model (data) and view (display).

    The controller is responsible for:
    - Initializing the simulation environment (model) and the display (view).
    - Running the main simulation loop.
    - Updating the view at each step.
    - Handling basic user interactions like exiting the simulation.
    - Attempting to maintain a target frame rate.
    """

    def __init__(self, env_height: int, env_width: int,
                 populate_proba: float, frame_per_second: int):
        """
        Initializes the controller, model, and view.

        Args:
            env_height: The height of the simulation environment grid.
            env_width: The width of the simulation environment grid.
            populate_proba: The initial probability of a tile being populated by an agent.
            frame_per_second: The target frame rate for the simulation display.
        """
        self.model: MyModel = MyModel(env_height, env_width, populate_proba)
        self.view: MyView = MyView(self.model, frame_per_second)
        self.model.register_observer(self.view) # Register view as an observer of model

    def clear_screen(self):
        """Clears the terminal screen using ANSI escape codes."""
        # ANSI escape sequence to clear the screen and move cursor to home position
        print("\033[H\033[J", end="")

    def update_view(self):
        """Clears the screen and then updates the view to reflect the current model state."""
        self.clear_screen()
        self.view.update()

    def run(self):
        """
        Runs the main simulation loop.

        This loop continues until a KeyboardInterrupt (e.g., Ctrl+C) is received
        or a fatal error occurs. Inside the loop, it:
        1. Executes a simulation step in the model.
        2. Updates the view.
        3. Calculates the time taken for the step and rendering.
        4. Attempts to sleep for an appropriate duration to match the target FPS,
           compensating for processing time and sleep inaccuracies.
        """
        try:
            self.update_view() # Initial view update before the loop starts
            last_frame_time: float = time.perf_counter() # Time at the end of the last frame
            fps_smoothing_factor: float = 0.9 # Smoothing factor for FPS calculation (Exponential Moving Average)
            smoothed_fps: float = float(self.view.frame_per_second) # Initialize with target FPS
            target_frame_duration: float = 1.0 / self.view.frame_per_second
            # Accumulates discrepancies between desired sleep time and actual time passed,
            # to compensate for imprecise sleep calls or long simulation steps.
            total_sleep_deficit: float = 0.0


            while True:
                try:
                    sim_time_start: float = time.perf_counter()
                    self.model.run_simulation_step() # Run one step of the simulation logic
                    # The model will now notify the view, so self.update_view() is no longer needed here.
                    sim_time_end: float = time.perf_counter()
                    simulation_duration: float = sim_time_end - sim_time_start

                    # self.update_view() # REMOVED: View is updated by observer pattern

                    current_frame_time: float = time.perf_counter()
                    # Duration of the entire frame (simulation + rendering + sleep management)
                    actual_frame_duration: float = current_frame_time - last_frame_time
                    last_frame_time = current_frame_time

                    # Calculate current FPS and apply smoothing
                    current_fps: float = 1.0 / actual_frame_duration if actual_frame_duration > 0 else 0
                    smoothed_fps = (smoothed_fps * fps_smoothing_factor) + (current_fps * (1 - fps_smoothing_factor))

                    # Display performance metrics
                    print(f"Simulation duration={simulation_duration:.6f} sec")
                    print(f"Actual FPS={smoothed_fps:.2f}, Target FPS={self.view.frame_per_second}")
                    print(f"Frame processing time (sim+render)={actual_frame_duration:.6f} sec")
                    
                    # Calculate sleep time needed to meet target FPS, factoring in previous deficit/surplus
                    # We aim for the (simulation_step_time + sleep_time) to equal target_frame_duration
                    time_to_sleep: float = target_frame_duration - (time.perf_counter() - sim_time_start) + total_sleep_deficit
                    
                    if time_to_sleep > 0:
                        # For very short sleeps, time.sleep() can be inaccurate.
                        # A common strategy is to sleep for slightly less and then busy-wait.
                        # However, busy-waiting consumes CPU. Here, we primarily rely on time.sleep().
                        # A more advanced approach might use a hybrid sleep/busy-wait or OS-specific timers.
                        time.sleep(time_to_sleep)
                    
                    # Update the sleep deficit based on how accurately we hit the target_frame_duration
                    # This helps compensate for oversleeping or undersleeping in previous frames.
                    time_passed_this_frame = time.perf_counter() - sim_time_start
                    total_sleep_deficit += (target_frame_duration - time_passed_this_frame)
                    # Clamp the deficit to prevent it from growing too large or small,
                    # which could lead to instability in frame timing.
                    total_sleep_deficit = max(min(total_sleep_deficit, target_frame_duration / 2), -target_frame_duration / 2)


                except Exception as e:
                    print(f"Error during simulation step: {e}")
                    # Depending on the error, one might want to log it and continue,
                    # or break the loop for critical errors. For now, it continues.
                    continue # Skip to the next iteration of the main loop

        except KeyboardInterrupt:
            print("\nSimulation interrupted by user.")
        except Exception as e:
            # Catch any other unexpected exceptions that weren't handled in the inner loop
            print(f"Fatal error: {e}")
        finally:
            # This block executes whether the loop terminated normally,
            # due to an exception, or KeyboardInterrupt.
            print("Exiting simulation. Performing cleanup...")
            self.cleanup()

    def cleanup(self):
        """
        Performs any necessary cleanup when the simulation ends.
        
        This method can be extended to include actions like saving simulation state,
        closing files, or releasing resources. Currently, it's a placeholder.
        """
        # Example: self.model.save_state("final_simulation_state.json")
        # Example: self.view.close_display_window()
        print("Cleanup complete.")
        pass
