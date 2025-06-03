use crate::git::GitAnalyzer;
use crate::models::{CodeDebtItem, Severity};
use crate::patterns::Pattern;
use crate::progress::ProgressReporter;
use anyhow::Result;
use git2::Repository;
use ignore::WalkBuilder;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct CodeDebtScanner {
    pub(crate) patterns: Vec<Pattern>,
    pub(crate) file_extensions: Vec<String>,
    pub(crate) ignore_dirs: Vec<String>,
    pub(crate) enable_git_blame: bool,
    pub(crate) detect_duplicates: bool,
    pub(crate) git_repo: Option<Repository>,
    pub(crate) progress_reporter: Option<Box<dyn ProgressReporter>>,
}

impl Default for CodeDebtScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeDebtScanner {
    pub fn new() -> Self {
        Self {
            patterns: Pattern::default_patterns(),
            file_extensions: Pattern::default_file_extensions(),
            ignore_dirs: Pattern::default_ignore_dirs(),
            enable_git_blame: false,
            detect_duplicates: false,
            git_repo: None,
            progress_reporter: None,
        }
    }

    pub fn with_patterns(mut self, patterns: Vec<Pattern>) -> Self {
        // Validate patterns before setting them
        if patterns.is_empty() {
            // Keep default patterns if no patterns provided
            return self;
        }
        self.patterns = patterns;
        self
    }

    pub fn with_file_extensions(mut self, extensions: Vec<String>) -> Self {
        self.file_extensions = extensions;
        self
    }

    pub fn with_ignore_dirs(mut self, dirs: Vec<String>) -> Self {
        self.ignore_dirs = dirs;
        self
    }

    pub fn with_git_blame(mut self, enable: bool) -> Self {
        self.enable_git_blame = enable;
        if enable {
            // Try to open git repository
            if let Ok(repo) = Repository::discover(".") {
                self.git_repo = Some(repo);
            }
        }
        self
    }

    pub fn with_duplicate_detection(mut self, enable: bool) -> Self {
        self.detect_duplicates = enable;
        self
    }

    pub fn with_progress_reporter(mut self, reporter: Box<dyn ProgressReporter>) -> Self {
        self.progress_reporter = Some(reporter);
        self
    }

