# CodeDebt 🚀

Ultra-fast code debt detection library and CLI tool written in Rust.

## Features

- **Blazingly Fast**: Parallel file scanning with Rust performance
- **Smart Patterns**: Detects TODO, FIXME, HACK, XXX, and more
- **Severity Levels**: Critical, High, Medium, Low classification
- **Multiple Formats**: Pretty, JSON, CSV output
- **Library + CLI**: Use as library or standalone tool
- **Git-aware**: Respects .gitignore automatically
- **Enhanced Intelligence**: Git blame integration, duplicate detection, file type analysis
- **Advanced Filtering**: Filter by age, duplicates, severity, and file types

## Installation

```bash
cargo install codedebt
```

Or build from source:
```bash
git clone https://github.com/haasonsaas/codedebt
cd codedebt
cargo build --release
```

## CLI Usage

### Basic Usage
```bash
# Scan current directory
codedebt

# Scan specific directory
codedebt /path/to/project

# Show only critical and high severity
codedebt --severity high

# Output as JSON
codedebt --format json

# Show summary only
codedebt --summary

# Custom file extensions
codedebt --extensions "rs,py,js"

# Ignore additional directories
codedebt --ignore "vendor,tmp"
```

### Enhanced Intelligence Features
```bash
# Enable git blame integration (shows author, age, commit info)
codedebt --git-blame

# Detect duplicate patterns across files
codedebt --detect-duplicates

# Show file type distribution
codedebt --file-types

# Show age distribution of code debt
codedebt --git-blame --age-distribution

# Filter by maximum age (requires git blame)
codedebt --git-blame --max-age 30

# Show only duplicates with minimum count
codedebt --detect-duplicates --min-duplicates 3

# Combine multiple intelligence features
codedebt --git-blame --detect-duplicates --file-types --age-distribution
```

### Getting Help
```bash
# Get help
codedebt --help
```

## Library Usage

```rust
use codedebt::{CodeDebtScanner, Severity, Pattern};
use regex::Regex;

// Basic usage
let scanner = CodeDebtScanner::new();
let items = scanner.scan("./src")?;

// Enhanced intelligence features
let scanner = CodeDebtScanner::new()
    .with_git_blame(true)                    // Enable git blame integration
    .with_duplicate_detection(true)          // Enable duplicate detection
    .with_file_extensions(vec!["rs".to_string(), "py".to_string()]);

let all_items = scanner.scan(".")?;

// Use enhanced methods
let summary = scanner.get_summary(&all_items);
let file_types = scanner.get_file_type_summary(&all_items);
let age_distribution = scanner.get_age_distribution(&all_items);

// Apply filters
let recent_items = scanner.filter_by_age(&all_items, 30);  // Last 30 days
let duplicates = scanner.find_duplicates(&all_items, 2);   // 2+ occurrences

// Custom patterns
let custom_patterns = vec![
    Pattern {
        name: "URGENT".to_string(),
        regex: Regex::new(r"(?i)\bURGENT\b").unwrap(),
        severity: Severity::Critical,
    },
];

let scanner = CodeDebtScanner::new().with_patterns(custom_patterns);
```

## Performance

- **Lightning fast** - scans large codebases in milliseconds
- **Parallel processing** of files using Rayon
- **Memory efficient** streaming with ignore crate
- **Optimized regex** compilation and matching

Example performance on codedebt project (57 items found in 3.6ms):
```
⚡ Scanned in 3.57ms
```

## Patterns Detected

| Pattern | Severity | Description |
|---------|----------|-------------|
| HACK, XXX | Critical | Code that needs immediate attention |
| PRODUCTION_DEBT | Critical | Temporary/placeholder code in production context |
| FIXME | High | Broken code that needs fixing |
| TEMPORARY, PLACEHOLDER | High | Code not meant for production |
| TODO | Medium | Future improvements |
| NOTE.*fix | Medium | Notes about things that need fixing |
| MOCK, STUB | Low | Test or development code |

## Example Output

### Basic Output
```
🔍 18 code debt items found:

🚨 CRITICAL PRODUCTION_DEBT ./src/config.rs:42:15 placeholder="Production API Key"
⚠️  HIGH TEMPORARY ./src/auth.rs:123:8 // Temporary workaround for OAuth
📝 MEDIUM TODO ./src/utils.rs:67:4 // TODO: Add proper error handling
💡 LOW MOCK_STUB ./tests/setup.rs:12:8 Mock database connection

⚡ Scanned in 5.17ms
```

### Enhanced Intelligence Output
```
🔍 12 code debt items found:

🚨 CRITICAL HACK ./src/auth.rs:45:8 // HACK: bypass validation for demo
    👤 john.doe@company.com • 📅 3 days ago • 🔄 2 duplicates

⚠️  HIGH FIXME ./src/utils.rs:123:4 // FIXME: memory leak in parser
    👤 jane.smith@company.com • 📅 2 weeks ago

📝 MEDIUM TODO ./src/config.rs:67:4 // TODO: add environment validation
    👤 bob.wilson@company.com • 📅 1 month ago • 🔄 5 duplicates

📊 File Type Distribution:
════════════════════════════════════════
rs                 8
js                 3
py                 1

📅 Age Distribution:
════════════════════════════════════════
This week           2
This month          7
Last 3 months       3

⚡ Scanned in 8.42ms
```

## Enhanced Intelligence Features

### Git Blame Integration
- **Author tracking**: See who created each piece of code debt
- **Age analysis**: Understand how old technical debt is
- **Commit context**: Link debt to specific commits for deeper investigation
- **Age filtering**: Focus on recent or old debt with `--max-age`

### Duplicate Detection
- **Pattern matching**: Find repeated code debt across your codebase
- **Count tracking**: See how many times each pattern appears
- **Duplicate filtering**: Focus on the most common issues with `--min-duplicates`
- **Team awareness**: Identify systemic problems affecting multiple files

### File Type Analysis
- **Language breakdown**: See which programming languages have the most debt
- **Technology debt**: Understand debt distribution across your tech stack
- **Targeting improvements**: Focus refactoring efforts on the most problematic file types

### Age Distribution
- **Time-based insights**: Understand debt accumulation patterns over time
- **Project timeline**: See how debt relates to project milestones
- **Priority guidance**: Focus on recent debt that's easier to fix or old debt that's become critical

## Supported Languages

Rust, Python, JavaScript, TypeScript, Go, Java, C/C++, Ruby, PHP, C#, Swift, Kotlin, Scala, and more.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development

```bash
# Clone the repository
git clone https://github.com/haasonsaas/codedebt
cd codedebt

# Build and test
cargo build
cargo test

# Run on a sample project
cargo run -- /path/to/test/project
```

## License

MIT - see LICENSE file for details