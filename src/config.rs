use anyhow::{anyhow, Error, Result};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

const BUILTIN_PROJECTS: &str = include_str!("default.csv");

#[derive(Debug, Default)]
pub struct Config {
    pub projects: Vec<Project>,
}

impl Config {
    pub fn find_project(&self, name: &str) -> Option<&Project> {
        self.projects.iter().find(|project| {
            project
                .check
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
        for project in &self.projects {
            let (purge_matches, check_matches) = matches.entry(&project.id).or_default();
            if project.test_purge(name) {
                purge_matches.insert(name);
            }
            if project.test_check(name) {
                check_matches.insert(name);
            }
        }
    }

    pub fn is_empty_projects(&self) -> bool {
        self.projects.is_empty()
    }

    pub fn is_project_no_check(&self, id: &str) -> bool {
        if let Some(project) = self
            .projects
            .iter()
            .find(|project| project.id.as_str() == id)
        {
            project.check.is_none()
        } else {
            false
        }
    }

    pub fn get_project_name(&self, id: &str) -> Option<String> {
        if let Some(project) = self
            .projects
            .iter()
            .find(|project| project.id.as_str() == id)
        {
            project.name.clone()
        } else {
            None
        }
    }

    pub fn add_default_projects(&mut self) -> Result<()> {
        self.add_projects_from_file(BUILTIN_PROJECTS)
    }

    pub fn add_projects_from_file(&mut self, content: &str) -> Result<()> {
        for (index, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            self.add_project(line)
                .map_err(|_| anyhow!("Invalid project value '{}' at line {}", line, index + 1))?;
        }
        Ok(())
    }

    pub fn add_project(&mut self, value: &str) -> Result<()> {
        let project: Project = value.parse()?;
        self.projects.push(project);
        Ok(())
    }

    pub fn list_projects(&self) -> Result<()> {
        for project in &self.projects {
            println!("{}", project.id);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Project {
    id: String,
    purge: Regex,
    check: Option<Regex>,
    name: Option<String>,
}

impl Project {
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

impl FromStr for Project {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(';').collect();
        let (purge, check, name) = match parts.len() {
            1 => (parts[0].trim(), "", ""),
            2 => (parts[0].trim(), parts[1].trim(), ""),
            3 => (parts[0].trim(), parts[1].trim(), parts[2].trim()),
            _ => ("", "", ""),
        };
        let err = || anyhow!("Invalid project value '{}'", s);
        if purge.is_empty() {
            return Err(err());
        }
        Ok(Project {
            id: s.to_string(),
            purge: Regex::new(purge).map_err(|_| err())?,
            check: if check.is_empty() {
                None
            } else {
                let check = Regex::new(check).map_err(|_| err())?;
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
