use chrono::{DateTime, Local};
use clap::Parser;
use crossterm::event::KeyCode;
use fs_extra::dir::{DirEntryAttr, DirEntryValue};
use std::time::{Duration, Instant};
use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use crate::fs::is_dangerous;

#[derive(Debug, Clone, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum SortBy {
    Size,
    Path,
    LastMod,
}

#[derive(Debug, Clone)]
pub struct NodeModule {
    pub path: PathBuf,
    pub size: u64,
    pub modified: i64,
    pub deleted: bool,
    pub is_dangerous: bool,
}

impl NodeModule {
    #[inline]
    pub fn new(
        path: PathBuf,
        details: Option<(HashMap<DirEntryAttr, DirEntryValue>, SystemTime)>,
    ) -> Self {
        let (size, modified) = if let Some((attrs, parent_modified)) = details {
            let size = attrs.get(&DirEntryAttr::Size).and_then(|v| match v {
                DirEntryValue::U64(size) => Some(*size),
                _ => None,
            });

            let size = size.unwrap_or(0);

            (size, parent_modified)
        } else {
            (0, SystemTime::now())
        };

        let modified = {
            let local = DateTime::<Local>::from(modified);
            let now = Local::now().signed_duration_since(local);
            now.num_seconds()
        };

        NodeModule {
            path: path.clone(),
            size,
            modified,
            deleted: false,
            is_dangerous: is_dangerous(&path),
        }
    }
}

pub struct App {
    pub modules: Vec<NodeModule>,
    pub scroll: usize,
    pub scan_time: Duration,
    pub total_deleted: u64,
}

impl App {
    pub fn new(modules: Vec<NodeModule>, start: Instant) -> Self {
        Self {
            modules,
            scroll: 0,
            scan_time: start.elapsed(),
            total_deleted: 0,
        }
    }

    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up if self.scroll > 0 => self.scroll -= 1,
            KeyCode::Down if self.scroll < self.modules.len().saturating_sub(1) => self.scroll += 1,
            KeyCode::Char(' ') => {
                if let Some(module) = self.modules.get_mut(self.scroll) {
                    if module.deleted {
                        return;
                    }

                    let path = module.path.clone();
                    module.deleted = true;

                    tokio::spawn(tokio::fs::remove_dir_all(path));

                    self.total_deleted += module.size;
                }
            }
            _ => {}
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    ///Set the directory from which to begin searching. By default, starting-point is .
    #[arg(short, long, default_value_t = String::from("."))]
    pub directory: String,

    ///Exclude directories from search (directory list must be inside double quotes "", each directory separated by ',' ) Example: "ignore1, ignore2"
    #[arg(
        short = 'x',
        long = "exclude-hidden-directories",
        default_value_t = false
    )]
    pub exclude_hidden: bool,

    ///Specify the name of the directories you want to search (by default, is node_modules)
    #[arg(short, long, default_value_t = String::from("node_modules"))]
    pub target: String,

    ///Start searching from the home of the user (example: "/home/user" in linux)
    #[arg(short, long, default_value_t = false)]
    pub full: bool,

    ///Show folders in Gigabytes instead of Megabytes.
    #[arg(long = "gb", default_value_t = false)]
    pub in_gb: bool,

    ///Exclude directories from search (directory list must be inside double quotes "", each directory separated by ',' ) Example: "ignore1, ignore2"
    #[arg(long = "exclude", short = 'E')]
    pub exclude_paths: Option<String>,

    /// Sort results by: size, path or last-mod
    #[arg(long, short, value_enum)]
    pub sort: Option<SortBy>,
}
