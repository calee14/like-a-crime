use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, SampleFormat, SizedSample, StreamConfig};
use crossterm::cursor::{self, MoveToColumn};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{event, execute, terminal};
use fundsp::hacker::{hammond_hz, multipass, reverb_stereo, sine, sine_hz, soft_saw_hz, square_hz};
use fundsp::math::midi_hz;
use fundsp::prelude::AudioUnit;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
// ------------------------------------------------------------------
// Public Entry Point
// ------------------------------------------------------------------

/// Starts the audio synthesis, playing a sine wave (A4, 440Hz) for the specified
/// duration in seconds. This function is blocking for the duration of playback.
pub fn run_synthesizer(should_quit: Arc<Mutex<bool>>) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel();

    let should_quit_clone = should_quit.clone();

    let input_thread = thread::spawn(move || {
        let _ = terminal::enable_raw_mode();
        loop {
            if event::poll(Duration::from_millis(100)).unwrap_or(false)
                && let Ok(event) = event::read()
                && let Event::Key(key_event) = event
                && tx.send(key_event.code).is_err()
            {
                break;
            }
            let should_quit = should_quit_clone.lock().unwrap();
            if *should_quit {
                break;
            }
            drop(should_quit);
            thread::sleep(Duration::from_millis(10));
        }
    });

    let mut stdout = io::stdout();
    // execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
    // writeln!(stdout, "press 'q' to exit\n")?;
    // stdout.flush()?;

    loop {
        if let Ok(key_code) = rx.try_recv() {
            match key_code {
                KeyCode::Char('q') => {
                    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                    writeln!(stdout, "[Synth Thread] 'q' received. shutting down...\n")?;
                    stdout.flush()?;
                    break;
                }
                KeyCode::Char('a') => {
                    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                    writeln!(stdout, "[Synth Thread] 'a' received\n")?;
                    stdout.flush()?;
                }
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    // end loop in all threads
    *should_quit.lock().unwrap() = true;
    let _ = input_thread.join();

    let _ = terminal::disable_raw_mode();
    stdout.flush()?;
    Ok(())
    // let audio_graph = create_simple_fm();
    //
    // // Start the output stream and play the audio on a separate thread
    // run_output(audio_graph);
    //
    //
    // // Block the current thread for the specified duration to allow the sound to be heard.
    // println!("Playing sound for {} seconds...", duration_secs);
    // std::thread::sleep(Duration::from_secs(duration_secs));
}

// ------------------------------------------------------------------
// Core Audio Functions
// ------------------------------------------------------------------

/// This function determines the sample format, which depends on your system,
/// then starts the synth, passing along the audio graph to generate the sound.
// UPDATED: Changed AudioUnit64 to AudioUnit
fn run_output(audio_graph: Box<dyn AudioUnit>) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();

    // Match the system's required sample format and start the synth stream
    match config.sample_format() {
        SampleFormat::F32 => run_synth::<f32>(audio_graph, device, config.into()),
        SampleFormat::I16 => run_synth::<i16>(audio_graph, device, config.into()),
        SampleFormat::U16 => run_synth::<u16>(audio_graph, device, config.into()),
        _ => panic!("Unsupported format"),
    }
}

/// Starts a thread that will play the audio using the provided audio graph.
fn run_synth<T: SizedSample + FromSample<f64>>(
    mut audio_graph: Box<dyn AudioUnit>,
    device: Device,
    config: StreamConfig,
) {
    // Spawning a thread to handle audio playback in the background
    std::thread::spawn(move || {
        let sample_rate = config.sample_rate.0 as f64;
        audio_graph.set_sample_rate(sample_rate);

        // Closure to get the next stereo audio sample from the graph
        // Note: AudioUnit::get_stereo() returns (f64, f64), which matches this setup.
        let mut next_value = move || audio_graph.get_stereo();

        let channels = config.channels as usize;
        let err_fn = |err| eprintln!("an error occurred on stream: {err}");

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    write_data(data, channels, &mut next_value)
                },
                err_fn,
                None,
            )
            .unwrap();

        stream.play().unwrap();

        // Keep the thread alive so the audio stream continues
        loop {
            std::thread::sleep(Duration::from_millis(1));
        }
    });
}

/// Generates audio samples and writes them to the output buffer.
fn write_data<T: SizedSample + FromSample<f64>>(
    output: &mut [T],
    channels: usize,
    next_sample: &mut dyn FnMut() -> (f32, f32),
) {
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left: T = T::from_sample(sample.0 as f64);
        let right: T = T::from_sample(sample.1 as f64);

        // Write the left/right samples to the channels
        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = if channel & 1 == 0 { left } else { right };
        }
    }
}

// ------------------------------------------------------------------
// Audio Graph Creation Functions
// ------------------------------------------------------------------

/// Simple sine wave at 440 Hz (standard tuning for A4)
// UPDATED: Changed AudioUnit64 to AudioUnit
fn create_sine_440() -> Box<dyn AudioUnit> {
    let synth = sine_hz(440.0);
    Box::new(synth)
}

/// C major chord created by summing sine waves.
#[allow(dead_code)]
fn create_c_major() -> Box<dyn AudioUnit> {
    let synth = soft_saw_hz(261.6) + soft_saw_hz(329.628) + soft_saw_hz(391.995);
    Box::new(synth)
}

/// Simple FM synthesiser taken from the FunDSP docs
#[allow(dead_code)]
fn create_simple_fm() -> Box<dyn AudioUnit> {
    // Frequency (f) and Modulation index (m)
    let f = 440.0;
    let m = 5.0;
    let synth = (sine_hz(f) * f * m + f) >> sine();
    Box::new(synth)
}
