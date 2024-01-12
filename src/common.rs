use anyhow::{anyhow, bail, Context, Error, Result};
use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, str::FromStr};

/// storage space unit
static UNITS: [char; 4] = ['T', 'G', 'M', 'K'];

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub rules: Vec<Rule>,
    pub exclude: Vec<String>,
    pub time: Option<(usize, Ordering)>,
    pub size: Option<(u64, Ordering)>,
}

impl Config {
    pub fn is_rule_no_detect(&self, id: &str) -> bool {
        if let Some(rule) = self.rules.iter().find(|rule| rule.id.as_str() == id) {
            rule.no_detect()
        } else {
            false
        }
    }

    pub fn add_rule(&mut self, value: &str) -> Result<()> {
        let rule: Rule = value.parse()?;
        self.rules.push(rule);
        Ok(())
    }

    pub fn set_time(&mut self, time: &str) -> Result<()> {
        let (order, time) = extract_order(time);
        let time: usize = time.parse().map_err(|_| anyhow!("Invalid time value"))?;
        self.time = Some((time, order));
        Ok(())
    }

    pub fn set_size(&mut self, size: &str) -> Result<()> {
        let (order, size) = extract_order(size);
        let size: u64 = parse_size(size).ok_or_else(|| anyhow!("Invalid size value"))?;
        self.size = Some((size, order));
        Ok(())
    }
}

fn extract_order(value: &str) -> (Ordering, &str) {
    if let Some(value) = value.strip_prefix('+') {
        (Ordering::Greater, value)
    } else if let Some(value) = value.strip_prefix('-') {
        (Ordering::Less, value)
    } else {
        (Ordering::Equal, value)
    }
}

fn parse_size(value: &str) -> Option<u64> {
    for (i, ch) in UNITS.into_iter().rev().enumerate() {
        if let Some(value) = value.strip_suffix(ch) {
            let unit = 1024_u64.pow((i + 1) as _);
            let value: f64 = value.parse().ok()?;
            let value = value * (unit as f64);
            return Some(value as u64);
        }
    }
    let value: f64 = value.parse().ok()?;
    Some(value as u64)
}

#[derive(Debug, Clone)]
pub struct Rule {
    id: String,
    targets: HashMap<String, Vec<String>>,
    detects: Vec<glob::Pattern>,
}

impl Rule {
    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn check_target(&self, name: &str) -> Option<&Vec<String>> {
        self.targets.get(name)
    }

    pub fn no_detect(&self) -> bool {
        self.detects.is_empty()
    }

    pub fn check_project(&self, name: &str) -> bool {
        if self.no_detect() {
            false
        } else {
            self.detects.iter().any(|v| v.matches(name))
        }
    }
}

impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (target_paths, detects) = match s.split_once('@') {
            Some((v1, v2)) => (v1.trim().split(',').collect::<Vec<&str>>(), v2.trim()),
            None => (s.split(',').collect(), ""),
        };
        let err_msg = || format!("Invalid rule '{}'", s);
        if target_paths.is_empty() {
            bail!("{}", err_msg())
        }
        let detects = if detects.is_empty() {
            vec![]
        } else {
            detects
                .split(',')
                .map(|v| glob::Pattern::new(v).with_context(err_msg))
                .collect::<Result<_>>()?
        };
        let mut targets: HashMap<String, Vec<String>> = HashMap::new();
        for target in target_paths {
            match target.split_once('/') {
                Some((dir, _)) => {
                    targets
                        .entry(dir.to_string())
                        .or_default()
                        .push(target.to_string());
                }
                None => {
                    targets
                        .entry(target.to_string())
                        .or_default()
                        .push(target.to_string());
                }
            }
        }
        Ok(Rule {
            id: s.to_string(),
            detects,
            targets,
        })
    }
}

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
    pub rule_id: String,
    pub time: Option<Duration>,
    pub time_text: String,
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
    pub fn new(
        path: PathBuf,
        relative_path: PathBuf,
        rule_id: &str,
        time: Option<Duration>,
        size: Option<u64>,
    ) -> Self {
        let size_text = size.map(human_readable_folder_size).unwrap_or_default();
        let time_text = time
            .map(|v| {
                let v = v.as_secs_f64() / 86400.0;
                let v = v.ceil() as u64;
                format!("{v}d")
            })
            .unwrap_or_default();
        PathItem {
            path,
            relative_path,
            rule_id: rule_id.to_string(),
            time,
            time_text,
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
    format!("{size}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule() {
        let rule: Rule = "target".parse().unwrap();
        assert_eq!(rule.no_detect(), true);
        assert_eq!(
            rule.check_target("target"),
            Some(&vec!["target".to_string()])
        );
        assert_eq!(rule.check_target("-target"), None);
        assert_eq!(rule.check_target("target-"), None);
        assert_eq!(rule.check_target("Target"), None);

        let rule: Rule = "Debug,Release@*.sln".parse().unwrap();
        assert_eq!(rule.no_detect(), false);
        assert_eq!(rule.check_target("Debug"), Some(&vec!["Debug".to_string()]));
        assert_eq!(rule.check_target("Debug-"), None);
        assert_eq!(rule.check_target("-Debug"), None);
        assert!(rule.check_project("App.sln"));
    }

    #[test]
    fn test_extract_order() {
        assert_eq!(extract_order("+10"), (Ordering::Greater, "10"));
        assert_eq!(extract_order("10"), (Ordering::Equal, "10"));
        assert_eq!(extract_order("-10"), (Ordering::Less, "10"));
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1K"), Some(1024));
        assert_eq!(parse_size("1M"), Some(1024 * 1024));
        assert_eq!(parse_size("1G"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("1T"), Some(1024 * 1024 * 1024 * 1024));
        assert_eq!(parse_size("1.2M"), Some(1258291));
    }
}
