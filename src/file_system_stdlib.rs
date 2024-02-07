use notify::{RecursiveMode, Watcher};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use tracing::warn;
use walkdir::WalkDir;

use crate::ArchivalError;

use super::{FileSystemAPI, WatchableFileSystemAPI};

pub struct NativeFileSystem {
    pub root: PathBuf,
}

impl NativeFileSystem {
    pub fn new(root: &Path) -> Self {
        Self {
            root: Path::new(root).to_owned(),
        }
    }

    fn get_path(&self, rel: &Path) -> PathBuf {
        self.root.join(rel)
    }
}

impl FileSystemAPI for NativeFileSystem {
    fn exists(&self, path: &Path) -> Result<bool, Box<dyn Error>> {
        Ok(fs::metadata(self.get_path(path)).is_ok())
    }
    fn is_dir(&self, path: &Path) -> Result<bool, Box<dyn Error>> {
        Ok(self.get_path(path).is_dir())
    }
    fn remove_dir_all(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        Ok(fs::remove_dir_all(self.get_path(path))?)
    }
    fn create_dir_all(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        Ok(fs::create_dir_all(self.get_path(path))?)
    }
    fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>, Box<dyn Error>> {
        let mut files = vec![];
        for f in (fs::read_dir(self.get_path(path))?).flatten() {
            files.push(f.path().strip_prefix(&self.root)?.to_path_buf());
        }
        Ok(files)
    }
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        Ok(Some(fs::read(self.get_path(path))?))
    }
    fn read_to_string(&self, path: &Path) -> Result<Option<String>, Box<dyn Error>> {
        Ok(Some(fs::read_to_string(self.get_path(path))?))
    }
    fn delete(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        if self.is_dir(path)? {
            return Err(ArchivalError::new("use remove_dir_all to delete directories").into());
        }
        Ok(fs::remove_file(self.get_path(path))?)
    }
    fn write_str(&mut self, path: &Path, contents: String) -> Result<(), Box<dyn Error>> {
        Ok(fs::write(self.get_path(path), contents)?)
    }
    fn write(&mut self, path: &Path, contents: Vec<u8>) -> Result<(), Box<dyn Error>> {
        Ok(fs::write(self.get_path(path), contents)?)
    }
    fn copy_recursive(&mut self, from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
        let mut options = fs_extra::dir::CopyOptions::new();
        options.overwrite = true;
        options.content_only = true;
        fs_extra::dir::copy(self.get_path(from), self.get_path(to), &options)?;
        Ok(())
    }
    fn walk_dir(&self, path: &Path) -> Result<Box<dyn Iterator<Item = PathBuf>>, Box<dyn Error>> {
        let iterator = WalkDir::new(self.get_path(path))
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|de| de.into_path());
        Ok(Box::new(iterator))
    }
}

impl WatchableFileSystemAPI for NativeFileSystem {
    fn watch(
        &self,
        root: PathBuf,
        watch_paths: Vec<String>,
        changed: impl Fn(Vec<PathBuf>) + Send + Sync + 'static,
    ) -> Result<Box<dyn FnOnce()>, Box<dyn Error>> {
        let root = fs::canonicalize(self.get_path(&root)).unwrap();
        let watch_path = root.to_owned();
        changed(vec![]);
        let mut watcher = notify::recommended_watcher(
            move |res: Result<notify::Event, notify::Error>| match res {
                Ok(event) => {
                    let changed_paths: Vec<PathBuf> = event
                        .paths
                        .into_iter()
                        .filter(|p| {
                            let p = if let Ok(f) = fs::canonicalize(p) {
                                f
                            } else {
                                warn!("Invalid path {}", p.display());
                                return false;
                            };
                            if let Ok(rel) = p.strip_prefix(&root) {
                                for dir in &watch_paths {
                                    let mut dir = dir.to_string();
                                    if let Ok(stripped) = Path::new(&dir).strip_prefix(&root) {
                                        dir = stripped.to_string_lossy().into_owned();
                                    }
                                    if rel.starts_with(dir) {
                                        return true;
                                    }
                                }
                                false
                            } else {
                                warn!(
                                    "File changed outside of root ({}): {}",
                                    root.display(),
                                    p.display()
                                );
                                true
                            }
                        })
                        .collect();
                    if !changed_paths.is_empty() {
                        changed(changed_paths);
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            },
        )?;

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(&watch_path, RecursiveMode::Recursive)?;
        let path = watch_path.to_owned();
        let unwatch = move || {
            watcher.unwatch(&path).unwrap();
        };
        Ok(Box::new(unwatch))
    }
}
