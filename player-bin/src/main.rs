use std::{
    io::{self, Error},
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
    SeekTo,
    SeekReset,
}

enum KeyboardStatus {
    Empty,
    Error,
    Cmd,
}

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        loop {
            let mut input = String::new();
            let result = io::stdin().read_line(&mut input);

            let status = check_keyboard_status(result);

            match status {
                KeyboardStatus::Empty => break,
                KeyboardStatus::Error => break,
                KeyboardStatus::Cmd => {}
            }

            let cmd = check_keyboard_command(input);

            match cmd {
                Some(t) => {
                    let res = tx.send(t);
                    match res {
                        Ok(_) => continue,
                        Err(_) => break,
                    }
                }
                None => continue,
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
    println!("Seek to 1:00: J");
    println!("Seek to 0:00: R");
    println!("Exit: Q");

    let player = start_playing(path.as_ref())?;

    while !player.is_finished() {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(cmd) => match cmd {
                KeyboardCommand::Pause => {
                    player.toggle_pause();
                    if player.is_paused() {
                        println!("Paused");
                    } else {
                        println!("Unpaused");
                    }
                }
                KeyboardCommand::Louder => {
                    let volume = player.volume();
                    player.set_volume(volume + 0.1);
                    println!("Volume: {:.1}", player.volume());
                }
                KeyboardCommand::Quieter => {
                    let volume = player.volume();
                    player.set_volume(volume - 0.1);
                    println!("Volume: {:.1}", player.volume());
                }
                KeyboardCommand::Exit => {
                    println!("Stopping");
                    break;
                }
                KeyboardCommand::SeekTo => {
                    player.seek(Duration::from_secs(60));
                }
                KeyboardCommand::SeekReset => {
                    player.seek(Duration::from_secs(0));
                }
            },
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn check_keyboard_status(result: Result<usize, Error>) -> KeyboardStatus {
    match result {
        Ok(0) => KeyboardStatus::Empty,
        Err(_) => KeyboardStatus::Error,
        Ok(_) => KeyboardStatus::Cmd,
    }
}

fn check_keyboard_command(input: String) -> Option<KeyboardCommand> {
    match input.trim().to_uppercase().as_str() {
        "W" => Some(KeyboardCommand::Louder),
        "S" => Some(KeyboardCommand::Quieter),
        "P" => Some(KeyboardCommand::Pause),
        "Q" => Some(KeyboardCommand::Exit),
        "J" => Some(KeyboardCommand::SeekTo),
        "R" => Some(KeyboardCommand::SeekReset),

        _ => None,
    }
}
