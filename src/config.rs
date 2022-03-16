use anyhow::{anyhow, Error, Result};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

const BUILTIN_RULES: &str = include_str!("default.csv");

#[derive(Debug, Default)]
pub struct Config {
    pub rules: Vec<Rule>,
}

impl Config {
    pub fn find_rule(&self, name: &str) -> Option<&Rule> {
        self.rules.iter().find(|rule| {
            rule.check
                .as_ref()
                .map(|check| check.as_str() == name)
                .unwrap_or(true)
        })
    }
    pub fn match_patch<'a, 'b>(
        &'a self,
        matches: &mut HashMap<&'a str, (HashSet<&'b str>, HashSet<&'b str>)>,
        name: &'b str,
    ) {
        for rule in &self.rules {
            let (purge_matches, check_matches) = matches.entry(&rule.id).or_default();
            if rule.test_purge(name) {
                purge_matches.insert(name);
            }
            if rule.test_check(name) {
                check_matches.insert(name);
            }
        }
    }

    pub fn is_empty_rules(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn is_rule_no_check(&self, id: &str) -> bool {
        if let Some(rule) = self.rules.iter().find(|rule| rule.id.as_str() == id) {
            rule.check.is_none()
        } else {
            false
        }
    }

    pub fn get_rule_name(&self, id: &str) -> Option<String> {
        if let Some(rule) = self.rules.iter().find(|rule| rule.id.as_str() == id) {
            rule.name.clone()
        } else {
            None
        }
    }

    pub fn add_default_rules(&mut self) {
        self.load_rules_from_file(BUILTIN_RULES)
            .expect("broken builtin config file");
    }

    pub fn load_rules_from_file(&mut self, content: &str) -> Result<()> {
        for (index, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            self.add_rule(line)
                .map_err(|_| anyhow!("Invalid rule '{}' at line {}", line, index + 1))?;
        }
        Ok(())
    }

    pub fn add_rule(&mut self, value: &str) -> Result<()> {
        let rule: Rule = value.parse()?;
        self.rules.push(rule);
        Ok(())
    }

    pub fn list_rules(&self) -> Result<()> {
        for rule in &self.rules {
            println!("{}", rule.id);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Rule {
    id: String,
    purge: Regex,
    check: Option<Regex>,
    name: Option<String>,
}

impl Rule {
    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn test_purge(&self, name: &str) -> bool {
        self.purge.is_match(name)
    }

    pub fn test_check(&self, name: &str) -> bool {
        match self.check.as_ref() {
            Some(check) => check.is_match(name),
            None => false,
        }
    }
}

impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(';').collect();
        let (purge, check, name) = match parts.len() {
            1 => (parts[0].trim(), "", ""),
            2 => (parts[0].trim(), parts[1].trim(), ""),
            3 => (parts[0].trim(), parts[1].trim(), parts[2].trim()),
            _ => ("", "", ""),
        };
        let err = || anyhow!("Invalid rule '{}'", s);
        if purge.is_empty() {
            return Err(err());
        }
        Ok(Rule {
            id: s.to_string(),
            purge: to_regex(purge).map_err(|_| err())?,
            check: if check.is_empty() {
                None
            } else {
                let check = to_regex(check).map_err(|_| err())?;
                Some(check)
            },
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
        })
    }
}

fn to_regex(value: &str) -> Result<Regex> {
    let re = if value
        .chars()
        .all(|v| v.is_alphanumeric() || v == '.' || v == '-' || v == '_')
    {
        format!("^{}$", value.replace('.', "\\."))
    } else {
        value.to_string()
    };
    Regex::new(&re).map_err(|_| anyhow!("Invalid regex value '{}'", value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule() {
        let rule: Rule = "target".parse().unwrap();
        assert!(rule.test_purge("target"));
        assert!(!rule.test_purge("-target"));
        assert!(!rule.test_purge("target-"));
        assert!(!rule.test_purge("Target"));

        let rule: Rule = "^(Debug|Release)$;\\.sln$".parse().unwrap();
        assert!(rule.test_purge("Debug"));
        assert!(!rule.test_purge("Debug-"));
        assert!(!rule.test_purge("-Debug"));
        assert!(rule.test_check("App.sln"));
    }
}
