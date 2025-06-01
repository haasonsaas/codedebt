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

```
🔍 18 code debt items found:

🚨 CRITICAL PRODUCTION_DEBT ./src/config.rs:42:15 placeholder="Production API Key"
⚠️  HIGH TEMPORARY ./src/auth.rs:123:8 // Temporary workaround for OAuth
📝 MEDIUM TODO ./src/utils.rs:67:4 // TODO: Add proper error handling
💡 LOW MOCK_STUB ./tests/setup.rs:12:8 Mock database connection

⚡ Scanned in 5.17ms
```

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