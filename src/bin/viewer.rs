use eframe::egui;
use std::{
    collections::VecDeque,
    sync::mpsc::{self, Receiver},
    time::Duration,
};
use virtual_life::runner::{Command, Config, SNAPSHOT_CAPACITY, Snapshot, Status, Worker};

const HISTORY_LIMIT: usize = 128;

struct Viewer {
    worker: Option<Worker>,
    snapshots: Receiver<Snapshot>,
    latest: Option<Snapshot>,
    history: VecDeque<(u64, usize)>,
    selected_id: Option<u64>,
    config: Config,
    error: Option<String>,
}

impl Viewer {
    fn new(config: Config) -> Result<Self, String> {
        let (sender, snapshots) = mpsc::sync_channel(SNAPSHOT_CAPACITY);
        Ok(Self {
            worker: Some(Worker::spawn(config, Some(sender))?),
            snapshots,
            latest: None,
            history: VecDeque::new(),
            selected_id: None,
            config,
            error: None,
        })
    }

    fn receive(&mut self) {
        // Drain the bounded handoff; the engine has already done these ticks.
        while let Ok(sample) = self.snapshots.try_recv() {
            if self
                .history
                .back()
                .is_none_or(|&(tick, _)| tick != sample.tick)
            {
                if self.history.len() == HISTORY_LIMIT {
                    self.history.pop_front();
                }
                self.history.push_back((sample.tick, sample.count));
            }
            self.latest = Some(sample);
        }
        if self.worker.as_ref().is_some_and(Worker::is_finished)
            && let Err(error) = self.worker.take().unwrap().join()
        {
            self.error = Some(error);
        }
    }

    fn send(&self, command: Command) {
        if let Some(worker) = &self.worker {
            // A click may race natural completion. receive() joins the worker
            // and reports actual failures, rather than treating that race as one.
            let _ = worker.command(command);
        }
    }

