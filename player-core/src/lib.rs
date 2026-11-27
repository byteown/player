use std::ffi::OsStr;
use std::path::Path;

use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::formats::TrackType;
use symphonia::core::{
    formats::{FormatOptions, FormatReader, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};
use symphonia::default::get_codecs;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibError {
    #[error("Could not open file")]
    FileOpen(#[source] std::io::Error),

    #[error("Can't probe")]
    Probing(#[source] symphonia::core::errors::Error),

    #[error("No audio track")]
    NoAudioTrack,

    #[error("Can't determine codec params")]
    NoCodecParams,

    #[error("Params exist but not audio")]
    NoAudio,

    #[error("Can't create audio decoder")]
    CreateAudioDecoder(#[source] symphonia::core::errors::Error),

    #[error("Can't get sample rate")]
    SampleRate,

    #[error("Can't get channels")]
    Channels,

    #[error("Packet error")]
    Packet(#[source] symphonia::core::errors::Error),

    #[error("No output device available")]
    NoOutputDevice,

    #[error("Can't pick default output config")]
    NoDefaultOutputConfig(#[source] cpal::Error),

    #[error("Expected {expected} sample rate, got {got}")]
    SampleRateDiff { expected: u32, got: u32 },

    #[error("Expected {expected} channels, got {got}")]
    ChannelsDiff { expected: usize, got: usize },

    #[error("Stream error")]
    Stream(#[source] cpal::Error),
}

#[derive(Debug)]
pub struct TrackInfo {
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<usize>,
    pub length_seconds: Option<f64>,
}

pub struct DecodedTrack {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

pub struct Player {
    _stream: cpal::Stream,
}

fn get_format_reader(path: &Path) -> Result<Box<dyn FormatReader>, LibError> {
    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();

    let src = std::fs::File::open(path).map_err(LibError::FileOpen)?;

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    let extension: Option<&OsStr> = path.extension();
    let extension_text: Option<&str> = extension.and_then(|e| e.to_str());
    if let Some(ext) = extension_text {
        hint.with_extension(ext);
    }

    let probe = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .map_err(LibError::Probing)?;

    Ok(probe)
}

pub fn get_track_info(path: &Path) -> Result<TrackInfo, LibError> {
    let reader = get_format_reader(path)?;

    let track = reader
        .default_track(TrackType::Audio)
        .ok_or(LibError::NoAudioTrack)?;

    let track_codec_params = track.codec_params.as_ref().ok_or(LibError::NoCodecParams)?;

    let track_audio_codec_params = track_codec_params.audio().ok_or(LibError::NoAudio)?;

    let time = track
        .duration
        .zip(track.time_base)
        .and_then(|(duration, timebase)| timebase.calc_duration(duration));

    let channels = &track_audio_codec_params.channels;

    let registered_audio_decoder = get_codecs().get_audio_decoder(track_audio_codec_params.codec);

    let info = TrackInfo {
        codec: registered_audio_decoder
            .map(|audio_decoder| audio_decoder.codec.info.long_name.to_string()),
        sample_rate: track_audio_codec_params.sample_rate,
        channels: channels.as_ref().map(|ch| ch.count()),
        length_seconds: time.map(|tm| tm.as_secs_f64()),
    };

    Ok(info)
}

pub fn decode_track(path: &Path) -> Result<DecodedTrack, LibError> {
    let mut reader = get_format_reader(path)?;

    let track = reader
        .default_track(TrackType::Audio)
        .ok_or(LibError::NoAudioTrack)?;

    let track_codec_params = track.codec_params.as_ref().ok_or(LibError::NoCodecParams)?;

    let track_audio_codec_params = track_codec_params.audio().ok_or(LibError::NoAudio)?;

    let mut decoder = get_codecs()
        .make_audio_decoder(track_audio_codec_params, &Default::default())
        .map_err(LibError::CreateAudioDecoder)?;

    let track_id = track.id;

    let mut audio_buffer: Vec<f32> = vec![];
    let mut temp_buffer: Vec<f32> = vec![];

    let sample_rate = track_audio_codec_params
        .sample_rate
        .ok_or(LibError::SampleRate)?;
    let channels = track_audio_codec_params
        .channels
        .as_ref()
        .ok_or(LibError::Channels)?
        .count();

    loop {
        let packet = reader.next_packet();

        match packet {
            Ok(Some(packet)) => {
                if packet.track_id != track_id {
                    continue;
                }

                let decoded_packet = decoder.decode(&packet);

                match decoded_packet {
                    Ok(pac) => {
                        pac.copy_to_vec_interleaved(&mut temp_buffer);
                    }
                    Err(_) => {
                        continue;
                    }
                }

                audio_buffer.extend(&temp_buffer);
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                return Err(LibError::Packet(e));
            }
        }
    }

    let decoded_track = DecodedTrack {
        samples: audio_buffer,
        sample_rate,
        channels,
    };

    Ok(decoded_track)
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
