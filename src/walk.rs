use anyhow::Result;
use jwalk::WalkDirGeneric;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::Config;

#[derive(Debug)]
pub struct FindItem {
    /// Project kind
    pub kind: String,
    /// Path
    pub path: PathBuf,
}

pub fn scan(entry: &Path, config: Arc<Config>) -> Result<Vec<FindItem>> {
    let walk_dir = WalkDirGeneric::<((), Option<String>)>::new(entry).process_read_dir(
        move |_depth, _path, _state, children| {
            let mut projects = vec![];
            for dir_entry_result in children.iter() {
                if let Ok(dir_entry) = dir_entry_result {
                    if let Some(name) = dir_entry.file_name.to_str() {
                        if let Some(project) = config.find_project(name) {
                            projects.push(project);
                        }
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
    let mut output = vec![];
    for entry in walk_dir {
        if let Ok(dir_entry) = &entry {
            if let Some(name) = dir_entry.client_state.as_ref() {
                output.push(FindItem {
                    kind: name.to_string(),
                    path: dir_entry.path(),
                })
            }
        }
    }

    Ok(output)
}
