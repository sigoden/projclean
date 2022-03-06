use anyhow::{anyhow, Result};
use serde::Deserialize;

const DEFAULT_CONFIG: &str = include_str!("default.yaml");

#[derive(Debug, Deserialize)]
pub struct Config {
    pub projects: Vec<Project>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config: Config = serde_yaml::from_str(DEFAULT_CONFIG)
            .map_err(|e| anyhow!("Fail to load default config, {}", e))?;
        Ok(config)
    }
    pub fn find_project(&self, name: &str) -> Option<&Project> {
        self.projects.iter().find(|v| v.exist.as_str() == name)
    }
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub name: String,
    pub exist: String,
    pub purge: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let config = Config::load();
        assert!(config.is_ok());
    }
}
