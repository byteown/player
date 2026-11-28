use std::time::Duration;

use player_core::PartDecoder;

fn main() {
    let path = r"C:\Users\user\Downloads\Utsu-P - 天使だと思っていたのに I thought I was an angel feat. 初音ミク.mp3";

    let mut counter = 0;

    let mut decoder = PartDecoder::open(path.as_ref()).unwrap();

    decoder.seek(Duration::from_secs(156)).unwrap();

    while let Ok(Some(part)) = decoder.next_part() {
        counter += part.len();
    }

    println!("{counter}")
}
