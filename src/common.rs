use std::path::PathBuf;

/// storage space unit
static UNITS: [char; 4] = ['T', 'G', 'M', 'K'];

#[derive(Debug)]
pub enum Message {
    AddPath(PathItem),
    SetPathDeleted(PathBuf),
    PutError(String),
    DoneSearch,
}

#[derive(Debug)]
pub struct PathItem {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub size: Option<u64>,
    pub size_text: String,
    pub state: PathState,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PathState {
    Normal,
    StartDeleting,
    Deleted,
}

impl PathItem {
    pub fn new(path: PathBuf, relative_path: PathBuf, size: Option<u64>) -> Self {
        let size_text = match size {
            Some(size) => format!("[{}]", human_readable_folder_size(size)),
            None => "[?]".to_string(),
        };
        PathItem {
            path,
            relative_path,
            size,
            size_text,
            state: PathState::Normal,
        }
    }
}

pub fn human_readable_folder_size(size: u64) -> String {
    if size == 0 {
        return size.to_string();
    }
    for (i, u) in UNITS.iter().enumerate() {
        let num: u64 = 1024;
        let marker = num.pow((UNITS.len() - i) as u32);
        if size >= marker {
            if size / marker < 10 {
                return format!("{:.1}{}", (size as f32 / marker as f32), u);
            } else {
                return format!("{}{}", (size / marker), u);
            }
        }
    }
    format!("{size}B")
}
