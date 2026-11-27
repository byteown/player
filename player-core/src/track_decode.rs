use std::{ffi::OsStr, path::Path};

use symphonia::{
    core::{
        codecs::audio::AudioDecoder,
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

pub struct PartDecoder {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
    buf: Vec<f32>,
}

impl PartDecoder {
    pub fn open(path: &Path) -> Result<Self, LibError> {
        let reader = get_format_reader(path)?;

        let track = reader
            .default_track(TrackType::Audio)
            .ok_or(LibError::NoAudioTrack)?;

        let track_codec_params = track.codec_params.as_ref().ok_or(LibError::NoCodecParams)?;

        let track_audio_codec_params = track_codec_params.audio().ok_or(LibError::NoAudio)?;

        let decoder = get_codecs()
            .make_audio_decoder(track_audio_codec_params, &Default::default())
            .map_err(LibError::CreateAudioDecoder)?;

        let track_id = track.id;

        let sample_rate = track_audio_codec_params
            .sample_rate
            .ok_or(LibError::SampleRate)?;
        let channels = track_audio_codec_params
            .channels
            .as_ref()
            .ok_or(LibError::Channels)?
            .count();

        Ok(Self {
            reader,
            decoder,
            track_id,
            sample_rate,
            channels,
            buf: vec![],
        })
    }
    pub fn next_part(&mut self) -> Result<Option<&[f32]>, LibError> {
        loop {
            let packet = self.reader.next_packet();

            match packet {
                Ok(Some(packet)) => {
                    if packet.track_id != self.track_id {
                        continue;
                    }

                    let decoded_packet = self.decoder.decode(&packet);

                    match decoded_packet {
                        Ok(pac) => {
                            pac.copy_to_vec_interleaved(&mut self.buf);
                            return Ok(Some(&self.buf));
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }
                Ok(None) => return Ok(None),
                Err(e) => {
                    return Err(LibError::Packet(e));
                }
            }
        }
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn channels(&self) -> usize {
        self.channels
    }
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
