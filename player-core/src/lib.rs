use std::ffi::OsStr;
use std::path::Path;

use symphonia::core::errors::Error;

use symphonia::core::formats::TrackType;
use symphonia::core::{
    formats::{FormatOptions, FormatReader, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};
use symphonia::default::get_codecs;

#[derive(Debug)]
pub struct TrackInfo {
    pub codec: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<usize>,
    pub length_seconds: Option<f64>,
}

pub fn get_format_reader(path: &String) -> Result<Box<dyn FormatReader>, Error> {
    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();

    let src = std::fs::File::open(path)?;

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    let extension: Option<&OsStr> = Path::new(path).extension();
    let extension_text: Option<&str> = extension.and_then(|e| e.to_str());
    if let Some(ext) = extension_text {
        hint.with_extension(ext);
    }

    let probe = symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts)?;

    Ok(probe)
}

pub fn get_track_info(path: &String) -> Result<TrackInfo, Error> {
    let reader = get_format_reader(path)?;

    let track = reader
        .default_track(TrackType::Audio)
        .ok_or(Error::Unsupported("No audio track"))?;

    let track_codec_params = track
        .codec_params
        .as_ref()
        .ok_or(Error::Unsupported("Can't determine codec params"))?;

    let track_audio_codec_params = track_codec_params
        .audio()
        .ok_or(Error::Unsupported("Params exist but not audio"))?;

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
