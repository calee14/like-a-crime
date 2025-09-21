use symphonia::core::audio::Signal;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::conv::IntoSample;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use plotters::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("file path not provided");

    let src = std::fs::File::open(path).expect("failed to open media");

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("wav");

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    let dec_opts: DecoderOptions = Default::default();

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    let track_id = track.id;

    let mut all_samples: Vec<f32> = Vec::new();
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100) as f32;

    loop {
        // get packet from media
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                unimplemented!();
            }
            Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(err) => {
                panic!("{}", err);
            }
        };

        // consume new metadata that has been read after last packet
        while !format.metadata().is_latest() {
            format.metadata().pop();
        }

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // store samples
                let spec = *decoded.spec();
                let channels = spec.channels.count();

                match decoded {
                    symphonia::core::audio::AudioBufferRef::U8(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::U16(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::U24(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::U32(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::S8(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::S16(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::S24(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::S32(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::F32(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                    symphonia::core::audio::AudioBufferRef::F64(buf) => {
                        convert_samples_to_f32(&buf, channels, &mut all_samples);
                    }
                }
            }
            Err(Error::IoError(_)) => {
                continue;
            }
            Err(Error::DecodeError(_)) => {
                continue;
            }
            Err(err) => {
                panic!("{}", err)
            }
        }
    }
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

fn convert_samples_to_f32<S>(
    buf: &symphonia::core::audio::AudioBuffer<S>,
    channels: usize,
    all_samples: &mut Vec<f32>,
) where
    S: symphonia::core::sample::Sample + IntoSample<f32> + Copy,
{
    if channels == 1 {
        // mono: convert all samples directly
        let samples = buf.chan(0);
        for &sample in samples {
            all_samples.push(sample.into_sample());
        }
    } else {
        // multi-channel: mix to mono by averaging all channels
        let frame_count = buf.frames();
        for frame_idx in 0..frame_count {
            let mut sum = 0.0f32;
            for ch in 0..channels {
                let sample: f32 = buf.chan(ch)[frame_idx].into_sample();
                sum += sample;
            }
            all_samples.push(sum / channels as f32);
        }
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
