use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

pub struct AudioStreamer {
    samples: Vec<f32>,
    sample_rate: f32,
    current_position: Arc<Mutex<usize>>,

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
        let sample_rate = self.sample_rate;
        let current_position = self.current_position.clone();
        let audio_sender = self.audio_sender.clone();
        let analysis_sender = self.analysis_sender.clone();
        let chunk_size = self.chunk_size;
        let update_interval = self.update_interval;

        thread::spawn(move || {
            let mut last_update = Instant::now();

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
                if audio_sender.send(chunk.clone()).is_err() {
                    println!("Audio output buffer full, skipping chunk");
                }

                // send data to analysis buffer
                if analysis_sender.send((timestamp, chunk)).is_err() {
                    println!("Analysis buffer full, skipping chunk");
                }

                if is_end {
                    println!("Reached end of audio file");
                    break;
                }
            }
        });
    }

    pub fn get_current_time(&self) -> Duration {
        let position = *self.current_position.lock().unwrap();
        Duration::from_secs_f32(position as f32 / self.sample_rate)
    }

    pub fn get_current_position(&self) -> usize {
        *self.current_position.lock().unwrap()
    }

    pub fn seek_to_time(&self, time: Duration) {
        let target_position = (time.as_secs_f32() * self.sample_rate) as usize;
        let mut position = self.current_position.lock().unwrap();
        *position = target_position.min(self.samples.len());
    }

    pub fn seek_to_position(&self, sample_position: usize) {
        let mut position = self.current_position.lock().unwrap();
        *position = sample_position.min(self.samples.len());
    }

    pub fn get_total_duration(&self) -> Duration {
        Duration::from_secs_f32(self.samples.len() as f32 / self.sample_rate)
    }

    pub fn is_finished(&self) -> bool {
        let position = *self.current_position.lock().unwrap();
        position >= self.samples.len()
    }
}
