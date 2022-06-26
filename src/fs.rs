use anyhow::Result;
use jwalk::WalkDirGeneric;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use crate::{Config, Message, PathItem};

pub fn search(
    entry: PathBuf,
    config: Config,
    tx: Sender<Message>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let walk_dir = WalkDirGeneric::<((), Option<()>)>::new(entry.clone())
        .skip_hidden(false)
        .process_read_dir(move |_depth, _path, _state, children| {
            let mut checker = Checker::new(&config);
            for dir_entry in children.iter().flatten() {
                if let Some(name) = dir_entry.file_name.to_str() {
                    checker.check(name);
                }
            }
            let matches = checker.to_matches();
            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if let Some(name) = dir_entry.file_name.to_str() {
                        if matches.get(name).is_some() {
                            dir_entry.read_children_path = None;
                            dir_entry.client_state = Some(());
                        }
                    }
                }
            });
        });

    for dir_entry_result in walk_dir {
        if !running.load(Ordering::SeqCst) {
            let _ = tx.send(Message::DoneSearch);
            return Ok(());
        }
        if let Ok(dir_entry) = &dir_entry_result {
            if let Some(()) = dir_entry.client_state.as_ref() {
                let path = dir_entry.path();
                let size = du(&path).ok();
                let relative_path = path.strip_prefix(&entry)?.to_path_buf();
                let _ = tx.send(Message::AddPath(PathItem::new(path, relative_path, size)));
            }
        }
    }

    let _ = tx.send(Message::DoneSearch);

    Ok(())
}

pub fn ls(rx: Receiver<Message>) -> Result<()> {
    for message in rx {
        match message {
            Message::AddPath(path) => {
                println!("{}", path.path.display());
            }
            Message::DoneSearch => break,
            _ => {}
        }
    }
    Ok(())
}

#[derive(Debug)]
struct Checker<'a, 'b> {
    matches: HashMap<&'a str, (HashSet<&'b str>, HashSet<&'b str>)>,
    config: &'a Config,
}

impl<'a, 'b> Checker<'a, 'b> {
    fn new(config: &'a Config) -> Self {
        Self {
            config,
            matches: Default::default(),
        }
    }

    fn check(&mut self, name: &'b str) {
        for rule in &self.config.rules {
            let (purge_matches, check_matches) = self.matches.entry(rule.get_id()).or_default();
            if rule.test_purge(name) {
                purge_matches.insert(name);
            }
            if rule.test_check(name) {
                check_matches.insert(name);
            }
        }
    }

    fn to_matches(&self) -> HashMap<String, &'a str> {
        let mut matches: HashMap<String, &'a str> = HashMap::new();
        for (rule_id, (purge_matches, check_matches)) in &self.matches {
            if !purge_matches.is_empty()
                && (!check_matches.is_empty() || self.config.is_rule_no_check(rule_id))
            {
                for name in purge_matches {
                    if !matches.contains_key(*name) {
                        matches.insert(name.to_string(), rule_id);
                    }
                }
            }
        }
        matches
    }
}

fn du(path: &Path) -> Result<u64> {
    let mut total: u64 = 0;

    for dir_entry_result in WalkDirGeneric::<((), Option<u64>)>::new(path)
        .skip_hidden(false)
        .process_read_dir(|_, _, _, dir_entry_results| {
            dir_entry_results.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if !dir_entry.file_type.is_dir() {
                        dir_entry.client_state =
                            Some(dir_entry.metadata().map(|m| m.len()).unwrap_or_default());
                    }
                }
            })
        })
    {
        let dir_entry = dir_entry_result?;
        if let Some(len) = &dir_entry.client_state {
            total += len;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! assert_match_paths {
        ($id:literal, $names:expr) => {
            let none: &[&str] = &[];
            assert_match_paths!($id, $names, none);
        };
        ($id:literal, $names:expr, $matched:expr) => {
            let mut config = Config::default();
            let ret = config.add_rule($id);
            assert!(ret.is_ok());
            let mut checker = Checker::new(&config);
            for name in $names {
                checker.check(name);
            }
            let matches = checker.to_matches();
            let matched_names: Vec<&str> = matches.keys().map(|v| v.as_str()).collect();
            assert_eq!(matched_names, $matched);
        };
    }

    #[test]
    fn test_match_paths() {
        assert_match_paths!(
            "^target$@Cargo.toml",
            &["target", "Cargo.toml"],
            &["target"]
        );
        assert_match_paths!("target@Cargo.toml", &["target.rs", "Cargo.toml"]);
        assert_match_paths!(
            "^(Debug|Release)$@.*\\.sln",
            &["Debug", "Demo.sln"],
            &["Debug"]
        );
    }
}
