# CodeDebt 🚀

Ultra-fast code debt detection library and CLI tool written in Rust.

## Features

- **Blazingly Fast**: Parallel file scanning with Rust performance
- **Smart Patterns**: Detects TODO, FIXME, HACK, XXX, and more
- **Severity Levels**: Critical, High, Medium, Low classification
- **Multiple Formats**: Pretty, JSON, CSV output
- **Library + CLI**: Use as library or standalone tool
- **Git-aware**: Respects .gitignore automatically

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

## Library Usage

```rust
use codedebt::{CodeDebtScanner, Severity, Pattern};
use regex::Regex;

// Basic usage
let scanner = CodeDebtScanner::new();
let items = scanner.scan("./src")?;

// Custom patterns
let custom_patterns = vec![
    Pattern {
        name: "URGENT".to_string(),
        regex: Regex::new(r"(?i)\bURGENT\b").unwrap(),
        severity: Severity::Critical,
    },
];

let scanner = CodeDebtScanner::new()
    .with_patterns(custom_patterns)
    .with_file_extensions(vec!["rs".to_string(), "py".to_string()]);

let items = scanner.scan(".")?;
let summary = scanner.get_summary(&items);
```

## Performance

- **10-100x faster** than grep-based solutions
- **Parallel processing** of files using Rayon
- **Memory efficient** streaming
- **Optimized regex** compilation

## Patterns Detected

| Pattern | Severity | Description |
|---------|----------|-------------|
| HACK, XXX | Critical | Code that needs immediate attention |
| FIXME | High | Broken code that needs fixing |
| TODO | Medium | Future improvements |
| TEMPORARY, PLACEHOLDER | High | Code not meant for production |
| MOCK, STUB | Low | Test or development code |

## Supported Languages

Rust, Python, JavaScript, TypeScript, Go, Java, C/C++, Ruby, PHP, C#, Swift, Kotlin, Scala, and more.

## License

MIT