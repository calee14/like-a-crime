mod audio;
use plotters::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("file path not provided");

    let (all_samples, sample_rate) = match audio::decode_audio_wav(path, 44100) {
        Ok(v) => v,
        Err(err) => panic!("{}", err),
    };
    // plot the waveform
    if !all_samples.is_empty() {
        plot_waveform(&all_samples, sample_rate).expect("Failed to plot waveform");
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
}

fn plot_waveform(samples: &[f32], sample_rate: f32) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = "waveform.png";
    let root = BitMapBackend::new(output_path, (1200, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let duration = samples.len() as f32 / sample_rate;
    let max_amplitude = samples.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
    let min_amplitude = -max_amplitude;

    let mut chart = ChartBuilder::on(&root)
        .caption("Audio Waveform", ("Arial", 30))
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(0.0..duration, min_amplitude..max_amplitude)?;

    chart
        .configure_mesh()
        .x_desc("Time (seconds)")
        .y_desc("Amplitude")
        .draw()?;

    // downsample for plotting if too many samples
    let plot_samples: Vec<(f32, f32)> = if samples.len() > 10000 {
        // downsample by taking every nth sample
        let step = samples.len() / 10000;
        samples
            .iter()
            .step_by(step)
            .enumerate()
            .map(|(i, &amplitude)| {
                let time = (i * step) as f32 / sample_rate;
                (time, amplitude)
            })
            .collect()
    } else {
        samples
            .iter()
            .enumerate()
            .map(|(i, &amplitude)| {
                let time = i as f32 / sample_rate;
                (time, amplitude)
            })
            .collect()
    };

    chart
        .draw_series(LineSeries::new(plot_samples, &BLUE))?
        .label("Waveform")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));

    chart.configure_series_labels().draw()?;
    root.present()?;

    println!("Waveform saved as {}", output_path);
    Ok(())
}
