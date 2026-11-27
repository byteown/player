use std::time::Duration;

use anyhow::{Context, Result};
use player_core::{decode_track, get_track_info, start_playing};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).context("File path not provided")?;

    let info = get_track_info(path.as_ref())?;

    println!("{:?}", info);

    let decoded_track = decode_track(path.as_ref())?;

    let track_seconds =
        decoded_track.samples.len() / decoded_track.channels / decoded_track.sample_rate as usize;

    let _player = start_playing(decoded_track)?;

    std::thread::sleep(Duration::from_secs_f64(track_seconds as f64));

    Ok(())
}
