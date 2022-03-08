use anyhow::{anyhow, bail, Error, Result};
use cli_table::{
    format::{Border, Separator},
    print_stdout, Cell, Style, Table,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    str::FromStr,
};

const BUILTIN_PROJECTS: &str = include_str!("builtin.csv");

#[derive(Debug, Default)]
pub struct Config {
    projects: Vec<Project>,
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
    pub fn test_path<'a, 'b>(
        &'a self,
        matches: &mut HashMap<&'a Project, (HashSet<&'b str>, HashSet<&'b str>)>,
        name: &'b str,
    ) {
        for project in &self.projects {
            let (purge_matches, check_matches) =
                matches.entry(project).or_insert(Default::default());
            if project.purge.as_str() == name {
                purge_matches.insert(name);
            }
            if let Some(check) = project.check.as_ref() {
                if check.as_str() == name {
                    check_matches.insert(name);
                }
            }
        }
    }

    pub fn is_empty_projects(&self) -> bool {
        self.projects.is_empty()
    }

    pub fn add_builtin(&mut self) -> Result<()> {
        self.read_file(BUILTIN_PROJECTS)
    }

    pub fn add_project(&mut self, value: &str) -> Result<()> {
        let project: Project = value.parse()?;
        self.projects.push(project);
        Ok(())
    }

    pub fn export_projects(&self) -> Result<()> {
        for project in &self.projects {
            println!("{}", project);
        }
        Ok(())
    }

    pub fn list_projects(&self) -> Result<()> {
        let datagrid = self.projects.iter().map(|project| {
            vec![
                project.purge.clone().cell(),
                project.check.clone().unwrap_or_default().cell(),
                project.name.clone().unwrap_or_default().cell(),
            ]
        });
        let table = datagrid
            .table()
            .border(Border::builder().build())
            .title(vec![
                "To Purge".cell().bold(true),
                "For check".cell().bold(true),
                "Project Name".cell().bold(true),
            ])
            .separator(Separator::builder().build())
            .bold(true);

        print_stdout(table)?;
        Ok(())
    }

    pub fn read_file(&mut self, content: &str) -> Result<()> {
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
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Project {
    pub purge: String,
    pub check: Option<String>,
    pub name: Option<String>,
}

impl FromStr for Project {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        let (purge, check, name) = match parts.len() {
            1 => (parts[0].trim(), "", ""),
            2 => (parts[0].trim(), parts[1].trim(), ""),
            3 => (parts[0].trim(), parts[1].trim(), parts[2].trim()),
            _ => ("", "", ""),
        };
        if purge.is_empty() {
            bail!("Invalid project value '{}'", s);
        }
        Ok(Project {
            purge: purge.to_string(),
            check: if check.is_empty() {
                None
            } else {
                Some(check.to_string())
            },
            name: if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            },
        })
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.purge)?;
        match (&self.check, &self.name) {
            (None, None) => {}
            (None, Some(name)) => write!(f, ",,{}", name)?,
            (Some(check), None) => write!(f, ",{}", check)?,
            (Some(check), Some(name)) => write!(f, ",{},{}", check, name)?,
        }
        Ok(())
    }
}
