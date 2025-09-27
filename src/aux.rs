use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

pub struct AudioOutput {
    receiver: mpsc::Receiver<Vec<f32>>,
    buffer: std::collections::VecDeque<f32>,
}

impl AudioOutput {
    pub fn new(receiver: mpsc::Receiver<Vec<f32>>) -> Self {
        Self {
            receiver,
            buffer: std::collections::VecDeque::new(),
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

        // move sync mpsc channel outbound into our receiver obj
        let receiver = std::mem::replace(&mut self.receiver, mpsc::channel().1);
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let buffer_clone = buffer.clone();

        thread::spawn(move || {
            while let Ok(chunk) = receiver.recv() {
                let mut buf = buffer_clone.lock().unwrap();
                for sample in chunk {
                    buf.push_back(sample);
                }
            }
        });

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer.lock().unwrap();
                for sample in data.iter_mut() {
                    *sample = buf.pop_front().unwrap_or(0.0);
                }
            },
            |err| eprintln!("Audio output error: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }
}
