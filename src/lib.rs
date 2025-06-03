pub mod error;
pub mod git;
pub mod interactive;
pub mod models;
pub mod patterns;
pub mod progress;
pub mod scanner;
pub mod watch;

pub use models::{CodeDebtItem, Severity};
pub use patterns::Pattern;
pub use scanner::CodeDebtScanner;

#[cfg(test)]
mod tests {
    use crate::models::{CodeDebtItem, Severity};
    use crate::patterns::Pattern;
    use crate::scanner::CodeDebtScanner;
    use regex::Regex;
    use std::fs;
    use std::path::{Path, PathBuf};
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
