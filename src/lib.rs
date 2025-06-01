use anyhow::Result;
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
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "cc", "cxx",
            "h", "hpp", "rb", "php", "cs", "swift", "kt", "scala", "clj", "ml", "hs",
            "elm", "dart", "lua", "pl", "r", "jl", "nim", "zig", "v", "cr",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let ignore_dirs = vec![
            "node_modules", ".git", "target", "dist", "build", ".next", "vendor",
            "__pycache__", ".pytest_cache", "coverage", ".nyc_output", "bower_components",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self {
            patterns,
            file_extensions,
            ignore_dirs,
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
        results.sort_by(|a, b| {
            a.severity.cmp(&b.severity)
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

    pub fn filter_by_severity(&self, items: &[CodeDebtItem], min_severity: Severity) -> Vec<CodeDebtItem> {
        items
            .iter()
            .filter(|item| item.severity <= min_severity)
            .cloned()
            .collect()
    }
}