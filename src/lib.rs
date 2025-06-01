use anyhow::Result;
use chrono::{DateTime, Utc};
use git2::{Repository, BlameOptions};
use ignore::WalkBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDebtItem {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub column: usize,
    pub line_content: String,
    pub pattern_type: String,
    pub severity: Severity,
    
    // Enhanced intelligence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_extension: Option<String>,
    pub duplicate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub name: String,
    pub regex: Regex,
    pub severity: Severity,
}

pub struct CodeDebtScanner {
    patterns: Vec<Pattern>,
    file_extensions: Vec<String>,
    ignore_dirs: Vec<String>,
    enable_git_blame: bool,
    detect_duplicates: bool,
    git_repo: Option<Repository>,
}

impl Default for CodeDebtScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeDebtScanner {
    pub fn new() -> Self {
        let patterns = vec![
            Pattern {
                name: "HACK".to_string(),
                regex: Regex::new(r"(?i)\b(HACK|XXX)\b").unwrap(),
                severity: Severity::Critical,
            },
            Pattern {
                name: "FIXME".to_string(),
                regex: Regex::new(r"(?i)\bFIXME\b").unwrap(),
                severity: Severity::High,
            },
            Pattern {
                name: "TODO".to_string(),
                regex: Regex::new(r"(?i)\bTODO\b").unwrap(),
                severity: Severity::Medium,
            },
            Pattern {
                name: "NOTE_FIX".to_string(),
                regex: Regex::new(r"(?i)\bNOTE.*fix\b").unwrap(),
                severity: Severity::Medium,
            },
            Pattern {
                name: "TEMPORARY".to_string(),
                regex: Regex::new(r"(?i)\b(temporary|temp|placeholder)\b").unwrap(),
                severity: Severity::High,
            },
            Pattern {
                name: "MOCK_STUB".to_string(),
                regex: Regex::new(r"(?i)\b(mock|stub)\b").unwrap(),
                severity: Severity::Low,
            },
            Pattern {
                name: "PRODUCTION_DEBT".to_string(),
                regex: Regex::new(r"(?i)(temporary|placeholder|mock).*production").unwrap(),
                severity: Severity::Critical,
            },
        ];

        let file_extensions = vec![
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "cc", "cxx", "h",
            "hpp", "rb", "php", "cs", "swift", "kt", "scala", "clj", "ml", "hs", "elm", "dart",
            "lua", "pl", "r", "jl", "nim", "zig", "v", "cr",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let ignore_dirs = vec![
            "node_modules",
            ".git",
            "target",
            "dist",
            "build",
            ".next",
            "vendor",
            "__pycache__",
            ".pytest_cache",
            "coverage",
            ".nyc_output",
            "bower_components",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            patterns,
            file_extensions,
            ignore_dirs,
            enable_git_blame: false,
            detect_duplicates: false,
            git_repo: None,
        }
    }

    pub fn with_patterns(mut self, patterns: Vec<Pattern>) -> Self {
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

    pub fn scan<P: AsRef<Path>>(&self, root_path: P) -> Result<Vec<CodeDebtItem>> {
        let patterns = Arc::new(&self.patterns);
        let extensions: HashSet<String> = self.file_extensions.iter().cloned().collect();

        let walker = WalkBuilder::new(&root_path)
            .hidden(false)
            .ignore(true)
            .git_ignore(true)
            .build_parallel();

        let (tx, rx) = std::sync::mpsc::channel();

        walker.run(|| {
            let tx = tx.clone();
            let patterns = Arc::clone(&patterns);
            let extensions = extensions.clone();

            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if let Some(ext_str) = ext.to_str() {
                                if extensions.contains(ext_str) {
                                    if let Ok(content) = std::fs::read_to_string(path) {
                                        let items = Self::scan_content(path, &content, &patterns);
                                        for item in items {
                                            let _ = tx.send(item);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        });

        drop(tx);
        let mut results: Vec<CodeDebtItem> = rx.iter().collect();
        
        // Add git blame information if enabled
        if self.enable_git_blame {
            self.add_git_information(&mut results);
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

    fn scan_content(file_path: &Path, content: &str, patterns: &[Pattern]) -> Vec<CodeDebtItem> {
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

    fn add_git_information(&self, items: &mut [CodeDebtItem]) {
        if let Some(repo) = &self.git_repo {
            for item in items.iter_mut() {
                if let Ok(relative_path) = item.file_path.strip_prefix(repo.workdir().unwrap_or_else(|| std::path::Path::new("."))) {
                    if let Ok(blame) = repo.blame_file(relative_path, Some(&mut BlameOptions::new())) {
                        if let Some(hunk) = blame.get_line(item.line_number) {
                            let sig = hunk.final_signature();
                            let oid = hunk.final_commit_id();
                            
                            item.author = sig.name().map(|s| s.to_string());
                            item.commit_hash = Some(oid.to_string());
                            
                            if let Ok(commit) = repo.find_commit(oid) {
                                let timestamp = commit.time().seconds();
                                let datetime = DateTime::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now());
                                item.created_at = Some(datetime);
                                let now = Utc::now();
                                let duration = now.signed_duration_since(datetime);
                                item.age_days = Some(duration.num_days());
                            }
                        }
                    }
                }
            }
        }
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
            .filter(|item| {
                item.age_days.map_or(true, |age| age <= max_age_days)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_scanner_creation() {
        let scanner = CodeDebtScanner::new();
        assert!(!scanner.patterns.is_empty());
        assert!(!scanner.file_extensions.is_empty());
        assert!(!scanner.ignore_dirs.is_empty());
    }

    #[test]
    fn test_default_patterns() {
        let scanner = CodeDebtScanner::new();
        let pattern_names: Vec<String> = scanner.patterns.iter().map(|p| p.name.clone()).collect();

        assert!(pattern_names.contains(&"TODO".to_string()));
        assert!(pattern_names.contains(&"FIXME".to_string()));
        assert!(pattern_names.contains(&"HACK".to_string()));
        assert!(pattern_names.contains(&"TEMPORARY".to_string()));
        assert!(pattern_names.contains(&"PRODUCTION_DEBT".to_string()));
    }

    #[test]
    fn test_scan_content() {
        let test_content = r#"
fn main() {
    // TODO: implement this function
    println!("Hello, world!");
    // FIXME: this is broken
    let x = 5;
    // HACK: workaround
    let y = x * 2;
}
"#;

        let scanner = CodeDebtScanner::new();
        let file_path = Path::new("test.rs");
        let items = CodeDebtScanner::scan_content(file_path, test_content, &scanner.patterns);

        assert_eq!(items.len(), 3);

        // Check TODO
        let todo_item = items
            .iter()
            .find(|item| item.pattern_type == "TODO")
            .unwrap();
        assert_eq!(todo_item.severity, Severity::Medium);
        assert_eq!(todo_item.line_number, 3);

        // Check FIXME
        let fixme_item = items
            .iter()
            .find(|item| item.pattern_type == "FIXME")
            .unwrap();
        assert_eq!(fixme_item.severity, Severity::High);
        assert_eq!(fixme_item.line_number, 5);

        // Check HACK
        let hack_item = items
            .iter()
            .find(|item| item.pattern_type == "HACK")
            .unwrap();
        assert_eq!(hack_item.severity, Severity::Critical);
        assert_eq!(hack_item.line_number, 7);
    }

    #[test]
    fn test_production_debt_pattern() {
        let test_content = r#"
const API_KEY = "placeholder for production";
let temp_production_fix = true;
"#;

        let scanner = CodeDebtScanner::new();
        let file_path = Path::new("test.js");
        let items = CodeDebtScanner::scan_content(file_path, test_content, &scanner.patterns);

        let production_debt = items
            .iter()
            .find(|item| item.pattern_type == "PRODUCTION_DEBT");
        assert!(production_debt.is_some());
        assert_eq!(production_debt.unwrap().severity, Severity::Critical);
    }

    #[test]
    fn test_custom_patterns() {
        let custom_patterns = vec![Pattern {
            name: "URGENT".to_string(),
            regex: Regex::new(r"(?i)\bURGENT\b").unwrap(),
            severity: Severity::Critical,
        }];

        let scanner = CodeDebtScanner::new().with_patterns(custom_patterns);
        assert_eq!(scanner.patterns.len(), 1);
        assert_eq!(scanner.patterns[0].name, "URGENT");
    }

    #[test]
    fn test_file_extensions_filter() {
        let scanner =
            CodeDebtScanner::new().with_file_extensions(vec!["rs".to_string(), "py".to_string()]);

        assert_eq!(scanner.file_extensions.len(), 2);
        assert!(scanner.file_extensions.contains(&"rs".to_string()));
        assert!(scanner.file_extensions.contains(&"py".to_string()));
    }

    #[test]
    fn test_ignore_dirs_filter() {
        let custom_ignore = vec!["my_custom_dir".to_string()];
        let scanner = CodeDebtScanner::new().with_ignore_dirs(custom_ignore);

        assert!(scanner.ignore_dirs.contains(&"my_custom_dir".to_string()));
    }

    #[test]
    fn test_get_summary() {
        let items = vec![
            CodeDebtItem {
                file_path: PathBuf::from("test.rs"),
                line_number: 1,
                column: 1,
                line_content: "// TODO: test".to_string(),
                pattern_type: "TODO".to_string(),
                severity: Severity::Medium,
                author: None,
                age_days: None,
                commit_hash: None,
                created_at: None,
                file_extension: None,
                duplicate_count: 0,
            },
            CodeDebtItem {
                file_path: PathBuf::from("test.rs"),
                line_number: 2,
                column: 1,
                line_content: "// TODO: another test".to_string(),
                pattern_type: "TODO".to_string(),
                severity: Severity::Medium,
                author: None,
                age_days: None,
                commit_hash: None,
                created_at: None,
                file_extension: None,
                duplicate_count: 0,
            },
            CodeDebtItem {
                file_path: PathBuf::from("test.rs"),
                line_number: 3,
                column: 1,
                line_content: "// FIXME: broken".to_string(),
                pattern_type: "FIXME".to_string(),
                severity: Severity::High,
                author: None,
                age_days: None,
                commit_hash: None,
                created_at: None,
                file_extension: None,
                duplicate_count: 0,
            },
        ];

        let scanner = CodeDebtScanner::new();
        let summary = scanner.get_summary(&items);

        assert_eq!(summary.get("TODO"), Some(&2));
        assert_eq!(summary.get("FIXME"), Some(&1));
    }

    #[test]
    fn test_filter_by_severity() {
        let items = vec![
            CodeDebtItem {
                file_path: PathBuf::from("test.rs"),
                line_number: 1,
                column: 1,
                line_content: "// TODO: test".to_string(),
                pattern_type: "TODO".to_string(),
                severity: Severity::Medium,
                author: None,
                age_days: None,
                commit_hash: None,
                created_at: None,
                file_extension: None,
                duplicate_count: 0,
            },
            CodeDebtItem {
                file_path: PathBuf::from("test.rs"),
                line_number: 2,
                column: 1,
                line_content: "// HACK: critical".to_string(),
                pattern_type: "HACK".to_string(),
                severity: Severity::Critical,
                author: None,
                age_days: None,
                commit_hash: None,
                created_at: None,
                file_extension: None,
                duplicate_count: 0,
            },
            CodeDebtItem {
                file_path: PathBuf::from("test.rs"),
                line_number: 3,
                column: 1,
                line_content: "// mock data".to_string(),
                pattern_type: "MOCK_STUB".to_string(),
                severity: Severity::Low,
                author: None,
                age_days: None,
                commit_hash: None,
                created_at: None,
                file_extension: None,
                duplicate_count: 0,
            },
        ];

        let scanner = CodeDebtScanner::new();

        // Filter for high and above
        let high_items = scanner.filter_by_severity(&items, Severity::High);
        assert_eq!(high_items.len(), 1); // Only the HACK item
        assert_eq!(high_items[0].pattern_type, "HACK");

        // Filter for medium and above
        let medium_items = scanner.filter_by_severity(&items, Severity::Medium);
        assert_eq!(medium_items.len(), 2); // HACK and TODO
    }

    #[test]
    fn test_scan_real_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files
        create_test_file(
            temp_dir.path(),
            "test.rs",
            "// TODO: implement\nfn main() {\n    // FIXME: broken\n    println!(\"test\");\n}",
        );

        create_test_file(
            temp_dir.path(),
            "test.py",
            "# TODO: add error handling\ndef test():\n    # HACK: quick fix\n    pass",
        );

        // Create a file with unsupported extension (should be ignored)
        create_test_file(temp_dir.path(), "test.txt", "TODO: this should be ignored");

        let scanner = CodeDebtScanner::new();
        let items = scanner.scan(temp_dir.path()).unwrap();

        // Should find 4 items (2 from .rs file, 2 from .py file, 0 from .txt file)
        assert_eq!(items.len(), 4);

        // Check that all items have valid file paths
        for item in &items {
            assert!(item.file_path.exists());
            assert!(item.line_number > 0);
            assert!(item.column > 0);
            assert!(!item.line_content.is_empty());
        }
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical < Severity::High);
        assert!(Severity::High < Severity::Medium);
        assert!(Severity::Medium < Severity::Low);
    }

    #[test]
    fn test_case_insensitive_patterns() {
        let test_content = r#"
// todo: lowercase
// TODO: uppercase
// ToDo: mixed case
// FIXME: test
// fixme: lowercase
"#;

        let scanner = CodeDebtScanner::new();
        let file_path = Path::new("test.rs");
        let items = CodeDebtScanner::scan_content(file_path, test_content, &scanner.patterns);

        let todo_items: Vec<_> = items
            .iter()
            .filter(|item| item.pattern_type == "TODO")
            .collect();
        let fixme_items: Vec<_> = items
            .iter()
            .filter(|item| item.pattern_type == "FIXME")
            .collect();

        assert_eq!(todo_items.len(), 3); // All variations of TODO
        assert_eq!(fixme_items.len(), 2); // All variations of FIXME
    }
}
