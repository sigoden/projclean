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
use clap::{Arg, ArgAction, Command};
use crossbeam_utils::sync::WaitGroup;

use app::run;
use config::Config;
use fs::{delete_all, ls, search};

use common::{human_readable_folder_size, Message, PathItem, PathState};
use inquire::{formatter::MultiOptionFormatter, MultiSelect};

const RULES: [[&str; 2]; 7] = [
    ["node_modules", "js"],
    ["target@Cargo.toml", "rust"],
    ["^(Debug|Release)$@\\.sln$", "vs"],
    ["^(build|xcuserdata|DerivedData)$@Podfile", "ios"],
    ["build@build.gradle", "android"],
    ["target@pom.xml", "java"],
    ["vendor@composer.json", "php"],
];

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let running2 = running.clone();
    ctrlc::set_handler(move || {
        running2.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if let Err(err) = start(running) {
        eprintln!("{err}");
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
    if matches.get_flag("force") {
        let wg = WaitGroup::new();
        delete_all(rx, wg.clone())?;
        wg.wait();
    } else if matches.get_flag("print") {
        ls(rx)?;
    } else {
        run(rx, tx)?;
    }
    Ok(())
}

fn command() -> Command {
    Command::new(env!("CARGO_CRATE_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(concat!(
            env!("CARGO_PKG_DESCRIPTION"),
            " - ",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .arg(
            Arg::new("cwd")
                .short('C')
                .long("cwd")
                .value_name("DIR")
                .default_value(".")
                .action(ArgAction::Set)
                .help("Start searching from DIR"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Delete found targets without entering tui"),
        )
        .arg(
            Arg::new("print")
                .short('p')
                .long("print")
                .action(ArgAction::SetTrue)
                .help("Print found targets only"),
        )
        .arg(
            Arg::new("rules")
                .help("Search rules, like node_modules or target@Cargo.toml")
                .value_name("RULES")
                .action(ArgAction::Append),
        )
}

fn init_config(matches: &clap::ArgMatches) -> Result<Config> {
    let mut config = Config::default();

    let rules = if let Some(values) = matches.get_many::<String>("rules") {
        values.map(|v| v.to_string()).collect()
    } else {
        select_rules()?
    };
    for rule in rules {
        config.add_rule(&rule)?;
    }

    Ok(config)
}

fn set_working_dir(matches: &clap::ArgMatches) -> Result<PathBuf> {
    if let Some(current_dir) = matches.get_one::<String>("cwd") {
        let current_dir = Path::new(current_dir);

        if !is_existing_directory(current_dir) {
            return Err(anyhow!(
                "The '--file' path '{}' is not a directory.",
                current_dir.to_string_lossy()
            ));
        }
        let base_directory = canonicalize(current_dir).unwrap();
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

fn select_rules() -> Result<Vec<String>> {
    let options = RULES
        .map(|[rule, name]| format!("{name:<16}{rule}"))
        .to_vec();

    let to_rules = |selections: &[String]| {
        selections
            .iter()
            .map(|sel| {
                options
                    .iter()
                    .enumerate()
                    .find(|(_, v)| sel == *v)
                    .map(|(i, _)| RULES[i][0].to_string())
                    .unwrap()
            })
            .collect::<Vec<String>>()
    };

    let formatter: MultiOptionFormatter<String> = &|a| {
        to_rules(
            &a.iter()
                .map(|v| v.value.to_string())
                .collect::<Vec<String>>(),
        )
        .join(" ")
    };
    let selections = MultiSelect::new("Select search rules:", options.clone())
        .with_formatter(formatter)
        .without_help_message()
        .prompt()
        .unwrap_or_default();

    if selections.is_empty() {
        bail!("You did not select any rule :(")
    }
    Ok(to_rules(&selections))
}

fn is_existing_directory(path: &Path) -> bool {
    path.is_dir() && path.exists()
}
