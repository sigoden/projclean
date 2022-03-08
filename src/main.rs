use std::{
    env,
    fs::{canonicalize, read_to_string},
    path::{Path, PathBuf},
    process,
    sync::mpsc::channel,
    thread,
};

use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use projclean::{run, search, Config};

fn main() {
    if let Err(err) = start() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn start() -> Result<()> {
    let matches = command().get_matches();

    let config = init_config(&matches)?;

    if matches.is_present("export_projects") {
        config.export_projects()?;
        return Ok(());
    }

    if matches.is_present("list_projects") {
        config.list_projects()?;
        return Ok(());
    }

    let entry = set_working_dir(&matches)?;
    let (tx, rx) = channel();
    let tx2 = tx.clone();
    let handle = thread::spawn(move || search(entry, config, tx2));
    run(tx, rx)?;
    handle.join().unwrap()?;
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
            Arg::new("list_projects")
                .short('L')
                .long("list-projects")
                .help("List current projects"),
        )
        .arg(
            Arg::new("export_projects")
                .short('X')
                .long("export-projects")
                .help("Export current projects as csv"),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .help("Load projects from file")
                .allow_invalid_utf8(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .help("Add project to search target")
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("path")
                .allow_invalid_utf8(true)
                .help("The root directory for the filesystem search"),
        )
}

fn init_config(matches: &clap::ArgMatches) -> Result<Config> {
    let mut config = if let Some(config_file) = matches.value_of_os("file") {
        let config_file = Path::new(config_file);
        let content = read_to_string(config_file).map_err(|err| {
            anyhow!(
                "Cannot read config file '{}', {}",
                config_file.display(),
                err
            )
        })?;
        let mut config = Config::default();
        config.read_file(&content)?;
        config
    } else {
        Config::default()
    };

    if let Some(values) = matches.values_of("project") {
        for value in values {
            config.add_project(value)?;
        }
    }
    if config.is_empty_projects() {
        config.add_builtin().expect("broken builtin config file");
    }

    Ok(config)
}

fn set_working_dir(matches: &clap::ArgMatches) -> Result<PathBuf> {
    if let Some(base_directory) = matches.value_of_os("path") {
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

pub fn is_existing_directory(path: &Path) -> bool {
    path.is_dir() && path.exists()
}
