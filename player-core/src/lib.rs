use std::ffi::OsStr;
use std::path::Path;

use symphonia::core::errors::Error;

use symphonia::core::{
    formats::{FormatOptions, FormatReader, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

pub fn get_track_result(path: &String) -> Result<Box<dyn FormatReader>, Error> {
    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();

    let src = std::fs::File::open(path)?;

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    let extension: Option<&OsStr> = Path::new(path).extension();
    let result: Option<&str> = extension.and_then(|e| e.to_str());
    if let Some(n) = result {
        hint.with_extension(n);
    }

    let probe = symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts)?;

    Ok(probe)
}
