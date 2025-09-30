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
            // push existing content up by printing 30 blank lines
            for _ in 0..30 {
                println!();
            }
            // initialize fixed lines once
            print!("\x1B[?25l"); // hide cursor
            print!("\x1B[31;0H"); // line 29
            print!("Controls: [q]uit | [j][l] seek ±5s | [k] start/stop");
            print!("\x1B[32;0H"); // line 30
            print!("Command (then press Enter): ");

            print!("\x1B[32;32H");
            print!("\x1B[?25h"); // show cursor
            io::stdout().flush().unwrap();

            loop {
                thread::sleep(refresh_rate);

                let data = shared_data.lock().unwrap().clone();
                Self::render_frame(&data, waveform_width, history_lines);
            }
        })
    }

    fn render_frame(data: &VisualizerData, waveform_width: usize, history_lines: usize) {
        print!("\x1B[?25l"); // hide cursor
        // Clear only content area
        for line in 1..30 {
            print!("\x1B[{};0H\x1B[2K", line);
        }

        // Move to top
        print!("\x1B[H");

        // Render all dynamic content
        println!(
            "🎵 Time: {:?} / {:?}",
            data.current_time, data.total_duration
        );

        let current_note = data.current_note.as_deref().unwrap_or("♪ Analyzing...");
        println!("🎼 Current: {}", current_note);
        println!();

        println!("Waveform:");
        Self::render_waveform(&data.amplitude_samples, waveform_width);
        println!();

        println!("Note History:");
        Self::render_note_history(&data.note_history, history_lines);

        // Move cursor to input position (after "Command: ")
        print!("\x1B[32;28H"); // Line 30, column 9 (after "Command: ")
        print!("\x1B[?25h"); // show cursor
        //
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
