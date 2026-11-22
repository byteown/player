use player_core::decode_track;

fn main() {
    let path = r"C:\Users\user\Downloads\Utsu-P - 天使だと思っていたのに I thought I was an angel feat. 初音ミク.mp3";

    let info = decode_track(path.as_ref()).unwrap();

    let samples = info.samples.len();

    println!(
        "Samples: {}, Seconds: {}",
        samples,
        samples / info.channels / info.sample_rate as usize
    );
}
