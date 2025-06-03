use crate::scanner::CodeDebtScanner;
use anyhow::Result;
use colored::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::{Duration, Instant};

pub struct CodeDebtWatcher {
    scanner: CodeDebtScanner,
    paths: Vec<String>,
}

impl CodeDebtWatcher {
    pub fn new(scanner: CodeDebtScanner, paths: Vec<String>) -> Self {
        Self { scanner, paths }
    }

    pub fn watch(&self) -> Result<()> {
        println!("{} Watch mode started. Press Ctrl+C to exit.", "👁️ ".cyan());
        println!("{}", "─".repeat(50).dimmed());

        // Initial scan
        self.run_scan()?;

        // Set up file watcher
        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(
            tx,
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;

        // Watch all paths
        for path in &self.paths {
            watcher.watch(Path::new(path), RecursiveMode::Recursive)?;
        }

        // Watch for changes with debouncing
        let debounce_duration = Duration::from_millis(300);
        let mut last_scan = Instant::now();
        let mut pending_rescan = false;

        loop {
            match rx.recv_timeout(debounce_duration) {
                Ok(Ok(event)) => {
                    if self.should_rescan(&event) {
                        pending_rescan = true;
                        // Don't scan immediately, wait for debounce
                    }
                }
                Ok(Err(e)) => eprintln!("Watch error: {:?}", e),
                Err(RecvTimeoutError::Timeout) => {
                    // Check if we have a pending rescan and enough time has passed
                    if pending_rescan && last_scan.elapsed() >= debounce_duration {
                        println!("\n{} Change detected, rescanning...", "🔄".yellow());
                        self.run_scan()?;
                        last_scan = Instant::now();
                        pending_rescan = false;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        Ok(())
    }

    fn should_rescan(&self, event: &Event) -> bool {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                event.paths.iter().any(|path| {
                    if let Some(ext) = path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            return self.scanner.file_extensions.contains(&ext_str.to_string());
                        }
                    }
                    false
                })
            }
            _ => false,
        }
    }

    fn run_scan(&self) -> Result<()> {
        let start = std::time::Instant::now();
        let mut all_items = Vec::new();

        // Scan all paths
        for path in &self.paths {
            match self.scanner.scan(path) {
                Ok(items) => all_items.extend(items),
                Err(e) => eprintln!("Error scanning {}: {}", path, e),
            }
        }

        let duration = start.elapsed();

        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        if all_items.is_empty() {
            println!("{} No code debt found!", "✅".green());
        } else {
            println!("{} {} code debt items found:", "🔍".cyan(), all_items.len());
            println!("{}", "─".repeat(50).dimmed());

            // Show summary by severity
            let mut by_severity = std::collections::HashMap::new();
            for item in &all_items {
                *by_severity
                    .entry(format!("{:?}", item.severity))
                    .or_insert(0) += 1;
            }

            for (severity, count) in by_severity {
                let colored_severity = match severity.as_str() {
                    "Critical" => severity.red().bold(),
                    "High" => severity.yellow().bold(),
                    "Medium" => severity.blue().bold(),
                    "Low" => severity.white().bold(),
                    _ => severity.normal(),
                };
                println!("  {} {}", colored_severity, count);
            }
        }

        println!(
            "\n{} Scanned in {:.2}ms",
            "⚡".bright_yellow(),
            duration.as_secs_f64() * 1000.0
        );
        println!("{}", "─".repeat(50).dimmed());
        println!("{} Watching for changes...", "👁️ ".cyan());

        Ok(())
    }
}
