use colored::*;
use std::io::{self, Write};

pub trait ProgressReporter: Send + Sync {
    fn start(&self, total: usize);
    fn update(&self, current: usize);
    fn finish(&self);
}

pub struct TerminalProgressReporter {
    total: std::sync::Mutex<usize>,
    show_progress: bool,
}

impl TerminalProgressReporter {
    pub fn new(show_progress: bool) -> Self {
        Self {
            total: std::sync::Mutex::new(0),
            show_progress,
        }
    }
}

impl ProgressReporter for TerminalProgressReporter {
    fn start(&self, total: usize) {
        if self.show_progress && total > 100 {
            *self.total.lock().unwrap() = total;
            print!("{} Scanning files... ", "⏳".cyan());
            let _ = io::stdout().flush();
        }
    }

    fn update(&self, current: usize) {
        if self.show_progress {
            let total = *self.total.lock().unwrap();
            if total > 100 && current % 10 == 0 {
                let percentage = (current as f32 / total as f32 * 100.0) as u32;
                print!("\r{} Scanning files... {}% ", "⏳".cyan(), percentage);
                let _ = io::stdout().flush();
            }
        }
    }

    fn finish(&self) {
        if self.show_progress {
            let total = *self.total.lock().unwrap();
            if total > 100 {
                print!("\r{} Scanning complete!     \n", "✅".green());
                let _ = io::stdout().flush();
            }
        }
    }
}
