mod error;
mod player;
mod track_decode;
mod track_info;

pub use player::start_playing;
pub use track_info::get_track_info;

pub use error::LibError;
pub use player::Player;
pub use track_decode::DecodedTrack;
pub use track_decode::PartDecoder;
pub use track_info::TrackInfo;
