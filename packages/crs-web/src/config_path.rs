use errgonomic::handle_opt;
use serde::{Deserialize, Serialize};
use std::env::home_dir;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "PathBuf")]
pub struct ConfigPath(PathBuf);

impl TryFrom<PathBuf> for ConfigPath {
    type Error = TryFromPathBufForConfigPathError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        use TryFromPathBufForConfigPathError::*;
        let Ok(suffix) = path.strip_prefix("~") else {
            return Ok(Self(path));
        };
        let home = handle_opt!(home_dir().filter(|dir| dir.is_absolute()), HomeDirectoryNotFound, path);
        Ok(Self(home.join(suffix)))
    }
}

impl ConfigPath {
    pub fn resolve_relative_to(&mut self, dir: &Path) {
        if self.0.is_relative() {
            self.0 = dir.join(&self.0);
        }
    }
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[derive(Error, Debug)]
pub enum TryFromPathBufForConfigPathError {
    #[error("could not locate the home directory to expand '{}'", path.display())]
    HomeDirectoryNotFound { path: PathBuf },
}
