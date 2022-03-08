use anyhow::Result;
use jwalk::WalkDirGeneric;
use std::path::Path;
use std::sync::{mpsc::Sender, Arc};

use crate::{Config, Message, PathItem};

pub fn search(entry: &Path, config: Arc<Config>, tx: Sender<Message>) -> Result<()> {
    let walk_dir = WalkDirGeneric::<((), Option<String>)>::new(entry).process_read_dir(
        move |_depth, _path, _state, children| {
            let mut projects = vec![];
            for dir_entry in children.iter().flatten() {
                if let Some(name) = dir_entry.file_name.to_str() {
                    if let Some(project) = config.find_project(name) {
                        projects.push(project);
                    }
                }
            }
            children.retain(|dir_entry_result| {
                dir_entry_result
                    .as_ref()
                    .map(|dir_entry| dir_entry.file_type.is_dir())
                    .unwrap_or(false)
            });
            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if let Some(name) = dir_entry.file_name.to_str() {
                        for project in projects.iter() {
                            if project.purge.as_str() == name {
                                dir_entry.read_children_path = None;
                                dir_entry.client_state = Some(project.name.to_string());
                            }
                        }
                    }
                }
            });
        },
    );
    for entry in walk_dir {
        if let Ok(dir_entry) = &entry {
            if let Some(kind) = dir_entry.client_state.as_ref() {
                let path = dir_entry.path();
                let size = du(&path).ok();
                tx.send(Message::AddPath(PathItem::new(kind, &path, size)))?;
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
