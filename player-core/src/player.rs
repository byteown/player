use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::{error::LibError, track_decode::DecodedTrack};

pub struct Player {
    _stream: cpal::Stream,
}

pub fn start_playing(track: DecodedTrack) -> Result<Player, LibError> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or(LibError::NoOutputDevice)?;

    let supported_config = device
        .default_output_config()
        .map_err(LibError::NoDefaultOutputConfig)?;

    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels() as usize;

    if sample_rate != track.sample_rate {
        return Err(LibError::SampleRateDiff {
            expected: sample_rate,
            got: track.sample_rate,
        });
    }

    if channels != track.channels {
        return Err(LibError::ChannelsDiff {
            expected: channels,
            got: track.channels,
        });
    }

    let config: StreamConfig = supported_config.into();

    let mut pos: usize = 0;

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for n in data {
                    *n = *track.samples.get(pos).unwrap_or(&0.0);

                    pos += 1;
                }
            },
            move |_| {
                // TODO
            },
            None,
        )
        .map_err(LibError::Stream)?;

    stream.play().map_err(LibError::Stream)?;

    Ok(Player { _stream: stream })
}
