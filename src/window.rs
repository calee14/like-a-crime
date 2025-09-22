use std::f32::consts::PI;

fn hann_window(window_size: usize) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut window_coefficients: Vec<f32> = vec![0.0; window_size];

    for (i, coeff) in window_coefficients.iter_mut().enumerate() {
        let position = i as f32 / (window_size - 1) as f32;
        *coeff = 0.5 * (1.0 - (2.0 * PI * position).cos());
    }

    Ok(window_coefficients)
}

pub fn window_audio_samples(
    samples: &[f32],
    windowed_samples: &mut Vec<Vec<f32>>,
    window_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let hop_size = window_size / 4;

    let hann_coeffs = match hann_window(window_size) {
        Ok(coeffs) => coeffs,
        Err(err) => panic!("{}", err),
    };

    for pos in (0..window_size).step_by(hop_size) {
        let chunk = &samples[pos..(pos + window_size)];
        let mut window_chunk: Vec<f32> = vec![0.0; window_size];
        for i in 0..window_size - 1 {
            window_chunk[i] = chunk[i] * hann_coeffs[i];
        }
        windowed_samples.push(window_chunk);
    }
    Ok(())
}
