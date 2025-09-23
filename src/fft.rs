use realfft::{RealFftPlanner, num_complex::Complex};

pub fn fft_chunks(
    window_samples: &[Vec<f32>],
    window_size: usize,
    sample_rate: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut frequencies = Vec::with_capacity(window_samples.len());
    let mut spectrum = vec![Complex::default(); window_size / 2 + 1];

    let mut chunk = vec![0.0f32; window_size];
    for window in window_samples {
        chunk.copy_from_slice(window);
        fft.process(&mut chunk, &mut spectrum)?;

        let mut max_magnitude = 0.0;
        let mut max_bin = 0;

        for (bin, complex_val) in spectrum.iter().enumerate() {
            let magnitude = complex_val.norm();
            if magnitude > max_magnitude {
                max_magnitude = magnitude;
                max_bin = bin;
            }
        }

        // let magnitudes: Vec<f32> = spectrum
        //     .iter()
        //     .map(|c| c.norm()) // sqrt(real^2 + imag^2)
        //     .collect();
        let frequency = (max_bin as f32 * sample_rate) / window_size as f32;

        frequencies.push(frequency);
    }
    Ok(frequencies)
}

fn freq_bands(spectrum: &[Complex<f32>], sample_rate: f32, window_size: usize) -> Vec<f32> {
    let mut freqs = Vec::new();

    let bands = [
        (50.0, 250.0),    // bass
        (250.0, 800.0),   // low-mid
        (800.0, 2000.0),  // mid
        (2000.0, 6000.0), // high
    ];

    freqs
}
