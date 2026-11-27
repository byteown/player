use player_core::PartDecoder;

fn main() {
    let path = r"C:\Users\user\Downloads\Utsu-P - 天使だと思っていたのに I thought I was an angel feat. 初音ミク.mp3";

    let mut counter = 0;

    let mut decoder = PartDecoder::open(path.as_ref()).unwrap();

    loop {
        while let Ok(sample) = decoder.next_part() {
            counter += sample.unwrap().len();
        }
    }

    println!("{counter}")
}
