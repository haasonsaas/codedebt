use crate::models::Severity;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub name: String,
    pub regex: Regex,
    pub severity: Severity,
}

impl Pattern {
    pub fn default_patterns() -> Vec<Pattern> {
        vec![
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
        ]
    }

    pub fn default_file_extensions() -> Vec<String> {
        vec![
            "rs", "py", "js", "ts", "jsx", "tsx", "go", "java", "c", "cpp", "cc", "cxx", "h",
            "hpp", "rb", "php", "cs", "swift", "kt", "scala", "clj", "ml", "hs", "elm", "dart",
            "lua", "pl", "r", "jl", "nim", "zig", "v", "cr",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    pub fn default_ignore_dirs() -> Vec<String> {
        vec![
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
        .collect()
    }
}
