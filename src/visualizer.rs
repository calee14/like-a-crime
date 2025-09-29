use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct VisualizerData {
    pub current_time: Duration,
    pub amplitude_samples: Vec<f32>,
    pub note_history: VecDeque<(Duration, String)>,
    pub current_note: Option<String>,
    pub total_duration: Duration,
}

pub struct TerminalVisualizer {
    shared_data: Arc<Mutex<VisualizerData>>,
    refresh_rate: Duration,
    waveform_width: usize,
    history_lines: usize,
}

impl TerminalVisualizer {
    pub fn new(
        refresh_rate_ms: u64,
        waveform_width: usize,
        history_lines: usize,
    ) -> (Self, Arc<Mutex<VisualizerData>>) {
        let shared_data = Arc::new(Mutex::new(VisualizerData {
            current_time: Duration::ZERO,
            amplitude_samples: Vec::new(),
            note_history: VecDeque::new(),
            current_note: None,
            total_duration: Duration::ZERO,
        }));

        let visualizer = Self {
            shared_data: shared_data.clone(),
            refresh_rate: Duration::from_millis(refresh_rate_ms),
            waveform_width,
            history_lines,
        };

        (visualizer, shared_data)
    }

    pub fn start_rendering(&self) -> std::thread::JoinHandle<()> {
        let shared_data = self.shared_data.clone();
        let refresh_rate = self.refresh_rate;
        let waveform_width = self.waveform_width;
        let history_lines = self.history_lines;

        thread::spawn(move || {
            print!("\x1B[?25l\x1B[2J");
            io::stdout().flush().unwrap();

            loop {
                thread::sleep(refresh_rate);

                let data = shared_data.lock().unwrap().clone();
                Self::render_frame(&data, waveform_width, history_lines);
            }
        })
    }

    fn render_frame(data: &VisualizerData, waveform_width: usize, history_lines: usize) {
        // Move cursor to top-left
        print!("\x1B[2J\x1B[H");

        // Title bar
        println!(
            "🎵 Time: {:?} / {:?}",
            data.current_time, data.total_duration
        );

        // Current note (large display)
        let current_note = data.current_note.as_deref().unwrap_or("♪ Analyzing...");
        println!("🎼 Current: {}", current_note);
        println!();

        // Waveform visualizer
        println!("Waveform:");
        Self::render_waveform(&data.amplitude_samples, waveform_width);
        println!();

        // Note history
        println!("Note History:");
        Self::render_note_history(&data.note_history, history_lines);

        // Controls hint
        println!();
        print!("Controls: [q]uit | [←][→] seek ±5s | [space] pause\x1B[K");

        io::stdout().flush().unwrap();
    }

    fn render_waveform(samples: &[f32], width: usize) {
        if samples.is_empty() {
            println!("No audio data");
            return;
        }

        let step = samples.len().max(width) / width;
        let max_height = 13; // Much taller! (was 6)

        let mut grid = vec![vec![' '; width]; max_height];

        for i in 0..width {
            let sample_idx = (i * step).min(samples.len() - 1);
            let amplitude = samples[sample_idx].abs();

            // Scale to max possible height with sub-pixel precision
            let total_height = (amplitude * (max_height * 8) as f32) as usize;

            // Fill full blocks
            let full_blocks = total_height / 8;
            for row in 0..full_blocks.min(max_height) {
                let grid_row = max_height - 1 - row;
                grid[grid_row][i] = '█';
            }

            // Fill partial block at top
            let remainder = total_height % 8;
            if full_blocks < max_height && remainder > 0 {
                let grid_row = max_height - 1 - full_blocks;
                grid[grid_row][i] = match remainder {
                    1 => '▁',
                    2 => '▂',
                    3 => '▃',
                    4 => '▄',
                    5 => '▅',
                    6 => '▆',
                    7 => '▇',
                    _ => '█',
                };
            }
        }

        // Print the grid
        for row in &grid {
            println!("{}", row.iter().collect::<String>());
        }
    }
    fn render_note_history(history: &VecDeque<(Duration, String)>, max_lines: usize) {
        let recent_notes: Vec<_> = history.iter().rev().take(max_lines).collect();

        if recent_notes.is_empty() {
            println!("No notes detected yet...");
            return;
        }

        for (timestamp, note) in recent_notes.iter().rev() {
            println!("{:>8.1}s | {}", timestamp.as_secs_f32(), note);
        }
    }

    pub fn cleanup(&self) {
        // Show cursor and clear screen
        print!("\x1B[?25h\x1B[2J\x1B[H");
        io::stdout().flush().unwrap();
    }
}
