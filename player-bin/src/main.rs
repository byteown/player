use std::time::Duration;

use anyhow::{Context, Result};
use player_core::{get_track_info, start_playing};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).context("File path not provided")?;

    let info = get_track_info(path.as_ref())?;

    println!("{:?}", info);

    let _player = start_playing(path.as_ref())?;

    std::thread::sleep(Duration::from_secs_f64(info.length_seconds.context("Can't get track info length seconds")?));

    Ok(())
}
