use player_core::get_format_reader;
use symphonia::{core::formats::TrackType, default::get_codecs};

fn main() {
    let path = r"C:\Users\user\Downloads\Utsu-P - 天使だと思っていたのに I thought I was an angel feat. 初音ミク.mp3";

    let mut reader = get_format_reader(&path.to_string()).expect("Can't get format reader");

    let track = reader
        .default_track(TrackType::Audio)
        .expect("No audio track");

    let track_codec_params = track
        .codec_params
        .as_ref()
        .expect("Can't determine codec params");

    let track_audio_codec_params = track_codec_params
        .audio()
        .expect("Params exist but not audio");

    let mut decoder = get_codecs()
        .make_audio_decoder(track_audio_codec_params, &Default::default())
        .expect("Can't create audio decoder");

    let track_id = track.id;
    let channels = track_audio_codec_params.channels.as_ref().unwrap().count();
    let sample_rate = track_audio_codec_params.sample_rate.unwrap();

    let mut audio_buffer: Vec<f32> = vec![];
    let mut temp_buffer: Vec<f32> = vec![];

    loop {
        let packet = reader.next_packet();

        match packet {
            Ok(Some(packet)) => {
                if packet.track_id != track_id {
                    continue;
                }

                let decoded_packet = decoder.decode(&packet);

                match decoded_packet {
                    Ok(pac) => {
                        pac.copy_to_vec_interleaved(&mut temp_buffer);
                    }
                    Err(_) => {
                        continue;
                    }
                }

                audio_buffer.extend(&temp_buffer);
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }

    let samples = audio_buffer.len();

    println!(
        "Samples: {}, Seconds: {}",
        samples,
        samples / channels / sample_rate as usize
    );
}
