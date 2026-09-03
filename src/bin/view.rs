use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
    time::Duration,
};

use eframe::egui;
use virtual_life::{Simulation, SimulationConfig, Snapshot};

enum Command {
    SetRunning(bool),
    Step,
    Reset,
    SetSampleEvery(u64),
}

fn main() -> eframe::Result {
    let config = SimulationConfig::default();
    let (command_tx, command_rx) = mpsc::channel();
    let (snapshot_tx, snapshot_rx) = mpsc::sync_channel(1);
    spawn_simulation(config, command_rx, snapshot_tx);

    let mut latest = Simulation::new(config).snapshot();
    let mut running = false;
    let mut sample_every = 1_000_u64;

    eframe::run_ui_native(
        "VirtualLife",
        eframe::NativeOptions::default(),
        move |ui, _frame| {
            while let Ok(snapshot) = snapshot_rx.try_recv() {
                latest = snapshot;
            }

            ui.horizontal(|ui| {
                if ui.button(if running { "Pause" } else { "Run" }).clicked() {
                    running = !running;
                    let _ = command_tx.send(Command::SetRunning(running));
                }
                if ui.button("Step").clicked() {
                    running = false;
                    let _ = command_tx.send(Command::SetRunning(false));
                    let _ = command_tx.send(Command::Step);
                }
                if ui.button("Reset").clicked() {
                    running = false;
                    let _ = command_tx.send(Command::SetRunning(false));
                    let _ = command_tx.send(Command::Reset);
                }
                ui.label(format!("step {}", latest.step));
                ui.label(format!("agents {}", latest.agents.len()));
            });

            let old_sample_every = sample_every;
            ui.add(
                egui::Slider::new(&mut sample_every, 1..=100_000)
                    .logarithmic(true)
                    .text("simulation steps per visual sample"),
            );
            if sample_every != old_sample_every {
                let _ = command_tx.send(Command::SetSampleEvery(sample_every));
            }

            ui.separator();
            draw_world(ui, &latest);
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        },
    )
}

fn spawn_simulation(
    config: SimulationConfig,
    command_rx: Receiver<Command>,
    snapshot_tx: SyncSender<Snapshot>,
) {
    thread::spawn(move || {
        let mut simulation = Simulation::new(config);
        let mut running = false;
        let mut sample_every = 1_000_u64;
        let _ = snapshot_tx.try_send(simulation.snapshot());

        loop {
            while let Ok(command) = command_rx.try_recv() {
                apply_command(
                    command,
                    &mut simulation,
                    &mut running,
                    &mut sample_every,
                    &snapshot_tx,
                );
            }

            if running {
                simulation.step();
                if simulation.step_number().is_multiple_of(sample_every) {
                    // The viewer is observational only: a full channel drops a
                    // visual sample instead of ever blocking the simulation.
                    let _ = snapshot_tx.try_send(simulation.snapshot());
                }
            } else {
                match command_rx.recv_timeout(Duration::from_millis(20)) {
                    Ok(command) => apply_command(
                        command,
                        &mut simulation,
                        &mut running,
                        &mut sample_every,
                        &snapshot_tx,
                    ),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }
    });
}

fn apply_command(
    command: Command,
    simulation: &mut Simulation,
    running: &mut bool,
    sample_every: &mut u64,
    snapshot_tx: &SyncSender<Snapshot>,
) {
    match command {
        Command::SetRunning(value) => *running = value,
        Command::Step => {
            simulation.step();
            let _ = snapshot_tx.try_send(simulation.snapshot());
        }
        Command::Reset => {
            simulation.reset();
            let _ = snapshot_tx.try_send(simulation.snapshot());
        }
        Command::SetSampleEvery(value) => *sample_every = value.max(1),
    }
}

fn draw_world(ui: &mut egui::Ui, snapshot: &Snapshot) {
    let mut occupancy = vec![0_usize; snapshot.width * snapshot.height];
    for agent in &snapshot.agents {
        occupancy[agent.position.y * snapshot.width + agent.position.x] += 1;
    }

    let available = ui.available_size();
    let cell_size = (available.x / snapshot.width as f32)
        .min(available.y / snapshot.height as f32)
        .max(1.0);
    let drawing_size = egui::vec2(
        cell_size * snapshot.width as f32,
        cell_size * snapshot.height as f32,
    );
    let (response, painter) = ui.allocate_painter(drawing_size, egui::Sense::hover());

    let max_occupancy = occupancy.iter().copied().max().unwrap_or(1).max(1) as f32;
    for y in 0..snapshot.height {
        for x in 0..snapshot.width {
            let count = occupancy[y * snapshot.width + x];
            if count == 0 {
                continue;
            }

            let intensity = (80.0 + 175.0 * count as f32 / max_occupancy) as u8;
            let min = response.rect.min + egui::vec2(x as f32, y as f32) * cell_size;
            let rect = egui::Rect::from_min_size(min, egui::vec2(cell_size, cell_size));
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(intensity));

            if count > 1 && cell_size >= 12.0 {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    count,
                    egui::FontId::monospace(cell_size * 0.55),
                    egui::Color32::BLACK,
                );
            }
        }
    }
}
