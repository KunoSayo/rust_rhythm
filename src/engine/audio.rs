use log::info;
use rodio::OutputStreamHandle;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

pub struct AudioData {
    pub stream: rodio::OutputStream,
    pub stream_handle: OutputStreamHandle,
}

impl AudioData {
    pub fn new() -> anyhow::Result<AudioData> {
        let (stream, handle) = rodio::OutputStream::try_default()?;
        Ok(Self {
            stream,
            stream_handle: handle,
        })
    }
}

impl AudioData {}

pub fn sample_change_speed(samples: &[i16], channels: usize, speed: f32) -> Vec<i16> {
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = SincFixedIn::<f64>::new(
        1.0 / speed as f64,
        2.0,
        params,
        samples.len() / channels,
        channels,
    )
    .expect("Failed to get resampler");
    let audio_data = samples
        .iter()
        .map(|x| (*x as f64) / (i16::MAX - 1) as f64)
        .collect::<Vec<_>>();
    let mut chunks = vec![vec![]; channels];

    let f64_to_i16 = |x: f64| (x * (i16::MAX - 1) as f64).round() as i16;
    audio_data.chunks(channels).for_each(|x| {
        for (idx, value) in x.iter().enumerate() {
            chunks[idx].push(*value);
        }
    });

    info!("Process sample to speed {speed}");
    let result_data = resampler.process(&chunks, None).expect("Failed to process");
    info!("Processed sample to speed {speed}");
    // [[Channel data]; channel count]
    // so we should rearrange it.

    let mut result = Vec::with_capacity(chunks[0].len() * channels);

    for i in 0..chunks[0].len() {
        for j in 0..channels {
            result.push(f64_to_i16(result_data[j][i]));
        }
    }
    result
}
