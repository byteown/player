use std::{
    io,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use player_core::{get_track_info, start_playing};

enum KeyboardCommand {
    Pause,
    Louder,
    Quieter,
    Exit,
}

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        loop {
            let result = check_keyboard_command();

            match result {
                Some(_) => {
                    let _ = tx.send(result);
                }
                None => break,
            }
        }
    });

    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).context("File path not provided")?;

    let info = get_track_info(path.as_ref())?;

    println!("{:?}", info);
    println!("Pause: P");
    println!("Volume up: W");
    println!("Volume down: S");
    println!("Exit: Q");

    let player = start_playing(path.as_ref())?;

    while !player.is_finished() {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(cmd) => match cmd {
                Some(KeyboardCommand::Pause) => {
                    player.toggle_pause();
                    if player.is_paused() {
                        println!("Paused");
                    } else {
                        println!("Unpaused");
                    }
                }
                Some(KeyboardCommand::Louder) => {
                    let volume = player.volume();
                    player.set_volume(volume + 0.1);
                    println!("Volume: {}", player.volume());
                }
                Some(KeyboardCommand::Quieter) => {
                    let volume = player.volume();
                    player.set_volume(volume - 0.1);
                    println!("Volume: {}", player.volume());
                }
                Some(KeyboardCommand::Exit) => {
                    println!("Stopping");
                    break;
                }
                None => {}
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn check_keyboard_command() -> Option<KeyboardCommand> {
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    match input.to_uppercase().as_str().trim() {
        "P" => Some(KeyboardCommand::Pause),
        "W" => Some(KeyboardCommand::Louder),
        "S" => Some(KeyboardCommand::Quieter),
        "Q" => Some(KeyboardCommand::Exit),
        _ => None,
    }
}
