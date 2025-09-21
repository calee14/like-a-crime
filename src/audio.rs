use symphonia::core::audio::Signal;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::conv::IntoSample;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use std::vec::Vec;

pub fn decode_audio_wav(
    path: &String,
    sample_rate: u32,
) -> Result<(Vec<f32>, f32), Box<dyn std::error::Error>> {
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
    let sample_rate = track.codec_params.sample_rate.unwrap_or(sample_rate) as f32;

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
    return Ok((all_samples, sample_rate));
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
