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
use projclean::{ls, run, search, Config};

fn main() {
    if let Err(err) = start() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn start() -> Result<()> {
    let matches = command().get_matches();

    let config = init_config(&matches)?;

    if matches.is_present("list_rules") {
        config.list_rules()?;
        return Ok(());
    }

    let entry = set_working_dir(&matches)?;

    let (tx, rx) = channel();
    let tx2 = tx.clone();
    let handle = thread::spawn(move || search(entry, config, tx2));

    if matches.is_present("list_targets") {
        ls(rx)?;
    } else {
        run(rx, tx)?;
    }
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
            Arg::new("list_targets")
                .short('t')
                .long("list-targets")
                .help("List found targets"),
        )
        .arg(
            Arg::new("list_rules")
                .short('L')
                .long("list-rules")
                .help("List current rules"),
        )
        .arg(
            Arg::new("rule")
                .short('r')
                .long("rule")
                .value_name("RULE")
                .help("Add a search rule")
                .takes_value(true)
                .multiple_occurrences(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Load rules from <FILE>")
                .allow_invalid_utf8(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("entry")
                .allow_invalid_utf8(true)
                .value_name("PATH")
                .help("Start searching from"),
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
        config.load_rules_from_file(&content)?;
        config
    } else {
        Config::default()
    };

    if let Some(values) = matches.values_of("rule") {
        for value in values {
            config.add_rule(value)?;
        }
    }

    if config.rules.is_empty() {
        config.add_default_rules();
    }

    Ok(config)
}

fn set_working_dir(matches: &clap::ArgMatches) -> Result<PathBuf> {
    if let Some(base_directory) = matches.value_of_os("entry") {
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
