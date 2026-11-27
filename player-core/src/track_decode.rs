use std::{ffi::OsStr, path::Path};

use symphonia::{
    core::{
        formats::{FormatOptions, FormatReader, TrackType, probe::Hint},
        io::MediaSourceStream,
        meta::MetadataOptions,
    },
    default::get_codecs,
};

use crate::error::LibError;

pub struct DecodedTrack {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

pub(crate) fn get_format_reader(path: &Path) -> Result<Box<dyn FormatReader>, LibError> {
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
