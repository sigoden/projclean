use anyhow::{bail, Context, Error, Result};
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Default)]
pub struct Config {
    pub rules: Vec<Rule>,
}

impl Config {
    pub fn is_no_check_rule(&self, id: &str) -> bool {
        if let Some(rule) = self.rules.iter().find(|rule| rule.id.as_str() == id) {
            rule.check.is_none()
        } else {
            false
        }
    }

    pub fn add_rule(&mut self, value: &str) -> Result<()> {
        let rule: Rule = value.parse()?;
        self.rules.push(rule);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Rule {
    id: String,
    purge: HashMap<String, Vec<String>>,
    check: Option<glob::Pattern>,
}

impl Rule {
    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn test_purge(&self, name: &str) -> Option<&Vec<String>> {
        self.purge.get(name)
    }

    pub fn test_check(&self, name: &str) -> bool {
        match self.check.as_ref() {
            Some(check) => check.matches(name),
            None => false,
        }
    }
}

impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (purge_paths, check) = match s.split_once('@') {
            Some((v1, v2)) => (v1.trim().split(',').collect::<Vec<&str>>(), v2.trim()),
            None => (s.split(',').collect(), ""),
        };
        let err_msg = || format!("Invalid rule '{}'", s);
        if purge_paths.is_empty() {
            bail!("{}", err_msg())
        }
        let check = if check.is_empty() {
            None
        } else {
            Some(glob::Pattern::new(check).with_context(err_msg)?)
        };
        let mut purge: HashMap<String, Vec<String>> = HashMap::new();
        for path in purge_paths {
            match path.split_once('/') {
                Some((dir, _)) => {
                    purge
                        .entry(dir.to_string())
                        .or_default()
                        .push(path.to_string());
                }
                None => {
                    purge
                        .entry(path.to_string())
                        .or_default()
                        .push(path.to_string());
                }
            }
        }
        Ok(Rule {
            id: s.to_string(),
            check,
            purge,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule() {
        let rule: Rule = "target".parse().unwrap();
        assert_eq!(rule.test_purge("target"), Some(&vec!["target".to_string()]));
        assert_eq!(rule.test_purge("-target"), None);
        assert_eq!(rule.test_purge("target-"), None);
        assert_eq!(rule.test_purge("Target"), None);

        let rule: Rule = "Debug,Release@*.sln".parse().unwrap();
        assert_eq!(rule.test_purge("Debug"), Some(&vec!["Debug".to_string()]));
        assert_eq!(rule.test_purge("Debug-"), None);
        assert_eq!(rule.test_purge("-Debug"), None);
        assert!(rule.test_check("App.sln"));
    }
}
