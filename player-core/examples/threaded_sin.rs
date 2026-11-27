use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{self, Duration},
};

use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::RingBuffer;

fn main() {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("No output device available!");

    let supported_config = device
        .default_output_config()
        .expect("Can't pick default output config");

    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels();

    println!("Sample rate: {}", sample_rate);
    println!("Channels: {}", channels);
    println!("Samples format: {}", supported_config.sample_format());

    let config: StreamConfig = supported_config.into();

    let (mut producer, mut consumer) = RingBuffer::<f32>::new(4096);

    let running = Arc::new(AtomicBool::new(true));
    let running_gen = Arc::clone(&running);

    let generator_handle = thread::spawn(move || {
        let mut phase: f32 = 0.0;
        let sin_phase_step = std::f32::consts::TAU * 440.0;

        while running_gen.load(Ordering::Relaxed) {
            let available = producer.slots();

            let frames_to_write = available / channels as usize;

            if frames_to_write > 0 {
                let samples_to_write = frames_to_write * channels as usize;

                if let Ok(mut chunk) = producer.write_chunk(samples_to_write) {
                    let (first, second) = chunk.as_mut_slices();
                    let mut all_samples = first.iter_mut().chain(second.iter_mut());

                    for _ in 0..frames_to_write {
                        let value = phase.sin() * 0.2;
                        phase += sin_phase_step / sample_rate as f32;
                        if phase >= std::f32::consts::TAU {
                            phase -= std::f32::consts::TAU;
                        }

                        for _ in 0..channels {
                            if let Some(s) = all_samples.next() {
                                *s = value;
                            }
                        }
                    }

                    chunk.commit_all();
                }
            } else {
                thread::park_timeout(Duration::from_millis(2));
            }
        }
    });

    let gen_thread = generator_handle.thread().clone();

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let available = consumer.slots();
                let to_read = available.min(data.len());

                if to_read > 0 && let Ok(chunk) = consumer.read_chunk(to_read) {
                    let (first, second) = chunk.as_slices();
                    data[..first.len()].copy_from_slice(first);
                    data[first.len()..to_read].copy_from_slice(second);
                    chunk.commit_all();
                }

                if to_read < data.len() {
                    data[to_read..].fill(0.0);
                }

                gen_thread.unpark();
            },
            move |err| {
                eprintln!("An error occurred on the output audio stream: {}", err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();
    let three_sec = time::Duration::from_millis(3000);
    std::thread::sleep(three_sec);

    drop(stream);
    running.store(false, Ordering::Relaxed);
    generator_handle.thread().unpark();
    generator_handle.join().unwrap();
}