    pub fn scan<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<CodeDebtItem>> {
        let patterns = Arc::new(&self.patterns);
        let extensions: HashSet<String> = self.file_extensions.iter().cloned().collect();

        // Count total files for progress reporting
        let total_files = if self.progress_reporter.is_some() {
            self.count_files(&root_path)?
        } else {
            0
        };

        if let Some(reporter) = &self.progress_reporter {
            reporter.start(total_files);
        }

        let walker = WalkBuilder::new(&root_path)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .build_parallel();

        let (tx, rx) = std::sync::mpsc::channel();
        let progress_tx = tx.clone();

        walker.run(|| {
            let tx = tx.clone();
            let progress_tx = progress_tx.clone();
            let patterns = Arc::clone(&patterns);
            let extensions = extensions.clone();

            Box::new(move |entry| {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();

                        if path.is_file() {
                            if let Some(ext) = path.extension() {
                                if let Some(ext_str) = ext.to_str() {
                                    if extensions.contains(ext_str) {
                                        if let Ok(content) = std::fs::read_to_string(path) {
                                            let items =
                                                Self::scan_content(path, &content, &patterns);
                                            for item in items {
                                                let _ = tx.send(item);
                                            }
                                        }
                                        // Send progress update
                                        let _ = progress_tx.send(CodeDebtItem {
                                            file_path: PathBuf::new(),
                                            line_number: 0,
                                            column: 0,
                                            line_content: String::new(),
                                            pattern_type: "__PROGRESS__".to_string(),
                                            severity: Severity::Low,
                                            author: None,
                                            age_days: None,
                                            commit_hash: None,
                                            created_at: None,
                                            file_extension: None,
                                            duplicate_count: 0,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // Skip files we can't access (permissions, etc)
                    }
                }
                ignore::WalkState::Continue
            })
        });

        drop(tx);
        drop(progress_tx);

        let mut results: Vec<CodeDebtItem> = Vec::new();
        let mut processed_files = 0;

        for item in rx.iter() {
            if item.pattern_type == "__PROGRESS__" {
                processed_files += 1;
                if let Some(reporter) = &self.progress_reporter {
                    reporter.update(processed_files);
                }
            } else {
                results.push(item);
            }
        }

        if let Some(reporter) = &self.progress_reporter {
            reporter.finish();
        }

        // Add git blame information if enabled
        if self.enable_git_blame {
            GitAnalyzer::add_git_information(self.git_repo.as_ref(), &mut results);
        }

        // Detect duplicates if enabled
        if self.detect_duplicates {
            self.detect_duplicate_patterns(&mut results);
        }

        // Add file extension information
        self.add_file_extensions(&mut results);

        results.sort_by(|a, b| {
            a.severity
                .cmp(&b.severity)
                .then_with(|| a.file_path.cmp(&b.file_path))
                .then_with(|| a.line_number.cmp(&b.line_number))
        });

        Ok(results)
    }

    fn count_files<P: AsRef<Path>>(&self, root_path: P) -> Result<usize> {
        let extensions: HashSet<String> = self.file_extensions.iter().cloned().collect();
        let walker = WalkBuilder::new(&root_path)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .build();

        let count = walker
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let path = entry.path();
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| extensions.contains(ext))
                        .unwrap_or(false)
            })
            .count();

        Ok(count)
    }

    pub(crate) fn scan_content(
        file_path: &Path,
        content: &str,
        patterns: &[Pattern],
    ) -> Vec<CodeDebtItem> {
        content
            .lines()
            .enumerate()
            .flat_map(|(line_idx, line)| {
                patterns
                    .iter()
                    .filter_map(|pattern| {
                        pattern.regex.find(line).map(|m| CodeDebtItem {
                            file_path: file_path.to_path_buf(),
                            line_number: line_idx + 1,
                            column: m.start() + 1,
                            line_content: line.trim().to_string(),
                            pattern_type: pattern.name.clone(),
                            severity: pattern.severity.clone(),
                            author: None,
                            age_days: None,
                            commit_hash: None,
                            created_at: None,
                            file_extension: None,
                            duplicate_count: 0,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn get_summary(&self, items: &[CodeDebtItem]) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for item in items {
            *summary.entry(item.pattern_type.clone()).or_insert(0) += 1;
        }
        summary
    }

    pub fn filter_by_severity(
        &self,
        items: &[CodeDebtItem],
        min_severity: Severity,
    ) -> Vec<CodeDebtItem> {
        items
            .iter()
            .filter(|item| item.severity <= min_severity)
            .cloned()
            .collect()
    }

    fn detect_duplicate_patterns(&self, items: &mut [CodeDebtItem]) {
        let mut pattern_counts: HashMap<String, usize> = HashMap::new();

        // Count occurrences of similar patterns
        for item in items.iter() {
            let key = format!("{}:{}", item.pattern_type, item.line_content.trim());
            *pattern_counts.entry(key).or_insert(0) += 1;
        }

        // Update duplicate counts
        for item in items.iter_mut() {
            let key = format!("{}:{}", item.pattern_type, item.line_content.trim());
            item.duplicate_count = pattern_counts.get(&key).copied().unwrap_or(0);
        }
    }

    fn add_file_extensions(&self, items: &mut [CodeDebtItem]) {
        for item in items.iter_mut() {
            if let Some(ext) = item.file_path.extension() {
                item.file_extension = ext.to_str().map(|s| s.to_string());
            }
        }
    }

    pub fn get_file_type_summary(&self, items: &[CodeDebtItem]) -> HashMap<String, usize> {
        let mut summary = HashMap::new();
        for item in items {
            let file_type = item.file_extension.as_deref().unwrap_or("unknown");
            *summary.entry(file_type.to_string()).or_insert(0) += 1;
        }
        summary
    }

    pub fn get_age_distribution(&self, items: &[CodeDebtItem]) -> HashMap<String, usize> {
        let mut distribution = HashMap::new();
        for item in items {
            if let Some(age) = item.age_days {
                let bucket = match age {
                    0..=7 => "This week",
                    8..=30 => "This month",
                    31..=90 => "Last 3 months",
                    91..=365 => "This year",
                    _ => "Over a year",
                };
                *distribution.entry(bucket.to_string()).or_insert(0) += 1;
            } else {
                *distribution.entry("Unknown age".to_string()).or_insert(0) += 1;
            }
        }
        distribution
    }

    pub fn filter_by_age(&self, items: &[CodeDebtItem], max_age_days: i64) -> Vec<CodeDebtItem> {
        items
            .iter()
            .filter(|item| match item.age_days {
                None => true,
                Some(age) => age <= max_age_days,
            })
            .cloned()
            .collect()
    }

    pub fn find_duplicates(&self, items: &[CodeDebtItem], min_count: usize) -> Vec<CodeDebtItem> {
        items
            .iter()
            .filter(|item| item.duplicate_count >= min_count)
            .cloned()
            .collect()
    }
}
