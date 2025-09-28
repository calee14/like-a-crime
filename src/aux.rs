use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub timestamp: Duration,
    pub note: String,
}

impl PartialEq for AnalysisResult {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for AnalysisResult {}

impl PartialOrd for AnalysisResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AnalysisResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

pub struct AudioOutput {
    receiver: mpsc::Receiver<Vec<f32>>,
    buffer: Arc<Mutex<VecDeque<f32>>>,

    // store analysis results
    analysis_results: Arc<Mutex<BinaryHeap<Reverse<AnalysisResult>>>>,
    analysis_receiver: mpsc::Receiver<AnalysisResult>,

    // playback tracking
    current_playback_time: Arc<Mutex<Duration>>,
    sample_rate: f32,
}

impl AudioOutput {
    pub fn new(
        receiver: mpsc::Receiver<Vec<f32>>,
        analysis_receiver: mpsc::Receiver<AnalysisResult>,
        sample_rate: f32,
    ) -> Self {
        Self {
            receiver,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(88200))),
            analysis_results: Arc::new(Mutex::new(BinaryHeap::new())),
            analysis_receiver,
            current_playback_time: Arc::new(Mutex::new(Duration::ZERO)),
            sample_rate,
        }
    }

    pub fn start_playback(
        &mut self,
        sample_rate: f32,
    ) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No output device");

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(sample_rate as u32),
            buffer_size: cpal::BufferSize::Default,
        };

        // receive analysis
        self.start_analysis_collection();

        // start filling buffer
        self.start_buffer_filling();

        // sleep to let buffer fill
        thread::sleep(Duration::from_millis(200));

        let playback_buffer = self.buffer.clone();
        let analysis_results = self.analysis_results.clone();
        let current_time = self.current_playback_time.clone();
        let sample_rate = self.sample_rate;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = playback_buffer.lock().unwrap();
                let mut current_timestamp = current_time.lock().unwrap();

                for sample in data.iter_mut() {
                    *sample = buf.pop_front().unwrap_or(0.0);

                    // update current playback time
                    *current_timestamp += Duration::from_secs_f32(1.0 / sample_rate);
                }

                Self::check_and_display_analysis(&analysis_results, *current_timestamp);

                // warn
                if buf.len() < 4410 {
                    println!("Audio buffer running low: {} samples", buf.len());
                }
            },
            |err| eprintln!("Audio output error: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    fn start_analysis_collection(&mut self) {
        // move sync mpsc channel outbound into our receiver obj
        let analaysis_results = self.analysis_results.clone();
        let receiver = std::mem::replace(&mut self.analysis_receiver, mpsc::channel().1);

        thread::spawn(move || {
            while let Ok(result) = receiver.recv() {
                {
                    let mut results = analaysis_results.lock().unwrap();
                    results.push(Reverse(result));
                }
            }
        });
    }

    fn start_buffer_filling(&mut self) {
        // move sync mpsc channel outbound into our receiver obj
        let buffer = self.buffer.clone();
        let receiver = std::mem::replace(&mut self.receiver, mpsc::channel().1);

        thread::spawn(move || {
            while let Ok(chunk) = receiver.recv() {
                let mut buf = buffer.lock().unwrap();

                if buf.len() > 176400 {
                    // println!("Audio buffer overflowed by {} samples", buf.len() - 176400);
                }

                for sample in chunk {
                    buf.push_back(sample);
                }
            }
        });
    }

    fn check_and_display_analysis(
        analysis_results: &Arc<Mutex<BinaryHeap<Reverse<AnalysisResult>>>>,
        current_time: Duration,
    ) {
        let mut results = analysis_results.lock().unwrap();

        // Pop and print all results whose timestamp <= current_time
        while let Some(Reverse(front_result)) = results.peek() {
            if front_result.timestamp <= current_time {
                let Reverse(result) = results.pop().unwrap();
                println!("🎵 [{:?}] {}", result.timestamp, result.note);
            } else {
                break; // Stop when we hit a future timestamp
            }
        }
    }
}
