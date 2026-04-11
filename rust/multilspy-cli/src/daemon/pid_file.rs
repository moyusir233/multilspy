use std::fs;
use std::path::PathBuf;
use super::super::error::CliError;

pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn read(&self) -> Result<u32, CliError> {
        let content = fs::read_to_string(&self.path)?;
        let pid = content.trim().parse()?;
        Ok(pid)
    }

    pub fn write(&self, pid: u32) -> Result<(), CliError> {
        fs::write(&self.path, pid.to_string())?;
        Ok(())
    }

    pub fn remove(&self) -> Result<(), CliError> {
        if self.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}
