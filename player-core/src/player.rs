use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::{self, Sender},
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
    running: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    sender: Sender<DecoderCommand>,
    handle: Option<JoinHandle<()>>,
}
impl Drop for Player {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
impl Player {
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
    pub fn volume(&self) -> f32 {
        f32::from_bits(self.volume.load(Ordering::Relaxed))
    }
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
    pub fn toggle_pause(&self) {
        self.paused.fetch_xor(true, Ordering::Relaxed);
    }
    pub fn set_volume(&self, volume: f32) {
        if volume.is_nan() {
            return;
        }

        self.volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    pub fn seek(&self, value: std::time::Duration) {
        let _ = self.sender.send(DecoderCommand::Seek(value));
    }
}

enum DecoderCommand {
    Seek(std::time::Duration),
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
    let finished_callback = Arc::clone(&finished);

    let paused = Arc::new(AtomicBool::new(false));
    let paused_callback = Arc::clone(&paused);

    let volume = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
    let volume_callback = Arc::clone(&volume);

    let clear_buf = Arc::new(AtomicBool::new(false));
    let clear_buf_gen = Arc::clone(&clear_buf);
    let clear_buf_callback = Arc::clone(&clear_buf);

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let volume_value = f32::from_bits(volume_callback.load(Ordering::Relaxed));

                if clear_buf_callback.load(Ordering::Acquire) {
                    if let Ok(n) = consumer.read_chunk(consumer.slots()) {
                        n.commit_all();
                    }
                    clear_buf_callback.store(false, Ordering::Release);
                }

                if paused_callback.load(Ordering::Relaxed) {
                    data.fill(0.0);
                    return;
                }

                for sample in data {
                    match consumer.pop() {
                        Ok(value) => {
                            *sample = value * volume_value;
                        }
                        Err(_) => {
                            *sample = 0.0;

                            if decoder_done.load(Ordering::Acquire) {
                                finished_callback.store(true, Ordering::Relaxed);
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

    let (sender, receiver) = mpsc::channel::<DecoderCommand>();

    let generator_handle = thread::spawn(move || {
        let mut delayed_cmd: Option<DecoderCommand> = None;

        'next: loop {
            if let Ok(msg) = receiver.try_recv() {
                delayed_cmd = Some(msg);
            }

            if let Some(cmd) = delayed_cmd.take() {
                match cmd {
                    DecoderCommand::Seek(duration) => {
                        clear_buf_gen.store(true, Ordering::Release);
                        while clear_buf_gen.load(Ordering::Acquire) {
                            thread::park_timeout(Duration::from_millis(2));

                            if !running_gen.load(Ordering::Relaxed) {
                                return;
                            }
                        }
                        let _ = decoder.seek(duration);
                    }
                }
            }

            match decoder.next_part() {
                Ok(Some(part)) => {
                    for &sample in part {
                        loop {
                            let result = producer.push(sample);

                            if result.is_ok() {
                                break;
                            }

                            if let Ok(msg) = receiver.try_recv() {
                                delayed_cmd = Some(msg);
                                continue 'next;
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
        running,
        finished,
        paused,
        volume,
        sender,
        handle: Some(generator_handle),
    })
}
