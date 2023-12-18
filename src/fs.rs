use anyhow::Result;
use crossbeam_utils::sync::WaitGroup;
use jwalk::WalkDirGeneric;
use remove_dir_all::remove_dir_all;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use threadpool::ThreadPool;

use crate::{Config, Message, PathItem};

pub fn search(
    entry: PathBuf,
    config: Config,
    tx: Sender<Message>,
    running: Arc<AtomicBool>,
) -> Result<()> {
    let walk_dir = WalkDirGeneric::<((), Option<(String, Vec<String>)>)>::new(entry.clone())
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
                        if let Some((rule_id, purges)) = matches.get(name) {
                            dir_entry.read_children_path = None;
                            dir_entry.client_state = Some((rule_id.to_string(), purges.to_vec()));
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
            if let Some((_, purges)) = dir_entry.client_state.as_ref() {
                let entry_path = dir_entry.path();
                for purge in purges {
                    let mut path = entry_path.clone();
                    for part in purge.split('/').skip(1) {
                        path.push(part)
                    }
                    if !path.exists() {
                        continue;
                    }
                    let size = du(&path).ok();
                    let relative_path = path.strip_prefix(&entry)?.to_path_buf();
                    let _ = tx.send(Message::AddPath(PathItem::new(path, relative_path, size)));
                }
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

pub fn delete_all(rx: Receiver<Message>) -> Result<()> {
    let wg = WaitGroup::new();
    let pool = ThreadPool::default();
    for message in rx {
        match message {
            Message::AddPath(path) => {
                spawn_delete_path(pool.clone(), path.path.clone(), wg.clone());
            }
            Message::DoneSearch => break,
            _ => {}
        }
    }
    wg.wait();
    Ok(())
}

fn spawn_delete_path(pool: ThreadPool, path: PathBuf, wg: WaitGroup) {
    pool.execute(move || {
        match remove_dir_all(&path) {
            Ok(_) => println!("Delete {}", path.display()),
            Err(err) => eprintln!("Failed to delete {}, {}", path.display(), err),
        }
        drop(wg);
    });
}

#[derive(Debug)]
struct Checker<'a, 'b> {
    matches: HashMap<&'a str, CheckMatches<'a, 'b>>,
    config: &'a Config,
}

#[derive(Debug, Default)]
struct CheckMatches<'a, 'b> {
    purge: HashMap<&'b str, &'a Vec<String>>,
    check: HashSet<&'b str>,
}

impl<'a, 'b> Checker<'a, 'b> {
    fn new(config: &'a Config) -> Self {
        Self {
            config,
            matches: Default::default(),
        }
    }

    fn check(&mut self, name: &'b str) {
        if self.config.excludes.contains(&name.to_string()) {
            return;
        }
        for rule in &self.config.rules {
            let matches = self.matches.entry(rule.get_id()).or_default();
            if let Some(purges) = rule.test_purge(name) {
                matches.purge.insert(name, purges.as_ref());
            }
            if rule.test_check(name) {
                matches.check.insert(name);
            }
        }
    }

    fn to_matches(&self) -> HashMap<String, (&'a str, &'a Vec<String>)> {
        let mut output = HashMap::new();
        for (rule_id, matches) in &self.matches {
            if !matches.purge.is_empty()
                && (!matches.check.is_empty() || self.config.is_no_check_rule(rule_id))
            {
                for (name, purges) in &matches.purge {
                    if !output.contains_key(*name) {
                        output.insert(name.to_string(), (*rule_id, *purges));
                    }
                }
            }
        }
        output
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
        assert_match_paths!("target@Cargo.toml", &["target", "Cargo.toml"], &["target"]);
        assert_match_paths!("target@Cargo.toml", &["target.rs", "Cargo.toml"]);
        assert_match_paths!("Debug,Release@*.sln", &["Debug", "Demo.sln"], &["Debug"]);
    }
}
