use std::env::current_dir;
use std::io;
use std::path::PathBuf;

pub fn unwrap_or_current_dir(path: Option<PathBuf>) -> io::Result<PathBuf> {
    path.map_or_else(current_dir, Ok)
}
