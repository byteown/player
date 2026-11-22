use std::time;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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

    let config = supported_config.into();

    let mut phase: f32 = 0.0;

    let sin_phase_step = std::f32::consts::TAU * 440.0;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels as usize) {
                    let value = phase.sin() * 0.2;
                    phase += sin_phase_step / sample_rate as f32;

                    for sample in frame.iter_mut() {
                        *sample = value;
                    }

                    if phase >= std::f32::consts::TAU {
                        phase -= std::f32::consts::TAU;
                    }
                }
            },
            move |err| {
                eprintln!("An error occurred on the output audio stream: {}", err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();

    let two_sec = time::Duration::from_millis(2000);
    std::thread::sleep(two_sec);
}
