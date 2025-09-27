use crate::fft::fft_chunk;
use crate::notes::frequency_to_note;
use crate::window::window_audio_samples;
use std::time::Instant;
use std::{sync::mpsc, time::Duration};

pub struct AudioAnalyzer {
    sample_rate: f32,
    result_sender: mpsc::Sender<(Duration, String)>,
    current_time: Duration,
}

impl AudioAnalyzer {
    pub fn new(sample_rate: f32, result_sender: mpsc::Sender<(Duration, String)>) -> Self {
        Self {
            sample_rate,
            result_sender,
            current_time: Duration::ZERO,
        }
    }

    pub fn run(&mut self, receiver: mpsc::Receiver<Vec<f32>>) {
        while let Ok(samples) = receiver.recv() {
            let start_time = Instant::now();

            if let Ok(result) = self.analyze_chunk(&samples) {
                // send result with timestamp
                if self
                    .result_sender
                    .send((self.current_time, result))
                    .is_err()
                {
                    break; // Main thread has stopped listening
                }
            }

            let chunk_duration = Duration::from_secs_f32(samples.len() as f32 / self.sample_rate);
            self.current_time += chunk_duration;

            let analysis_time = start_time.elapsed();
            println!(
                "Analysis took: {:?} for chunk of {:?}",
                analysis_time, chunk_duration
            );
        }
    }

    fn analyze_chunk(&self, samples: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        let window_size = samples.len();
        let mut windowed_samples = Vec::new();

        // window the entire sample from Sender
        let _ = window_audio_samples(samples, &mut windowed_samples, window_size);

        if windowed_samples.is_empty() {
            return Ok("Silence".to_string());
        }

        // extract the one windowed sample
        let first_window = windowed_samples.first().unwrap();

        if let Ok(frequency_bands) = fft_chunk(first_window, self.sample_rate, 3)
            && !frequency_bands.is_empty()
            && !frequency_bands[0].is_empty()
        {
            let band_notes = frequency_bands
                .iter()
                .map(|band| frequency_to_note(band))
                .collect::<Vec<String>>()
                .join("|");

            Ok(band_notes)
        } else {
            Ok("No Signal".to_string())
        }
    }
}
