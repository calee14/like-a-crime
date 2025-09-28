use crate::aux::AnalysisResult;
use crate::fft::fft_chunk;
use crate::notes::frequency_to_note;
use crate::window::window_audio_samples;
use std::thread;
use std::{sync::mpsc, time::Duration};

pub struct AudioAnalyzer {
    sample_rate: f32,
    result_sender: mpsc::Sender<AnalysisResult>,
}

impl AudioAnalyzer {
    pub fn new(sample_rate: f32, result_sender: mpsc::Sender<AnalysisResult>) -> Self {
        Self {
            sample_rate,
            result_sender,
        }
    }

    pub fn run(&self, receiver: mpsc::Receiver<(Duration, Vec<f32>)>) {
        let sample_rate = self.sample_rate;
        let result_sender = self.result_sender.clone();
        thread::spawn(move || {
            while let Ok((timestamp, samples)) = receiver.recv() {
                Self::analyze_chunk(&samples, sample_rate, &result_sender, timestamp);
            }
        });
    }

    fn analyze_chunk(
        samples: &[f32],
        sample_rate: f32,
        result_sender: &mpsc::Sender<AnalysisResult>,
        timestamp: Duration,
    ) {
        let window_size = samples.len();
        let mut windowed_samples = Vec::new();

        // window the entire sample from Sender
        let _ = window_audio_samples(samples, &mut windowed_samples, window_size - 1);

        if windowed_samples.is_empty() {
            return;
        }

        // extract the one windowed sample
        let first_window = windowed_samples.first().unwrap();

        if let Ok(frequency_bands) = fft_chunk(first_window, sample_rate, 3)
            && !frequency_bands.is_empty()
            && !frequency_bands[0].is_empty()
        {
            let note = frequency_bands
                .iter()
                .map(|band| frequency_to_note(band))
                .collect::<Vec<String>>()
                .join(" | ");

            let result = AnalysisResult { timestamp, note };

            if result_sender.send(result).is_err() {
                println!("Analysis result buffer failed to send");
            }
        }
    }
}
