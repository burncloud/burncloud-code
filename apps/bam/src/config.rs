use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const DEFAULT_CONFIG: &str = r#"[github]
owner = "burncloud"
repository = "burncloud"
project_number = 1

[manager]
poll_interval_seconds = 60
max_concurrent_agents = 2
max_repair_attempts_per_error = 3
max_total_attempts_per_task = 12

[storage]
database_path = ".bam/state.sqlite"

[merge]
auto_merge = false

[local]
compile_allowed = false
tests_allowed = false
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub github: Github,
    pub manager: Manager,
    pub storage: Storage,
    pub merge: Merge,
    pub local: Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Github {
    pub owner: String,
    pub repository: String,
    pub project_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manager {
    pub poll_interval_seconds: u64,
    pub max_concurrent_agents: u8,
    pub max_repair_attempts_per_error: u32,
    pub max_total_attempts_per_task: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub database_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Merge {
    pub auto_merge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Local {
    pub compile_allowed: bool,
    pub tests_allowed: bool,
}

impl Config {
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: Self = toml::from_str(text).map_err(|e| format!("invalid TOML: {e}"))?;
        value.validate()?;
        Ok(value)
    }

    pub fn from_path(path: &Path) -> Result<Self, String> {
        let text =
            fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::parse(&text)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.github.owner.trim().is_empty()
            || self.github.repository.trim().is_empty()
            || self.github.project_number == 0
        {
            return Err("GitHub owner/repository/project_number must be set".into());
        }
        if self.manager.poll_interval_seconds == 0
            || !(1..=8).contains(&self.manager.max_concurrent_agents)
        {
            return Err("poll interval must be positive, agents must be between 1 and 8".into());
        }
        if self.manager.max_repair_attempts_per_error == 0
            || self.manager.max_total_attempts_per_task < self.manager.max_repair_attempts_per_error {
            return Err("repair budgets must be positive and internally consistent".into());
        }
        if self.storage.database_path.trim().is_empty() {
            return Err("database path must not be empty".into());
        }
        if self.merge.auto_merge {
            return Err("BAM P0: automatic merging is forbidden".into());
        }
        if self.local.compile_allowed || self.local.tests_allowed {
            return Err("BAM P0: local compilation and testing cannot be enabled".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_config_is_valid() {
        Config::parse(DEFAULT_CONFIG).expect("default configuration");
    }
    #[test]
    fn local_tests_cannot_be_enabled() {
        let text = DEFAULT_CONFIG.replace("tests_allowed = false", "tests_allowed = true");
        assert!(Config::parse(&text).is_err());
    }
    #[test]
    fn compilation_cannot_be_enabled() {
        let text = DEFAULT_CONFIG.replace("compile_allowed = false", "compile_allowed = true");
        assert!(Config::parse(&text).is_err());
    }
    #[test]
    fn merge_cannot_be_enabled() {
        let text = DEFAULT_CONFIG.replace("auto_merge = false", "auto_merge = true");
        assert!(Config::parse(&text).is_err());
    }
    #[test]
    fn unknown_configuration_is_rejected() {
        assert!(Config::parse(&format!("{DEFAULT_CONFIG}\nunknown = true\n")).is_err());
    }
}
