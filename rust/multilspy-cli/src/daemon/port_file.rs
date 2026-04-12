use std::fs;
use std::path::PathBuf;
use super::super::error::CliError;

pub struct PortFile {
    path: PathBuf,
}

impl PortFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn read(&self) -> Result<u16, CliError> {
        let content = fs::read_to_string(&self.path)?;
        let port = content.trim().parse()?;
        Ok(port)
    }

    pub fn write(&self, port: u16) -> Result<(), CliError> {
        fs::write(&self.path, port.to_string())?;
        Ok(())
    }

    pub fn remove(&self) -> Result<(), CliError> {
        if self.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}
