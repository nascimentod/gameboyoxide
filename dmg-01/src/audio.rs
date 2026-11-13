//! Host playback adapter. Emulation generates stereo f32 samples independently
//! of the audio device; the callback only consumes a bounded queue.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct HostAudio {
    _stream: cpal::Stream,
    queue: Arc<Mutex<VecDeque<[f32; 2]>>>,
    sample_rate: u32,
    nonzero_frames: Arc<AtomicU64>,
}

impl HostAudio {
    pub fn new() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no default audio output device".to_owned())?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.clone().into();
        if !(8_000..=192_000).contains(&config.sample_rate.0) {
            return Err(format!("unsupported sample rate {}", config.sample_rate.0));
        }
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let nonzero_frames = Arc::new(AtomicU64::new(0));
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                build::<f32>(&device, &config, queue.clone(), nonzero_frames.clone())
            }
            cpal::SampleFormat::I16 => {
                build::<i16>(&device, &config, queue.clone(), nonzero_frames.clone())
            }
            cpal::SampleFormat::U16 => {
                build::<u16>(&device, &config, queue.clone(), nonzero_frames.clone())
            }
            format => return Err(format!("unsupported audio sample format {format:?}")),
        }
        .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        eprintln!(
            "Audio: {} Hz, {} channels",
            config.sample_rate.0, config.channels
        );
        Ok(Self {
            _stream: stream,
            queue,
            sample_rate: config.sample_rate.0,
            nonzero_frames,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Software callback diagnostic; does not prove a speaker was audible.
    pub fn nonzero_frames_played(&self) -> u64 {
        self.nonzero_frames.load(Ordering::Relaxed)
    }

    pub fn submit(&self, samples: Vec<[f32; 2]>) {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.extend(samples);
        // At most 100 ms latency after a delayed window event. Drop old audio,
        // never block the emulation thread waiting for a device callback.
        let excess = queue.len().saturating_sub(self.sample_rate as usize / 10);
        queue.drain(..excess);
    }

    pub fn clear(&self) {
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    queue: Arc<Mutex<VecDeque<[f32; 2]>>>,
    nonzero_frames: Arc<AtomicU64>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let channels = usize::from(config.channels);
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            // Never wait for the producer from inside the real-time callback.
            if let Ok(mut queue) = queue.try_lock() {
                for frame in data.chunks_mut(channels) {
                    let stereo = queue.pop_front().unwrap_or([0.0; 2]);
                    if stereo.iter().any(|sample| sample.abs() > 0.0001) {
                        nonzero_frames.fetch_add(1, Ordering::Relaxed);
                    }
                    for (ch, value) in frame.iter_mut().enumerate() {
                        let sample = if channels == 1 {
                            (stereo[0] + stereo[1]) * 0.5
                        } else if ch < 2 {
                            stereo[ch]
                        } else {
                            0.0
                        };
                        *value = T::from_sample(sample.clamp(-1.0, 1.0));
                    }
                }
            } else {
                data.fill(T::from_sample(0.0));
            }
        },
        |err| eprintln!("Audio stream error: {err}"),
        None,
    )
}
