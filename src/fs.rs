use anyhow::Result;
use jwalk::WalkDirGeneric;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::config::Project;
use crate::{Config, Message, PathItem};

pub fn search(entry: PathBuf, config: Config, tx: Sender<Message>) -> Result<()> {
    let walk_dir = WalkDirGeneric::<((), Option<Option<String>>)>::new(entry.clone())
        .process_read_dir(move |_depth, _path, _state, children| {
            let mut matches: HashMap<&Project, (HashSet<&str>, HashSet<&str>)> = HashMap::new();
            for dir_entry in children.iter().flatten() {
                if let Some(name) = dir_entry.file_name.to_str() {
                    config.test_path(&mut matches, name);
                }
            }
            let mut matched_children: HashMap<String, &Project> = HashMap::new();
            for (project, (purge_matches, check_matches)) in matches {
                if !purge_matches.is_empty()
                    && (!check_matches.is_empty() || project.check.is_none())
                {
                    for name in purge_matches {
                        matched_children.insert(name.to_string(), project);
                    }
                }
            }
            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if let Some(name) = dir_entry.file_name.to_str() {
                        if let Some(project) = matched_children.get(name) {
                            dir_entry.read_children_path = None;
                            dir_entry.client_state = Some(project.name.clone());
                        }
                    }
                }
            });
        });

    for dir_entry_result in walk_dir {
        if let Ok(dir_entry) = &dir_entry_result {
            if let Some(kind) = dir_entry.client_state.as_ref() {
                let path = dir_entry.path();
                let size = du(&path).ok();
                let relative_path = path.strip_prefix(&entry)?.to_path_buf();
                tx.send(Message::AddPath(PathItem::new(
                    path,
                    relative_path,
                    size,
                    kind.clone(),
                )))?;
            }
        }
    }

    tx.send(Message::DoneSearch)?;

    Ok(())
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
