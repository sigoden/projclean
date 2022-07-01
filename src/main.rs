mod app;
mod common;
mod config;
mod fs;

use std::{
    env,
    fs::canonicalize,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc,
    },
    thread,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Arg, Command};

use app::run;
use config::Config;
use fs::{ls, search};

use common::{human_readable_folder_size, Message, PathItem, PathState};

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let running2 = running.clone();
    ctrlc::set_handler(move || {
        running2.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if let Err(err) = start(running) {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn start(running: Arc<AtomicBool>) -> Result<()> {
    let matches = command().get_matches();

    let config = init_config(&matches)?;

    let entry = set_working_dir(&matches)?;

    let (tx, rx) = channel();
    let tx2 = tx.clone();

    thread::spawn(move || search(entry, config, tx2, running));

    if matches.is_present("targets") {
        ls(rx)?;
    } else {
        run(rx, tx)?;
    }
    Ok(())
}

fn command() -> Command<'static> {
    Command::new(env!("CARGO_CRATE_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(concat!(
            env!("CARGO_PKG_DESCRIPTION"),
            " - ",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .arg(
            Arg::new("targets")
                .short('t')
                .long("targets")
                .help("Print found targets without entering tui"),
        )
        .arg(
            Arg::new("directory")
                .short('C')
                .long("directory")
                .value_name("DIR")
                .allow_invalid_utf8(true)
                .default_value(".")
                .help("Start searching from DIR"),
        )
        .arg(
            Arg::new("rules")
                .help("Search rules")
                .value_name("RULES")
                .multiple_values(true),
        )
}

fn init_config(matches: &clap::ArgMatches) -> Result<Config> {
    let mut config = Config::default();

    if let Some(values) = matches.values_of("rules") {
        for value in values {
            config.add_rule(value)?;
        }
    }

    if config.rules.is_empty() {
        bail!(
            "No search rules, see https://github.com/sigoden/projclean#search-rule");
    }

    Ok(config)
}

fn set_working_dir(matches: &clap::ArgMatches) -> Result<PathBuf> {
    if let Some(base_directory) = matches.value_of_os("directory") {
        let base_directory = Path::new(base_directory);

        if !is_existing_directory(base_directory) {
            return Err(anyhow!(
                "The '--file' path '{}' is not a directory.",
                base_directory.to_string_lossy()
            ));
        }
        let base_directory = canonicalize(base_directory).unwrap();
        env::set_current_dir(&base_directory).with_context(|| {
            format!(
                "Cannot set '{}' as the current working directory",
                base_directory.to_string_lossy()
            )
        })?;
        Ok(base_directory)
    } else {
        let current_dir = env::current_dir()?;
        Ok(current_dir)
    }
}

fn is_existing_directory(path: &Path) -> bool {
    path.is_dir() && path.exists()
}
