mod analyzer;
mod audio;
mod aux;
mod fft;
mod notes;
mod soundboard;
mod stream;
mod synth;
mod visualizer;
mod window;

use std::io::BufRead;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use std::{io, path, thread};

use crate::analyzer::AudioAnalyzer;
use crate::audio::decode_audio_wav;
use crate::aux::AudioOutput;
use crate::stream::AudioStreamer;
use crate::visualizer::TerminalVisualizer;

static SAMPLE_RATE: f32 = 44100.0;

enum OP {
    Synth,
    Analyze,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let first_arg = args.get(1).unwrap().as_str();
    let op = match first_arg {
        "-s" => OP::Synth,
        "-a" => OP::Analyze,
        _ => panic!("must specify argument -r (record) or -a (analyze)"),
    };

    let should_main_quit = Arc::new(Mutex::new(false));
    let should_main_quit_clone = should_main_quit.clone();

    match op {
        OP::Synth => {
            let _ = synth::run_synthesizer(should_main_quit_clone);
        }
        OP::Analyze => {
            let path = args.get(2).expect("file path not provided").clone();
            thread::spawn(move || {
                let _ = analyze_loop(&path, should_main_quit_clone);
            });
        }
    }

    // keep main loop alive and control threads
    loop {
        let should_quit = should_main_quit.lock().unwrap();
        if *should_quit {
            break;
        }

        // explicitly drop lock bc of sleep
        // avoid deadlock
        drop(should_quit);
        std::thread::sleep(Duration::from_millis(500));

        // println!("Current time: {:?}", streamer.get_current_time());
    }
}

fn analyze_loop(
    path: &String,
    should_quit: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // load audio file
    let (samples, sample_rate) = decode_audio_wav(path, SAMPLE_RATE)?;
    let total_duration = Duration::from_secs_f32(samples.len() as f32 / sample_rate);

    // create visualizer
    let (visualizer, vis_data) = TerminalVisualizer::new(50, 80, 10);
    // create streamer
    let (streamer, audio_rx, analysis_rx) = AudioStreamer::new(samples, sample_rate, 500);
    let (analysis_result_tx, analysis_result_rx) = mpsc::channel();
    // start streaming data from mem
    streamer.start_streaming();

    // set up and start analyzer
    let analyzer = AudioAnalyzer::new(sample_rate, analysis_result_tx);
    analyzer.run(analysis_rx);

    // set up and start aux
    let mut audio_output = AudioOutput::new(
        audio_rx,
        analysis_result_rx,
        vis_data,
        sample_rate,
        total_duration,
    );
    let _stream = audio_output.start_playback(sample_rate)?;

    // start visualizer
    visualizer.start_rendering();

    // input detection
    let should_quit_clone = should_quit.clone();

    thread::spawn(move || {
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        while let Some(Ok(line)) = lines.next() {
            if line.trim().eq_ignore_ascii_case("q") {
                let mut should_quit = should_quit_clone.lock().unwrap();
                *should_quit = true;
                break;
            }
            if line.trim().eq_ignore_ascii_case("k") {
                audio_output.toggle();
                streamer.toggle();
            }

            if line.trim().eq_ignore_ascii_case("l") {
                // go foward 5 secs
                let new_playback_time = streamer.skip_forward(5.0);
                audio_output.clear_buffers();
                audio_output.update_current_playback_time(new_playback_time);
            }
            if line.trim().eq_ignore_ascii_case("j") {
                // go backward 5 secs
                let new_playback_time = streamer.skip_backward(5.0);
                audio_output.clear_buffers();
                audio_output.update_current_playback_time(new_playback_time);
            }
        }
    });

    loop {
        let should_quit = should_quit.lock().unwrap();
        if *should_quit {
            break;
        }

        // explicitly drop lock bc of sleep
        // avoid deadlock
        drop(should_quit);
        std::thread::sleep(Duration::from_millis(500));

        // println!("Current time: {:?}", streamer.get_current_time());
    }
    // visualizer.cleanup();
    Ok(())
}