    fn draw_grid(ui: &mut egui::Ui, sample: &Snapshot, selected: &mut Option<u64>) {
        let colors = [
            egui::Color32::from_rgb(37, 83, 115),
            egui::Color32::from_rgb(91, 59, 120),
            egui::Color32::from_rgb(45, 100, 93),
            egui::Color32::from_rgb(115, 77, 42),
            egui::Color32::from_rgb(108, 49, 74),
        ];
        egui::Grid::new("world")
            .spacing(egui::vec2(4.0, 4.0))
            .show(ui, |ui| {
                ui.label("y / x");
                for x in 0..sample.width {
                    ui.label(x.to_string());
                }
                ui.end_row();
                for y in 0..sample.height {
                    ui.label(y.to_string());
                    for x in 0..sample.width {
                        let agent = sample.cells[y * sample.width + x];
                        let (label, color) = match agent {
                            Some(agent) => (
                                format!("ID {}\nvalue {}", agent.id, agent.value),
                                colors[(agent.id % colors.len() as u64) as usize],
                            ),
                            None => ("".into(), egui::Color32::from_gray(28)),
                        };
                        let is_selected = agent.is_some_and(|a| Some(a.id) == *selected);
                        let button = egui::Button::new(
                            egui::RichText::new(label).color(egui::Color32::WHITE),
                        )
                        .min_size(egui::vec2(80.0, 56.0))
                        .fill(color)
                        .stroke(egui::Stroke::new(
                            if is_selected { 2.0 } else { 1.0 },
                            if is_selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_gray(60)
                            },
                        ));
                        if ui
                            .add(button)
                            .on_hover_text(format!("({x},{y}) at tick {}", sample.tick))
                            .clicked()
                        {
                            *selected = agent.map(|a| a.id);
                        }
                    }
                    ui.end_row();
                }
            });
    }

    fn draw_plot(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Agent count / tick — last {} samples (limit {HISTORY_LIMIT})",
            self.history.len()
        ));
        let (response, painter) = ui.allocate_painter(
            egui::vec2(ui.available_width().min(760.0), 155.0),
            egui::Sense::hover(),
        );
        let rect = response.rect.shrink2(egui::vec2(30.0, 22.0));
        let first = self.history.front().map_or(0, |p| p.0);
        let last = self.history.back().map_or(1, |p| p.0).max(first + 1);
        let maximum = self.history.iter().map(|p| p.1).max().unwrap_or(1).max(1);
        let point = |(tick, count): (u64, usize)| {
            egui::pos2(
                rect.left() + ((tick - first) as f64 / (last - first) as f64) as f32 * rect.width(),
                rect.bottom() - count as f32 / maximum as f32 * rect.height(),
            )
        };
        let axis = egui::Stroke::new(1.0, egui::Color32::GRAY);
        let color = egui::Color32::from_rgb(112, 195, 230);
        painter.line_segment([rect.left_top(), rect.left_bottom()], axis);
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], axis);
        let font = egui::FontId::proportional(12.0);
        painter.text(
            rect.left_top() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_TOP,
            maximum.to_string(),
            font.clone(),
            egui::Color32::GRAY,
        );
        painter.text(
            rect.left_bottom() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_BOTTOM,
            "0",
            font.clone(),
            egui::Color32::GRAY,
        );
        let mut previous: Option<(u64, usize)> = None;
        for &sample in &self.history {
            let position = point(sample);
            if let Some(old) = previous {
                if sample.0 == old.0 + 1 {
                    painter.line_segment([point(old), position], egui::Stroke::new(2.0, color));
                } else {
                    // Never draw a continuous history through unobserved ticks.
                    let middle = point(old).lerp(position, 0.5);
                    painter.text(
                        middle,
                        egui::Align2::CENTER_CENTER,
                        "gap",
                        font.clone(),
                        egui::Color32::ORANGE,
                    );
                }
            }
            painter.circle_filled(position, 4.0, color);
            if self.history.len() <= 12 {
                painter.text(
                    egui::pos2(position.x, rect.bottom() + 5.0),
                    egui::Align2::CENTER_TOP,
                    sample.0.to_string(),
                    font.clone(),
                    egui::Color32::LIGHT_GRAY,
                );
            }
            previous = Some(sample);
        }
    }
}

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receive();
        // UI polling affects display times, never engine ticks.
        if self
            .latest
            .as_ref()
            .is_none_or(|s| s.status != Status::Completed)
            && self.error.is_none()
        {
            ui.ctx().request_repaint_after(Duration::from_millis(33));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("VirtualLife · live grid");
                ui.label("Five scripted transitions — a machinery test, not emergence.");
                let status = self.latest.as_ref().map(|s| s.status);
                ui.horizontal(|ui| {
                    let active = self.worker.is_some();
                    let running = active && status == Some(Status::Running);
                    let paused = active && status == Some(Status::Paused);
                    if ui.add_enabled(running, egui::Button::new("Pause")).clicked() {
                        self.send(Command::Pause);
                    }
                    if ui.add_enabled(paused, egui::Button::new("Resume")).clicked() {
                        self.send(Command::Resume);
                    }
                    if ui.add_enabled(paused, egui::Button::new("Single step")).clicked() {
                        self.send(Command::Step);
                    }
                    ui.label(match status {
                        Some(Status::Running) => "Running",
                        Some(Status::Paused) => "Paused",
                        Some(Status::Completed) => "Completed · tick 5 of 5",
                        None => "Waiting for worker…",
                    });
                });
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
                ui.separator();
                if let Some(sample) = &self.latest {
                    ui.strong(format!("Tick {}  ·  {} agents", sample.tick, sample.count));
                    ui.label(format!(
                        "Accepted totals: {} moves · {} creations · {} removals · {} value changes",
                        sample.totals.moves, sample.totals.creations,
                        sample.totals.removals, sample.totals.value_changes
                    ));
                    ui.add_space(6.0);
                    Self::draw_grid(ui, sample, &mut self.selected_id);
                    ui.add_space(6.0);
                    if let Some(id) = self.selected_id {
                        let selected = sample.cells.iter().enumerate().find_map(|(i, cell)| {
                            cell.filter(|a| a.id == id).map(|a| (i, a))
                        });
                        match selected {
                            Some((i, agent)) => {
                                ui.label(format!(
                                    "Inspection: ID {id}, value {}, at ({},{}) in tick {}",
                                    agent.value, i % sample.width, i / sample.width, sample.tick
                                ));
                            }
                            None => { ui.label(format!("ID {id} is absent at tick {}.", sample.tick)); }
                        }
                    } else {
                        ui.label("Click an agent to inspect its ID and value (read only).");
                    }
                    ui.separator();
                    ui.label(format!(
                        "Sample every {} ticks, plus controls and completion. Tick interval: {} ms.",
                        self.config.sample_every, self.config.tick_interval.as_millis()
                    ));
                    ui.label("Gaps are unobserved ticks. Accepted totals come from the engine.");
                    self.draw_plot(ui);
                    if sample.status == Status::Completed {
                        ui.label("The finite demo has ended. Close and relaunch to repeat it.");
                    }
                }
            });
        });
    }
}

// Dropping Viewer drops Worker, which sends Stop and joins the thread.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::default();
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--running" => config.start_paused = false,
            "--sample-every" => {
                config.sample_every = arguments.next().ok_or("missing sample cadence")?.parse()?
            }
            "--tick-ms" => {
                config.tick_interval =
                    Duration::from_millis(arguments.next().ok_or("missing tick interval")?.parse()?)
            }
            "--help" | "-h" => {
                println!(
                    "Usage: viewer [--running] [--sample-every N] [--tick-ms N]\nDefaults: paused, sample every tick, 750 ms minimum tick interval. Ends at tick 5."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {argument}").into()),
        }
    }
    let app = Viewer::new(config)?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([820.0, 760.0])
            .with_min_inner_size([720.0, 720.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "VirtualLife scripted grid",
        options,
        Box::new(|_| Ok(Box::new(app))),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Instant};

    #[test]
    fn a_control_click_racing_natural_completion_is_not_a_viewer_error() {
        let mut viewer = Viewer::new(Config {
            ticks: 0,
            ..Config::default()
        })
        .unwrap();
        let worker = viewer.worker.as_ref().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        worker.wait_for_completion(Duration::from_secs(10)).unwrap();
        while !worker.is_finished() && Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(worker.is_finished());
        // The worker ended, but the UI has not received its final sample yet.
        viewer.send(Command::Pause);
        viewer.receive();
        assert!(viewer.error.is_none());
        assert_eq!(viewer.latest.unwrap().status, Status::Completed);
    }
}
