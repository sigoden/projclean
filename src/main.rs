use std::{
    process,
    sync::{mpsc::channel, Arc},
};

use anyhow::Result;
use projclean::{run, search, Config};

fn main() {
    if let Err(err) = start() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn start() -> Result<()> {
    let mut path = std::env::current_dir().unwrap();
    path.push("tmp");
    let config = Config::load()?;
    let (tx, rx) = channel();
    search(&path, Arc::new(config), tx)?;
    run(rx)?;
    Ok(())
}
