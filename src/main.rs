mod audio;
mod fft;
mod plot;
mod window;

static WINDOW_SIZE: usize = 2048;
static SAMPLE_RATE: f32 = 44100.0;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("file path not provided");

    let (all_samples, sample_rate) = match audio::decode_audio_wav(path, SAMPLE_RATE) {
        Ok(v) => v,
        Err(err) => panic!("{}", err),
    };

    let mut windowed_samples: Vec<Vec<f32>> = Vec::new();
    match window::window_audio_samples(&all_samples, &mut windowed_samples, WINDOW_SIZE) {
        Ok(v) => v,
        Err(err) => panic!("{}", err),
    };

    let frequencies = match fft::fft_chunks(&windowed_samples, WINDOW_SIZE, SAMPLE_RATE) {
        Ok(v) => v,
        Err(err) => panic!("{}", err),
    };

    // plot the waveform
    if !all_samples.is_empty() {
        plot::plot_waveform(&windowed_samples[0], sample_rate).expect("Failed to plot waveform");
        println!(
            "Plotted {} samples at {} Hz",
            all_samples.len(),
            sample_rate
        );
        println!(
            "Duration: {:.2} seconds",
            all_samples.len() as f32 / sample_rate
        );
    } else {
        println!("No samples decoded");
    }

    for i in 10000..10020 {
        // for mag in magnitude_samples[i].iter() {
        print!("{} ", frequencies[i]);
        // }
        println!();
    }
}
