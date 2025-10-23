use eframe::{App, Frame, egui};
use egui::frame;
use fundsp::{math::midi_hz, shared::Shared};
use std::sync::{Arc, Mutex};

pub struct SynthApp {
    gate: Shared,
    frequency: Shared,
    current_note: Option<char>,

    should_quit: Arc<Mutex<bool>>,
}

impl SynthApp {
    pub fn new(gate: Shared, frequency: Shared, should_quit: Arc<Mutex<bool>>) -> Self {
        SynthApp {
            gate,
            frequency,
            current_note: None,
            should_quit,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InputEvent {
    KeyDown(char),
    KeyUp(char),
    Quit,
}

impl App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        ctx.request_repaint();

        let mut note_event = None;

        for event in &ctx.input(|i| i.events.clone()) {
            if let egui::Event::Key {
                key,
                pressed,
                repeat,
                ..
            } = event
            {
                let key_char = match key {
                    egui::Key::A => Some('a'),
                    egui::Key::S => Some('s'),
                    egui::Key::D => Some('d'),
                    egui::Key::F => Some('f'),
                    _ => None,
                };

                if let Some(note) = key_char {
                    if *pressed || *repeat {
                        note_event = Some(InputEvent::KeyDown(note));
                    } else {
                        note_event = Some(InputEvent::KeyUp(note));
                    }
                }

                if *key == egui::Key::Escape && *pressed {
                    note_event = Some(InputEvent::Quit);
                }
            }
        }

        if let Some(event) = note_event {
            match event {
                InputEvent::Quit => {
                    *self.should_quit.lock().unwrap() = true;
                    // close eframe app
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                InputEvent::KeyDown(key_char) => {
                    if self.current_note != Some(key_char) {
                        self.current_note = Some(key_char);
                        let midi_note = match key_char {
                            'a' => 60.0,
                            's' => 62.0,
                            'd' => 64.0,
                            'f' => 65.0,
                            _ => 60.0,
                        };
                        self.frequency.set_value(midi_hz(midi_note));
                        self.gate.set_value(1.0);
                    }
                }
                InputEvent::KeyUp(key_char) => {
                    if self.current_note == Some(key_char) {
                        self.current_note = None;
                        self.gate.set_value(0.0);
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("synth");
            ui.label(format!(
                "note: {}",
                self.current_note
                    .map_or("none".to_string(), |c| c.to_string())
            ));
            ui.label("press a, s, d, f to play. press Esc to quit");
        });
    }
}
