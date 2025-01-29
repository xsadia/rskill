use clap::Parser;
use rskill::{
    cli::{Args, NodeModule, SortBy},
    fs, tui,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.delete_all {
        let confirmed = tui::confirm_delete_all(&args.target)?;
        if !confirmed {
            return Ok(());
        }
    }
    let results = Arc::new(Mutex::new(Vec::<NodeModule>::with_capacity(1000)));
    let mut handles = Vec::with_capacity(10);

    let start_dir = if args.full {
        PathBuf::from(std::env::var("HOME")?)
    } else {
        std::fs::canonicalize(&args.directory)?
    };

    let scanning = Arc::new(AtomicBool::new(true));
    let start = std::time::Instant::now();
    let spinner_handle = {
        let scanning = Arc::clone(&scanning);
        tokio::spawn(tui::display_spinner(scanning))
    };

    let mut entries = tokio::fs::read_dir(&start_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let results = Arc::clone(&results);
        let args = args.clone();
        let path = entry.path();

        handles.push(tokio::spawn(async move {
            fs::scan_directory(path, args, results).await;
        }));
    }

    for handle in handles {
        handle.await?;
    }

    scanning.store(false, Ordering::Relaxed);
    let _ = spinner_handle.await?;

    let modules = results.lock().await;

    let modules_vec = match &args.sort {
        Some(o) => match o {
            SortBy::Path => {
                let mut modules_vec = modules.to_vec();
                modules_vec.sort_unstable_by(|a, b| {
                    a.path
                        .to_string_lossy()
                        .partial_cmp(&b.path.to_string_lossy())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                modules_vec
            }
            SortBy::Size => {
                let mut modules_vec = modules.to_vec();
                modules_vec.sort_unstable_by(|a, b| {
                    b.size
                        .partial_cmp(&a.size)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                modules_vec
            }
            SortBy::LastMod => {
                let mut modules_vec = modules.to_vec();
                modules_vec.sort_unstable_by(|a, b| b.modified.cmp(&a.modified));
                modules_vec
            }
        },
        None => modules.to_vec(),
    };

    let _ = tui::run_tui(modules_vec, args, start);
    Ok(())
}
