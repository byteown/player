use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::RingBuffer;

use crate::{PartDecoder, error::LibError};

pub struct Player {
    _stream: cpal::Stream,
    stop_flag: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}
impl Drop for Player {
    fn drop(&mut self) {
        self.stop_flag.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
impl Player {
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
}

pub fn start_playing(path: &Path) -> Result<Player, LibError> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or(LibError::NoOutputDevice)?;

    let supported_config = device
        .default_output_config()
        .map_err(LibError::NoDefaultOutputConfig)?;

    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels() as usize;

    let mut decoder = PartDecoder::open(path)?;

    if sample_rate != decoder.sample_rate() {
        return Err(LibError::SampleRateDiff {
            expected: sample_rate,
            got: decoder.sample_rate(),
        });
    }

    if channels != decoder.channels() {
        return Err(LibError::ChannelsDiff {
            expected: channels,
            got: decoder.channels(),
        });
    }

    let config: StreamConfig = supported_config.into();

    let capacity = decoder.sample_rate() * decoder.channels() as u32 / 2;
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(capacity as usize);

    let running = Arc::new(AtomicBool::new(true));
    let running_gen = Arc::clone(&running);

    let decoder_done = Arc::new(AtomicBool::new(false));
    let decoder_done_callback = Arc::clone(&decoder_done);

    let finished = Arc::new(AtomicBool::new(false));
    let finished_gen = Arc::clone(&finished);

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for sample in data {
                    match consumer.pop() {
                        Ok(value) => {
                            *sample = value;
                        }
                        Err(_) => {
                            *sample = 0.0;

                            if decoder_done.load(Ordering::Acquire) {
                                finished_gen.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
            },
            move |_| {
                // TODO
            },
            None,
        )
        .map_err(LibError::Stream)?;

    stream.play().map_err(LibError::Stream)?;

    let generator_handle = thread::spawn(move || {
        loop {
            match decoder.next_part() {
                Ok(Some(part)) => {
                    for &sample in part {
                        loop {
                            let result = producer.push(sample);

                            if result.is_ok() {
                                break;
                            }

                            if running_gen.load(Ordering::Relaxed) {
                                thread::park_timeout(Duration::from_millis(2));
                            } else {
                                return;
                            }
                        }
                    }
                }
                Ok(None) => {
                    decoder_done_callback.store(true, Ordering::Release);
                    return;
                }
                Err(_) => {
                    decoder_done_callback.store(true, Ordering::Release);
                    return;
                }
            }
        }
    });

    Ok(Player {
        _stream: stream,
        stop_flag: running,
        finished,
        handle: Some(generator_handle),
    })
}
