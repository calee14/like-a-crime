mod analyzer;
mod audio;
mod aux;
mod fft;
mod notes;
mod stream;
mod visualizer;
mod window;

use std::io;
use std::sync::mpsc;
use std::time::Duration;

use crate::analyzer::AudioAnalyzer;
use crate::audio::decode_audio_wav;
use crate::aux::AudioOutput;
use crate::stream::AudioStreamer;
use crate::visualizer::TerminalVisualizer;

static SAMPLE_RATE: f32 = 44100.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("file path not provided");

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

    // keep main loop alive and control threads
    loop {
        std::thread::sleep(Duration::from_millis(1000));

        // println!("Current time: {:?}", streamer.get_current_time());
    }

    visualizer.cleanup();
    Ok(())
}
