use crate::scanner::CodeDebtScanner;
use anyhow::Result;
use colored::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct CodeDebtWatcher {
    scanner: CodeDebtScanner,
    path: String,
}

impl CodeDebtWatcher {
    pub fn new(scanner: CodeDebtScanner, path: String) -> Self {
        Self { scanner, path }
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

        watcher.watch(Path::new(&self.path), RecursiveMode::Recursive)?;

        // Watch for changes
        for res in rx {
            match res {
                Ok(event) => {
                    if self.should_rescan(&event) {
                        println!("\n{} Change detected, rescanning...", "🔄".yellow());
                        self.run_scan()?;
                    }
                }
                Err(e) => eprintln!("Watch error: {:?}", e),
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
        let items = self.scanner.scan(&self.path)?;
        let duration = start.elapsed();

        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        if items.is_empty() {
            println!("{} No code debt found!", "✅".green());
        } else {
            println!("{} {} code debt items found:", "🔍".cyan(), items.len());
            println!("{}", "─".repeat(50).dimmed());

            // Show summary by severity
            let mut by_severity = std::collections::HashMap::new();
            for item in &items {
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