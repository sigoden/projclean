use std::path::PathBuf;

/// storage space unit
static UNITS: [char; 4] = ['T', 'G', 'M', 'K'];
/// limit kind string to 16 chars
const KIND_LIMIT_WIDTH: usize = 12;

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
    pub kind_text: String,
    pub state: PathState,
}

#[derive(Debug, PartialEq)]
pub enum PathState {
    Normal,
    StartDeleting,
    Deleted,
}

impl PathItem {
    pub fn new(
        path: PathBuf,
        relative_path: PathBuf,
        size: Option<u64>,
        kind: Option<String>,
    ) -> Self {
        let size_text = match size {
            Some(size) => format!("[{}]", human_readable_folder_size(size)),
            None => "[?]".to_string(),
        };
        let kind_text = match kind.as_ref() {
            Some(kind) => format!("({})", truncate_kind(kind)),
            None => "".to_string(),
        };
        PathItem {
            path,
            relative_path,
            size,
            size_text,
            kind_text,
            state: PathState::Normal,
        }
    }
}

fn truncate_kind(kind: &str) -> String {
    if kind.len() <= KIND_LIMIT_WIDTH {
        kind.to_string()
    } else {
        kind[0..KIND_LIMIT_WIDTH].to_string()
    }
}

pub fn human_readable_folder_size(size: u64) -> String {
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
    return format!("{}B", size);
}
