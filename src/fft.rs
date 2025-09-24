use realfft::{RealFftPlanner, num_complex::Complex};

pub fn fft_chunks(
    window_samples: &[Vec<f32>],
    window_size: usize,
    sample_rate: f32,
) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut frequencies = Vec::with_capacity(window_samples.len());
    let mut spectrum = vec![Complex::default(); window_size / 2 + 1];

    let mut chunk = vec![0.0f32; window_size];
    for window in window_samples {
        chunk.copy_from_slice(window);
        fft.process(&mut chunk, &mut spectrum)?;

        let band_frequencies = analyze_frequency_bands(&spectrum, sample_rate, window_size);
        frequencies.push(band_frequencies);
    }
    Ok(frequencies)
}

fn analyze_frequency_bands(
    spectrum: &[Complex<f32>],
    sample_rate: f32,
    window_size: usize,
) -> Vec<f32> {
    let bands = [
        (50.0, 250.0),    // low
        (250.0, 800.0),   // low-mid
        (800.0, 2000.0),  // mid
        (2000.0, 6000.0), // high
    ];
    let mut band_peaks = vec![0.0f32; bands.len()];

    for (i, (low_freq, high_freq)) in bands.iter().enumerate() {
        let low_bin = ((low_freq * window_size as f32) / sample_rate) as usize;
        let high_bin = ((high_freq * window_size as f32) / sample_rate) as usize;

        let mut max_magnitude = 0.0;
        let mut max_bin = low_bin;

        // iter frequencies in band range
        // spectrum is a vec (size: window_size.len() / 2 + 1)
        // which index called bin rep a freq
        for (bin, freq_vec) in spectrum
            .iter()
            .enumerate()
            .take(high_bin.min(spectrum.len()))
            .skip(low_bin)
        {
            let magnitude = freq_vec.norm();

            let frequency = (bin as f32 * sample_rate) / window_size as f32;
            let weight = if frequency > 400.0 {
                (frequency / 400.0).sqrt().min(2.0)
            } else {
                1.0
            };

            let weighted_magnitude = magnitude * weight;

            if weighted_magnitude > max_magnitude {
                max_magnitude = magnitude;
                max_bin = bin;
            }
        }

        let frequency = (max_bin as f32 * sample_rate) / window_size as f32;
        band_peaks[i] = frequency;
    }

    band_peaks
}
