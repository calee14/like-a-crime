use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::thread;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use symphonia::core::audio::{SampleBuffer, Signal};
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::conv::IntoSample;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use cpal::{SampleRate, Stream};

use crate::analyzer::AudioAnalyzer;
use crate::stream;

pub struct StreamingPlayer {
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    analysis_sender: mpsc::Sender<Vec<f32>>,
    current_time: Arc<Mutex<Duration>>,
    sample_rate: f32,
}

impl StreamingPlayer {
    pub fn new(sample_rate: f32) -> (Self, mpsc::Receiver<(Duration, String)>) {
        let (analysis_tx, analysis_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        let mut analyzer = AudioAnalyzer::new(sample_rate, result_tx);
        thread::spawn(move || {
            analyzer.run(analysis_rx);
        });

        (
            Self {
                sample_buffer: Arc::new(Mutex::new(VecDeque::new())),
                analysis_sender: analysis_tx,
                current_time: Arc::new(Mutex::new(Duration::ZERO)),
                sample_rate,
            },
            result_rx,
        )
    }

    pub fn play_file(&self, file_path: &str) -> Result<Stream, Box<dyn std::error::Error>> {
        let sample_buffer = self.sample_buffer.clone();
        let analysis_sender = self.analysis_sender.clone();
        let file_path = file_path.to_string();

        thread::spawn(move || {
            if let Err(e) = Self::decode_audio_stream(&file_path, sample_buffer, analysis_sender) {
                eprintln!("Decoding error: {}", e)
            }
        });

        self.start_audio_output()
    }

    pub fn decode_audio_stream(
        file_path: &str,
        sample_buffer: Arc<Mutex<VecDeque<f32>>>,
        analysis_sender: mpsc::Sender<Vec<f32>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let src = std::fs::File::open(file_path).expect("failed to open media");

        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        let mut hint = Hint::new();
        hint.with_extension("wav");

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .expect("no supported audio tracks");

        let dec_opts: DecoderOptions = Default::default();

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .expect("unsupported codec");

        let track_id = track.id;
        let mut sample_buf = None;
        let mut analysis_chunk = Vec::new();
        const ANALYSIS_CHUNK_SIZE: usize = 8192;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => unimplemented!(),
                Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(err) => return Err(err.into()),
            };

            while !format.metadata().is_latest() {
                format.metadata().pop();
            }

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Initialize sample buffer on first decode
                    if sample_buf.is_none() {
                        let spec = *decoded.spec();
                        let duration = decoded.capacity() as u64;
                        sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                    }

                    if let Some(ref mut buf) = sample_buf {
                        buf.copy_interleaved_ref(decoded);
                        let samples = buf.samples();

                        // Add to playback buffer
                        {
                            let mut buffer = sample_buffer.lock().unwrap();
                            for &sample in samples {
                                buffer.push_back(sample);
                            }
                        }

                        // Collect samples for analysis
                        analysis_chunk.extend_from_slice(samples);

                        // Send chunk for analysis when we have enough samples
                        if analysis_chunk.len() >= ANALYSIS_CHUNK_SIZE {
                            let chunk_to_analyze = analysis_chunk.clone();
                            analysis_chunk.clear();

                            // Non-blocking send - if analysis is behind, skip this chunk
                            if let Err(_) = analysis_sender.send(chunk_to_analyze) {
                                // Analysis thread is busy, skip this chunk
                                println!("Analysis thread busy, skipping chunk");
                            }
                        }
                    }
                }
                Err(Error::IoError(_)) => continue,
                Err(Error::DecodeError(_)) => continue,
                Err(err) => return Err(err.into()),
            }
        }
        Ok(())
    }

    pub fn start_audio_output(&self) -> Result<Stream, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No output device");

        let config = cpal::StreamConfig {
            channels: 2, // Stereo
            sample_rate: SampleRate(self.sample_rate as u32),
            buffer_size: cpal::BufferSize::Default,
        };

        let sample_buffer = self.sample_buffer.clone();
        let current_time = self.current_time.clone();
        let sample_rate = self.sample_rate;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buffer = sample_buffer.lock().unwrap();

                for sample in data.iter_mut() {
                    *sample = buffer.pop_front().unwrap_or(0.0);
                }

                // Update current playback time
                let samples_played = data.len() / 2; // Stereo
                let time_increment = Duration::from_secs_f32(samples_played as f32 / sample_rate);
                {
                    let mut time = current_time.lock().unwrap();
                    *time += time_increment;
                }
            },
            |err| eprintln!("Audio output error: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }
}
