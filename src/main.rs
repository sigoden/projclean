use std::{path::Path, process, sync::Arc};

use anyhow::Result;
use projclean::{run, scan, Config};

fn main() {
    match start() {
        Err(err) => {
            eprintln!("{}", err);
            process::exit(1);
        }
        _ => {}
    }
}

fn start() -> Result<()> {
    let path = Path::new("/home/sigo/w");
    let config = Config::load()?;
    let config = Arc::new(config);
    let list = scan(path, config)?;
    run(list)?;
    Ok(())
}
