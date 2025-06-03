use crate::models::{CodeDebtItem, Severity};
use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};

pub struct InteractiveMode {
    items: Vec<CodeDebtItem>,
    filtered_items: Vec<CodeDebtItem>,
    current_index: usize,
    filter_severity: Option<Severity>,
    sort_by: SortBy,
}

#[derive(Clone, Copy, Debug)]
enum SortBy {
    Severity,
    File,
    Age,
}

impl InteractiveMode {
    pub fn new(items: Vec<CodeDebtItem>) -> Self {
        let filtered_items = items.clone();
        Self {
            items,
            filtered_items,
            current_index: 0,
            filter_severity: None,
            sort_by: SortBy::Severity,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;

        loop {
            self.draw_ui()?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                    KeyCode::PageUp => self.page_up(),
                    KeyCode::PageDown => self.page_down(),
                    KeyCode::Char('s') => self.cycle_sort(),
                    KeyCode::Char('f') => self.cycle_filter(),
                    KeyCode::Enter => self.show_details()?,
                    _ => {}
                }
            }
        }

        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    fn draw_ui(&self) -> Result<()> {
        print!("\x1B[2J\x1B[1;1H"); // Clear screen

        // Header
        println!("{} Code Debt Interactive Explorer", "🔍".cyan().bold());
        println!("{}", "═".repeat(60).dimmed());

        // Stats
        println!(
            "Total: {} | Filtered: {} | Sort: {:?} | Filter: {:?}",
            self.items.len().to_string().yellow(),
            self.filtered_items.len().to_string().yellow(),
            self.sort_by,
            self.filter_severity
        );
        println!("{}", "─".repeat(60).dimmed());

        // Items
        if !self.filtered_items.is_empty() {
            let start = self.current_index.saturating_sub(10);
            let end = (start + 20).min(self.filtered_items.len());

            for (idx, item) in self.filtered_items[start..end].iter().enumerate() {
                let abs_idx = start + idx;
                let is_selected = abs_idx == self.current_index;

                let severity_icon = match item.severity {
                    Severity::Critical => "🚨",
                    Severity::High => "⚠️ ",
                    Severity::Medium => "📝",
                    Severity::Low => "💡",
                };

                let line = format!(
                    "{} {} {} {}:{}",
                    severity_icon,
                    item.pattern_type,
                    item.file_path.display(),
                    item.line_number,
                    item.column
                );

                if is_selected {
                    println!("{} {}", ">".green().bold(), line.bold());
                } else {
                    println!("  {}", line);
                }
            }
        }

        // Help
        println!("\n{}", "─".repeat(60).dimmed());
        println!(
            "{}: Navigate | {}: Sort | {}: Filter | {}: Details | {}: Quit",
            "↑↓/jk".cyan(),
            "s".cyan(),
            "f".cyan(),
            "Enter".cyan(),
            "q/Esc".cyan()
        );

        io::stdout().flush()?;
        Ok(())
    }

    fn move_up(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.current_index < self.filtered_items.len().saturating_sub(1) {
            self.current_index += 1;
        }
    }

    fn page_up(&mut self) {
        self.current_index = self.current_index.saturating_sub(10);
    }

    fn page_down(&mut self) {
        self.current_index =
            (self.current_index + 10).min(self.filtered_items.len().saturating_sub(1));
    }

    fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            SortBy::Severity => SortBy::File,
            SortBy::File => SortBy::Age,
            SortBy::Age => SortBy::Severity,
        };
        self.apply_sort();
    }

    fn cycle_filter(&mut self) {
        self.filter_severity = match self.filter_severity {
            None => Some(Severity::Low),
            Some(Severity::Low) => Some(Severity::Medium),
            Some(Severity::Medium) => Some(Severity::High),
            Some(Severity::High) => Some(Severity::Critical),
            Some(Severity::Critical) => None,
        };
        self.apply_filter();
    }

    fn apply_sort(&mut self) {
        match self.sort_by {
            SortBy::Severity => {
                self.filtered_items.sort_by(|a, b| {
                    a.severity
                        .cmp(&b.severity)
                        .then_with(|| a.file_path.cmp(&b.file_path))
                });
            }
            SortBy::File => {
                self.filtered_items.sort_by(|a, b| {
                    a.file_path
                        .cmp(&b.file_path)
                        .then_with(|| a.line_number.cmp(&b.line_number))
                });
            }
            SortBy::Age => {
                self.filtered_items.sort_by(|a, b| {
                    b.age_days
                        .unwrap_or(0)
                        .cmp(&a.age_days.unwrap_or(0))
                        .then_with(|| a.severity.cmp(&b.severity))
                });
            }
        }
        self.current_index = 0;
    }

    fn apply_filter(&mut self) {
        self.filtered_items = if let Some(max_severity) = &self.filter_severity {
            self.items
                .iter()
                .filter(|item| item.severity <= *max_severity)
                .cloned()
                .collect()
        } else {
            self.items.clone()
        };
        self.apply_sort();
    }

    fn show_details(&self) -> Result<()> {
        if let Some(item) = self.filtered_items.get(self.current_index) {
            print!("\x1B[2J\x1B[1;1H"); // Clear screen

            println!("{} Code Debt Details", "📋".cyan().bold());
            println!("{}", "═".repeat(60).dimmed());

            println!("File: {}", item.file_path.display().to_string().cyan());
            println!("Line: {}", item.line_number.to_string().green());
            println!("Column: {}", item.column.to_string().green());
            println!("Pattern: {}", item.pattern_type.purple());
            println!("Severity: {:?}", item.severity);

            if let Some(author) = &item.author {
                println!("Author: {}", author.yellow());
            }

            if let Some(age) = item.age_days {
                println!("Age: {} days", age.to_string().yellow());
            }

            if item.duplicate_count > 1 {
                println!("Duplicates: {}", item.duplicate_count.to_string().red());
            }

            println!("\nContent:");
            println!("{}", "─".repeat(60).dimmed());
            println!("{}", item.line_content);
            println!("{}", "─".repeat(60).dimmed());

            println!("\nPress any key to return...");
            io::stdout().flush()?;
            event::read()?;
        }
        Ok(())
    }
}
