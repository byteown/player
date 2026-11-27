use std::path::Path;

use symphonia::{core::formats::TrackType, default::get_codecs};

use crate::{error::LibError, track_decode::get_format_reader};

#[derive(Debug)]
pub struct TrackInfo {
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<usize>,
    pub length_seconds: Option<f64>,
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
