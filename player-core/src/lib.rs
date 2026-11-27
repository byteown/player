mod error;
mod player;
mod track_decode;
mod track_info;

pub use player::start_playing;
pub use track_decode::decode_track;
pub use track_info::get_track_info;

pub use track_info::TrackInfo;
pub use track_decode::DecodedTrack;
pub use player::Player;
pub use error::LibError;
