from observer import *
from model import MyModel
from view import MyView
import os
import platform
import time

class MyController():

    def __init__(self, env_height, env_width,
                 populate_proba, frame_per_second):
        self.model = MyModel(env_height, env_width, populate_proba)
        self.view = MyView(self.model, frame_per_second)

    def clear_screen(self):
        # ANSI escape sequence to clear the screen
        print("\033[H\033[J", end="")

    def update_view(self):
        self.clear_screen()
        self.view.update()

    def run(self):
        try:
            self.update_view()
            last_frame_time = time.perf_counter()
            fps_smoothing = 0.9
            smoothed_fps = self.view.frame_per_second
            target_frame_duration = 1.0 / self.view.frame_per_second
            total_sleep_deficit = 0.0

            while True:
                try:
                    sim_time_start = time.perf_counter()
                    self.model.run_simulation_step()
                    sim_time_end = time.perf_counter()
                    simulation_duration = sim_time_end - sim_time_start

                    self.update_view()

                    current_frame_time = time.perf_counter()
                    total_frame_duration = current_frame_time - last_frame_time
                    last_frame_time = current_frame_time

                    # Calculate FPS and apply smoothing
                    current_fps = 1.0 / total_frame_duration if total_frame_duration > 0 else 0
                    smoothed_fps = (smoothed_fps * fps_smoothing) + (current_fps * (1 - fps_smoothing))

                    print(f"Simulation duration={simulation_duration:.6f} sec")
                    print(f"Actual FPS={smoothed_fps:.2f}, Target FPS={self.view.frame_per_second}")
                    print(f"total_frame_duration_in_sec={total_frame_duration:.6f} sec")
                    
                    # Calculate sleep time, factoring in any previous deficit
                    sleep_duration = max(target_frame_duration - simulation_duration + total_sleep_deficit, 0)
                    if sleep_duration > 0:
                        # Sleep for most of the duration, then busy-wait for more accuracy
                        time.sleep(sleep_duration if sleep_duration < 0.002 else sleep_duration - 0.002)
                        while time.perf_counter() - sim_time_start < target_frame_duration:
                            pass
                    # Update sleep deficit
                    total_sleep_deficit += target_frame_duration - (time.perf_counter() - sim_time_start)
                    total_sleep_deficit = max(min(total_sleep_deficit, 0.1), -0.1)  # Keep within reasonable bounds

                except Exception as e:
                    print(f"Error during simulation step: {e}")
                    # Log the error and continue or break based on severity
                    continue

        except KeyboardInterrupt:
            print("\nSimulation interrupted by user.")
        except Exception as e:
            print(f"Fatal error: {e}")
        finally:
            # Cleanup code here
            self.cleanup()

    def cleanup(self):
        """Perform any necessary cleanup when simulation ends"""
        pass
