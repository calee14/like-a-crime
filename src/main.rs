mod analyzer;
mod audio;
mod aux;
mod fft;
mod notes;
mod plot;
mod stream;
mod window;

use std::time::Duration;

use crate::analyzer::AudioAnalyzer;
use crate::audio::decode_audio_wav;
use crate::aux::AudioOutput;
use crate::stream::AudioStreamer;

static WINDOW_SIZE: usize = 2048;
static SAMPLE_RATE: f32 = 44100.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("file path not provided");

    // load audio file
    let (samples, sample_rate) = decode_audio_wav(path, SAMPLE_RATE)?;

    // create streamer
    let (streamer, audio_rx, analysis_rx) = AudioStreamer::new(samples, sample_rate, 500);

    // start streaming data from mem
    streamer.start_streaming();

    // set up aux
    let mut audio_output = AudioOutput::new(audio_rx);
    let _stream = audio_output.start_playback(sample_rate)?;

    // set up analyzer
    let analyzer = AudioAnalyzer::new(sample_rate);
    analyzer.run(analysis_rx);

    // keep main loop alive and control threads
    loop {
        std::thread::sleep(Duration::from_millis(1000));

        // println!("Current time: {:?}", streamer.get_current_time());
        if streamer.is_finished() {
            break;
        }
    }
    Ok(())
}
