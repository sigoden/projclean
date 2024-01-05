mod app;
mod common;
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
use fs::{delete_all, ls, search};

use common::{human_readable_folder_size, Config, Message, PathItem, PathState};
use inquire::{formatter::MultiOptionFormatter, MultiSelect};

const RULES: [(&str, &str); 22] = [
    ("node", "node_modules"),
    ("cargo", "target@Cargo.toml"),
    ("maven", "target@pom.xml"),
    ("gradle", ".gradle,build@build.gradle"),
    ("gradle-kts", ".gradle,build@build.gradle.kts"),
    ("cmake", "build@CMakeLists.txt"),
    ("composer", "vendor@composer.json"),
    ("swift", ".build,.swiftpm@Package.swift"),
    ("dart", ".dart_tool,build@pubspec.yaml"),
    ("cocoapods", "Pods@Podfile"),
    ("sbt", "target,project/target@build.sbt"),
    ("zig", "zig-cache,zig-out@build.zig"),
    ("stack", ".stack-work@stack.yaml"),
    ("jupyter", ".ipynb_checkpoints@*.ipynb"),
    ("ocaml", "_build@dune-project"),
    ("elixir", "_build@mix.exs"),
    ("erlang", "_build@rebar.config"),
    ("vs", ".vs,Debug,Release@*.sln"),
    ("vc", "Debug,Release@*.vcxproj"),
    ("c#", "bin,obj@*.csproj"),
    ("f#", "bin,obj@*.fsproj"),
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
    if matches.get_flag("delete-all") {
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
                .short('x')
                .long("exclude")
                .value_name("DIR")
                .value_delimiter(',')
                .action(ArgAction::Append)
                .help("Exclude directories from search, e.g. ignore1,ignore2"),
        )
        .arg(
            Arg::new("time")
                .short('t')
                .long("time")
                .value_name("[+|-]DAY")
                .allow_hyphen_values(true)
                .action(ArgAction::Set)
                .help("Path was last modified less than, more than or exactly <DAY> days"),
        )
        .arg(
            Arg::new("size")
                .short('s')
                .long("size")
                .value_name("[+|-]SIZE")
                .allow_hyphen_values(true)
                .action(ArgAction::Set)
                .help("Path uses less than, more than or exactly <SIZE> units (K|M|G|T) of space"),
        )
        .arg(
            Arg::new("delete-all")
                .short('D')
                .long("delete-all")
                .action(ArgAction::SetTrue)
                .help("Automatically delete all found targets"),
        )
        .arg(
            Arg::new("print")
                .short('P')
                .long("print")
                .action(ArgAction::SetTrue)
                .help("Print the found targets"),
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

    config.exclude = matches
        .get_many::<String>("exclude")
        .map(|v| v.cloned().collect())
        .unwrap_or_default();

    if let Some(time) = matches.get_one::<String>("time") {
        config.set_time(time)?;
    }

    if let Some(size) = matches.get_one::<String>("size") {
        config.set_size(size)?;
    }

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

    let min_height = 3;
    let height = crossterm::terminal::size()
        .map(|(_, height)| height as usize)
        .unwrap_or(min_height + 1);

    let selections = MultiSelect::new("Select search rules:", options.clone())
        .with_formatter(formatter)
        .with_page_size(height - min_height)
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
