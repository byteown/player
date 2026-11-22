use anyhow::{Context, Result};
use player_core::get_track_result;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).context("File path not provided")?;

    let _result = get_track_result(path)?;

    Ok(())
}
