use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use fs_extra::dir::{get_details_entry, DirEntryAttr, DirEntryValue};
use tokio::sync::Mutex;
use walkdir::WalkDir;

use crate::cli::{Args, NodeModule};

const READ_BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

thread_local! {
    static DIR_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(Vec::with_capacity(READ_BUFFER_SIZE));
}

#[inline]
pub fn get_dir_details(
    path: &PathBuf,
) -> Option<(HashMap<DirEntryAttr, DirEntryValue>, SystemTime)> {
    let parent_path = path.parent()?;

    let mut config = HashSet::with_capacity(2);
    config.insert(DirEntryAttr::Size);
    config.insert(DirEntryAttr::Modified);

    DIR_BUFFER.with(|buffer| {
        let mut buffer = buffer.borrow_mut();
        buffer.clear();
        let node_details = get_details_entry(path, &config).ok()?;
        let parent_modified = get_details_entry(parent_path, &config)
            .ok()?
            .get(&DirEntryAttr::Modified)
            .and_then(|v| match v {
                DirEntryValue::SystemTime(time) => Some(*time),
                _ => None,
            })?;

        Some((node_details, parent_modified))
    })
}

#[inline]
pub fn is_nested_module(path: &Path, target: &str) -> bool {
    path.to_string_lossy().matches(target).count() > 1
}

pub async fn scan_directory(root: PathBuf, args: Args, results: Arc<Mutex<Vec<NodeModule>>>) {
    let canonical_root = match std::fs::canonicalize(&root) {
        Ok(path) => path,
        Err(_) => return,
    };

    let target = args.target.clone();
    let excluded_paths_option = &args.exclude_paths;
    let excluded_paths: Vec<&str> = if let Some(excluded_paths) = excluded_paths_option {
        excluded_paths.split(",").collect()
    } else {
        Vec::new()
    };

    let entries: Vec<_> = WalkDir::new(&canonical_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(move |e| {
            let is_target = e.file_name().to_string_lossy() == target;
            let path = e.path().to_string_lossy();

            let is_excluded = excluded_paths
                .iter()
                .any(|excluded| path.contains(excluded));

            if is_target {
                !is_nested_module(e.path(), &target) && !is_excluded
            } else {
                (!args.exclude_hidden || !is_dangerous(e.path())) && !is_excluded
            }
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy() == args.target)
        .collect();

    let modules: Vec<_> = entries
        .par_iter()
        .map(|e| {
            let path = e.path().to_path_buf();
            let attrs = get_dir_details(&path);
            NodeModule::new(path, attrs)
        })
        .collect();

    let mut results = results.lock().await;
    results.extend(modules);
}

pub fn is_dangerous(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    let is_hidden = path_str
        .split('/')
        .chain(path_str.split('\\'))
        .any(|part| part.starts_with('.') && part != "." && part != "..");

    let is_mac_app = path_str.contains(".app/") || path_str.ends_with(".app");

    let is_windows_app_data = path_str.contains("\\AppData\\");

    is_hidden || is_mac_app || is_windows_app_data
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_hidden_file_unix() {
        let path = PathBuf::from("/home/user/.hidden_file");
        assert!(
            is_dangerous(&path),
            "Hidden file on Unix should be dangerous"
        );
    }

    #[test]
    fn test_hidden_file_windows() {
        let path = PathBuf::from("C:\\Users\\user\\.hidden_file");
        assert!(
            is_dangerous(&path),
            "Hidden file on Windows should be dangerous"
        );
    }

    #[test]
    fn test_hidden_directory_unix() {
        let path = PathBuf::from("/home/user/.hidden_dir/file.txt");
        assert!(
            is_dangerous(&path),
            "File in hidden directory on Unix should be dangerous"
        );
    }

    #[test]
    fn test_hidden_directory_windows() {
        let path = PathBuf::from("C:\\Users\\user\\.hidden_dir\\file.txt");
        assert!(
            is_dangerous(&path),
            "File in hidden directory on Windows should be dangerous"
        );
    }

    #[test]
    fn test_mac_app_bundle() {
        let path = PathBuf::from("/Applications/MyApp.app/Contents/MacOS/MyApp");
        assert!(is_dangerous(&path), "macOS app bundle should be dangerous");
    }

    #[test]
    fn test_windows_app_data() {
        let path = PathBuf::from("C:\\Users\\user\\AppData\\Local\\Temp\\file.txt");
        assert!(
            is_dangerous(&path),
            "Windows AppData path should be dangerous"
        );
    }

    #[test]
    fn test_safe_path_unix() {
        let path = PathBuf::from("/home/user/Documents/file.txt");
        assert!(
            !is_dangerous(&path),
            "Normal Unix path should not be dangerous"
        );
    }

    #[test]
    fn test_safe_path_windows() {
        let path = PathBuf::from("C:\\Users\\user\\Documents\\file.txt");
        assert!(
            !is_dangerous(&path),
            "Normal Windows path should not be dangerous"
        );
    }

    #[test]
    fn test_root_path() {
        let path = PathBuf::from("/");
        assert!(!is_dangerous(&path), "Root path should not be dangerous");
    }

    #[test]
    fn test_empty_path() {
        let path = PathBuf::from("");
        assert!(!is_dangerous(&path), "Empty path should not be dangerous");
    }

    #[test]
    fn test_dot_path() {
        let path = PathBuf::from(".");
        assert!(
            !is_dangerous(&path),
            "Current directory path should not be dangerous"
        );
    }

    #[test]
    fn test_dot_dot_path() {
        let path = PathBuf::from("..");
        assert!(
            !is_dangerous(&path),
            "Parent directory path should not be dangerous"
        );
    }
}
