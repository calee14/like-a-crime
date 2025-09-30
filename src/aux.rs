use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use crate::visualizer::VisualizerData;

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
    total_duration: Duration,
    is_paused: Arc<Mutex<bool>>,
    fade_samples: Arc<Mutex<usize>>,

    // visualizer
    visualizer_data: Arc<Mutex<VisualizerData>>,
}

impl AudioOutput {
    pub fn new(
        receiver: mpsc::Receiver<Vec<f32>>,
        analysis_receiver: mpsc::Receiver<AnalysisResult>,
        visualizer_data: Arc<Mutex<VisualizerData>>,
        sample_rate: f32,
        total_duration: Duration,
    ) -> Self {
        Self {
            receiver,
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(88200))),
            analysis_results: Arc::new(Mutex::new(BinaryHeap::new())),
            analysis_receiver,
            current_playback_time: Arc::new(Mutex::new(Duration::ZERO)),
            sample_rate,
            total_duration,
            visualizer_data,
            is_paused: Arc::new(Mutex::new(false)),
            fade_samples: Arc::new(Mutex::new(0)),
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
        let visualizer_data = self.visualizer_data.clone();
        let sample_rate = self.sample_rate;
        let total_duration = self.total_duration;
        let is_paused = self.is_paused.clone();

        // 5 ms of fade
        let fade_duration_samples = (sample_rate * 0.005) as usize;
        let fade_samples = self.fade_samples.clone();

        // build stream object
        // move ownership of cloned pointers to callback
        // stream will periodically invoke callback per sample rate
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let paused = *is_paused.lock().unwrap();
                let mut fade_count = fade_samples.lock().unwrap();

                if paused {
                    let mut buf = playback_buffer.lock().unwrap();

                    // output silence
                    for sample in data.iter_mut() {
                        if *fade_count < fade_duration_samples {
                            let audio_sample = buf.pop_front().unwrap_or(0.0);
                            let fade_multiplier =
                                1.0 - (*fade_count as f32 / fade_duration_samples as f32);
                            *sample = audio_sample * fade_multiplier;
                            *fade_count += 2;
                        } else {
                            *sample = 0.0;
                        }
                    }
                    return;
                }

                // load samples from playback buffer into data vector of stream obj
                let mut buf = playback_buffer.lock().unwrap();
                let mut current_timestamp = current_time.lock().unwrap();

                let mut frame_samples = Vec::with_capacity(data.len());

                for sample in data.iter_mut() {
                    let audio_sample = buf.pop_front().unwrap_or(0.0);
                    *sample = audio_sample;

                    // add frames for visualizer
                    frame_samples.push(audio_sample);

                    // update current playback time
                    *current_timestamp += Duration::from_secs_f32(1.0 / sample_rate);
                }

                // update visualizer data
                {
                    let mut vis_data = visualizer_data.lock().unwrap();

                    vis_data.current_time = *current_timestamp;
                    vis_data.total_duration = total_duration;

                    vis_data.amplitude_samples.extend(frame_samples);
                    let vis_len = vis_data.amplitude_samples.len();
                    if vis_len > 2048 {
                        vis_data.amplitude_samples.drain(0..vis_len - 2048);
                    }
                }

                Self::check_and_display_analysis(
                    &analysis_results,
                    *current_timestamp,
                    &visualizer_data,
                );

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
        visualizer_data: &Arc<Mutex<VisualizerData>>,
    ) {
        let mut results = analysis_results.lock().unwrap();

        // Pop and print all results whose timestamp <= current_time
        while let Some(Reverse(front_result)) = results.peek() {
            if front_result.timestamp <= current_time {
                let Reverse(result) = results.pop().unwrap();

                {
                    let mut vis_data = visualizer_data.lock().unwrap();
                    vis_data.current_note = Some(result.note.clone());
                    vis_data
                        .note_history
                        .push_back((result.timestamp, result.note));

                    if vis_data.note_history.len() > 20 {
                        vis_data.note_history.pop_front();
                    }
                }
                // println!("🎵 [{:?}] {}", result.timestamp, result.note);
            } else {
                break; // Stop when we hit a future timestamp
            }
        }
    }

    pub fn pause(&self) {
        let mut paused = self.is_paused.lock().unwrap();
        *paused = true;
    }

    pub fn resume(&self) {
        let mut paused = self.is_paused.lock().unwrap();
        let mut fade = self.fade_samples.lock().unwrap();
        *paused = false;
        *fade = 0;
    }

    pub fn toggle(&self) {
        let mut paused = self.is_paused.lock().unwrap();
        *paused = !(*paused);
        if !*paused {
            let mut fade = self.fade_samples.lock().unwrap();
            *fade = 0;
        }
    }
}
