use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

pub struct AudioStreamer {
    samples: Vec<f32>,
    sample_rate: f32,
    current_position: Arc<Mutex<usize>>,
    playback_start: Instant,

    audio_sender: mpsc::Sender<Vec<f32>>,
    analysis_sender: mpsc::Sender<(Duration, Vec<f32>)>,

    chunk_size: usize,
    update_interval: Duration,
}

impl AudioStreamer {
    pub fn new(
        samples: Vec<f32>,
        sample_rate: f32,
        chunk_duration_ms: u64,
    ) -> (
        Self,
        mpsc::Receiver<Vec<f32>>,
        mpsc::Receiver<(Duration, Vec<f32>)>,
    ) {
        let chunk_size = ((sample_rate * chunk_duration_ms as f32) / 1000.0) as usize;
        let update_interval = Duration::from_millis(chunk_duration_ms);

        let (audio_tx, audio_rx) = mpsc::channel();
        let (analysis_tx, analysis_rx) = mpsc::channel();

        (
            Self {
                samples,
                sample_rate,
                current_position: Arc::new(Mutex::new(0)),
                playback_start: Instant::now(),
                audio_sender: audio_tx,
                analysis_sender: analysis_tx,
                chunk_size,
                update_interval,
            },
            audio_rx,
            analysis_rx,
        )
    }

    pub fn start_streaming(&self) {
        let samples = self.samples.clone();
        let sample_rate = self.sample_rate.clone();
        let current_position = self.current_position.clone();
        let audio_sender = self.audio_sender.clone();
        let analysis_sender = self.analysis_sender.clone();
        let chunk_size = self.chunk_size;
        let update_interval = self.update_interval;
        let playback_start = self.playback_start;

        thread::spawn(move || {
            let mut last_udpate = Instant::now();

            loop {
                // wait for next update
                let elapsed = last_update.elapsed();
                if elapsed < update_interval {
                    thread::sleep(update_interval - elapsed);
                }
                last_update = Instant::now();

                // get curr position and read next chunk
                let (chunk, timestamp, is_end) = {
                    let mut pos = current_position.lock().unwrap();
                    let start_pos = *pos;

                    if start_pos >= samples.len() {
                        break; // end of file
                    }

                    let end_pos = (start_pos + chunk_size).min(samples.len());
                    let chunk = samples[start_pos..end_pos].to_vec();

                    // calc timestamp based on sample pos
                    let timestamp = Duration::from_secs_f32(start_pos as f32 / sample_rate);

                    *pos = end_pos;
                    let is_end = end_pos >= samples.len();

                    (chunk, timestamp, is_end)
                };

                // send data to audio buffer
                if let Err(_) = audio_sender.send(chunk.clone()) {
                    println!("Audio output buffer full, skipping chunk");
                }

                // send data to analysis buffer
                if let Err(_) = analysis_sender.send((timestamp, chunk)) {
                    println!("Analysis buffer full, skipping chunk");
                }

                if is_end {
                    println!("Reached end of audio file");
                    break;
                }
            }
        });
    }
}

