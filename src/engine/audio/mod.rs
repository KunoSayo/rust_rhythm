use crate::engine::ResourceLocation;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{BufferSize, StreamConfig, SupportedBufferSize, SupportedStreamConfig};
use egui::ahash::HashMap;
use log::info;
use rodio::buffer::SamplesBuffer;
use rodio::source::SeekError;
use rodio::{OutputStream, Sink, Source, StreamError};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use rodio::mixer::Mixer;

pub mod sources;


pub type OutputStreamHandle = Mixer;


pub struct AudioData {
    stream: OutputStream,
    pub stream_handle: Mixer,
    pub cached_sfx: HashMap<ResourceLocation, SamplesBuffer>,
    sink_pool: VecDeque<Sink>,
}

impl AudioData {
    fn create_stream_from_device(
        device: rodio::Device,
    ) -> Result<(OutputStream), StreamError> {
        // let cfg = device
        //     .default_output_config()
        //     .map_err(|e| StreamError::DefaultStreamConfigError(e))?;
        // 
        // let (cfg_min, cfg_max) = match cfg.buffer_size() {
        //     SupportedBufferSize::Range { min, max } => (*min, *max),
        //     SupportedBufferSize::Unknown => (1, cfg.sample_rate().0),
        // };
        // let my_max = 1.max(cfg.sample_rate().0 * 1 / 1_0000);
        // let cfg = SupportedStreamConfig::new(
        //     cfg.channels(),
        //     cfg.sample_rate(),
        //     SupportedBufferSize::Range {
        //         min: cfg_min.min(my_max),
        //         max: cfg_max.min(my_max),
        //     },
        //     cfg.sample_format(),
        // );
        rodio::OutputStreamBuilder::from_device(device)?
            .open_stream()

    }
    fn create_stream() -> Result<(OutputStream), StreamError> {
        let default_device = cpal::default_host()
            .default_output_device()
            .ok_or(StreamError::NoDevice)?;

        let process_device = |device: rodio::Device| -> rodio::Device { device };

        let default_stream = Self::create_stream_from_device(default_device);

        default_stream.or_else(|original_err| {
            let mut devices = match cpal::default_host().output_devices() {
                Ok(d) => d,
                Err(_) => return Err(original_err),
            };

            devices
                .find_map(|d| Self::create_stream_from_device(d).ok())
                .ok_or(original_err)
        })
    }
    pub fn new() -> anyhow::Result<AudioData> {
        let (stream) = Self::create_stream()?;
        let mut sink_pool = VecDeque::default();
        sink_pool.resize_with(8, || Sink::connect_new(stream.mixer()));
        let stream_handle = stream.mixer().clone();
        Ok(Self {
            stream,
            stream_handle,
            cached_sfx: Default::default(),
            sink_pool,
        })
    }

    pub fn play_sfx(&mut self, loc: &ResourceLocation) {
        if let Some(buffer) = self.cached_sfx.get(loc) {
            let front_sink = self.sink_pool.front().unwrap();
            if front_sink.empty() {
                front_sink.append(buffer.clone());
                let front_sink = self.sink_pool.pop_front().unwrap();
                self.sink_pool.push_back(front_sink);
            } else  {
                let sink = Sink::connect_new(&self.stream_handle);
                sink.append(buffer.clone());
                sink.detach()
            }
        }
    }
}

impl AudioData {}

pub fn sample_change_speed(samples: &[f32], channels: usize, speed: f32) -> Vec<f32> {
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

    let f64_to_i16 = |x: f64| (x * (i16::MAX - 1) as f64).round() as f32;
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
