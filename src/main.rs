use rand::prelude::*;

mod editor;
mod gameplay;
#[cfg(unix)]
mod playback;

use editor::TasEditor;

fn main() {
    let mut playback = false;
    for arg in std::env::args() {
        if arg == "--playback" {
            playback = true;
        }
    }

    if playback {
        #[cfg(unix)]
        {
            if !nix::unistd::geteuid().is_root() {
                eprintln!("Need root permissions to playback a TAS.");
                std::process::exit(2);
            }
            playback::playback();
        }
        #[cfg(not(unix))]
        eprintln!("Playback is currently only supported on Linux.");
    } else {
        let mut rng = gameplay::PieceGenerator::new(0);
        print!("#Q=[]({:?})", rng.next());
        for _ in 0..500 {
            print!("{:?}", rng.next());
        }
        println!();
    }
}
