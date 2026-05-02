use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Paths {
    pub config: PathBuf,
    pub data_dir: PathBuf,
    pub history_dir: PathBuf,
    pub state_file: PathBuf,
    pub git_pushes_csv: PathBuf,
    pub lock_file: PathBuf,
}

impl Paths {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("cannot determine config directory")?
            .join("prodstats");
        let data_dir = dirs::data_dir()
            .context("cannot determine data directory")?
            .join("prodstats");
        Ok(Self {
            config: config_dir.join("config.toml"),
            history_dir: data_dir.join("history"),
            state_file: data_dir.join("state.json"),
            git_pushes_csv: data_dir.join("git-pushes.csv"),
            lock_file: data_dir.join("prodstatsd.lock"),
            data_dir,
        })
    }
}
