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

use app::run;
use config::Config;
use fs::{delete_all, ls, search};

use common::{human_readable_folder_size, Message, PathItem, PathState};
use inquire::{formatter::MultiOptionFormatter, MultiSelect};

const RULES: [(&str, &str); 21] = [
    ("node", "node_modules"),
    ("cargo", "target@Cargo.toml"),
    ("maven", "target@pom.xml"),
    ("gradle", ".gradle,build@build.gradle"),
    (
        "python",
        "__pycache__,.mypy_cache,.pytest_cache,.ruff_cache,.tox@*.py",
    ),
    ("composer", "vendor@composer.json"),
    ("swift", ".build,.swiftpm@Package.swift"),
    (
        "dart",
        ".dart_tool,build,linux/flutter/ephemeral,windows/flutter/ephemeral@pubspec.yaml",
    ),
    ("sbt", "target,project/target@build.sbt"),
    ("zig", "zig-cache,zig-out@build.zig"),
    ("stack", ".stack-work@stack.yaml"),
    ("jupyter", ".ipynb_checkpoints@*.ipynb"),
    ("ocaml", "_build@dune-project"),
    ("elixir", "_build@mix.exs"),
    ("erlang", "_build@rebar.config"),
    ("vc", "Debug,Release@*.vcxproj"),
    ("c#", "bin,obj@*.csproj"),
    ("f#", "bin,obj@*.fsproj"),
    (
        "unity",
        "Library,Temp,Obj,Logs,MemoryCaptures,Build,Builds@Assembly-CSharp.csproj",
    ),
    (
        "unreal",
        "Binaries,Build,Saved,DerivedDataCache,Intermediate@*.uproject",
    ),
    ("godot", ".godot@project.godot"),
];

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let running_cloned = running.clone();
    ctrlc::set_handler(move || {
        running_cloned.store(false, Ordering::SeqCst);
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
        delete_all(rx)?;
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
        .arg(
            Arg::new("cwd")
                .short('C')
                .long("cwd")
                .value_name("DIR")
                .default_value(".")
                .action(ArgAction::Set)
                .help("Start searching from <DIR>"),
        )
        .arg(
            Arg::new("exclude")
                .short('e')
                .long("exclude")
                .value_name("DIR")
                .value_delimiter(',')
                .action(ArgAction::Append)
                .help("Exclude directories from search. e.g. ignore1,ignore2"),
        )
        .arg(
            Arg::new("print")
                .short('p')
                .long("print")
                .action(ArgAction::SetTrue)
                .help("Print the found targets, do not enter TUI"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Forcefully delete the found targets, do not enter TUI"),
        )
        .arg(
            Arg::new("rules")
                .help("Search rules, e.g. node_modules target@Cargo.toml")
                .value_name("RULES")
                .action(ArgAction::Append),
        )
}

fn init_config(matches: &clap::ArgMatches) -> Result<Config> {
    let mut config = Config::default();

    let rules = if let Some(values) = matches.get_many::<String>("rules") {
        values.cloned().collect()
    } else {
        select_rules()?
    };

    config.excludes = matches
        .get_many::<String>("exclude")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

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
        .map(|(name, rule)| format!("{name:<16}{rule}"))
        .to_vec();

    let to_rules = |selections: &[String]| {
        selections
            .iter()
            .map(|sel| {
                options
                    .iter()
                    .enumerate()
                    .find(|(_, v)| sel == *v)
                    .map(|(i, _)| RULES[i].1.to_string())
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
        .with_page_size(10)
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
