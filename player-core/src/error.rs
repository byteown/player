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

    #[error("No part available")]
    NoPart,
}
