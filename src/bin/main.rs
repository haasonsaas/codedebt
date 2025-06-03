use clap::{Parser, ValueEnum};
use codedebt::{CodeDebtScanner, Severity};
use colored::*;
use glob::glob;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "codedebt")]
#[command(about = "Ultra-fast code debt detection tool")]
#[command(version)]
struct Cli {
    /// Directory or glob pattern to scan
    #[arg(default_value = ".")]
    path: String,

    /// Minimum severity level to show
    #[arg(short, long, value_enum, default_value = "low")]
    severity: SeverityArg,

    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    format: OutputFormat,

    /// Show summary only
    #[arg(short = 'S', long)]
    summary: bool,

    /// File extensions to scan (comma-separated)
    #[arg(short, long)]
    extensions: Option<String>,

    /// Additional directories to ignore (comma-separated)
    #[arg(short, long)]
    ignore: Option<String>,

    /// Enable git blame integration for age detection
    #[arg(long)]
    git_blame: bool,

    /// Enable duplicate pattern detection
    #[arg(long)]
    detect_duplicates: bool,

    /// Show file type distribution
    #[arg(long)]
    file_types: bool,

    /// Show age distribution (requires --git-blame)
    #[arg(long)]
    age_distribution: bool,

    /// Filter by maximum age in days (requires --git-blame)
    #[arg(long)]
    max_age: Option<i64>,

    /// Show only duplicates with minimum count
    #[arg(long)]
    min_duplicates: Option<usize>,

    /// Enable watch mode
    #[arg(short, long)]
    watch: bool,

    /// Enable interactive mode
    #[arg(short = 'I', long)]
    interactive: bool,

    /// Show progress indicator for large repositories
    #[arg(long)]
    progress: bool,
}

#[derive(Clone, ValueEnum)]
enum SeverityArg {
    Critical,
    High,
    Medium,
    Low,
}

impl From<SeverityArg> for Severity {
    fn from(arg: SeverityArg) -> Self {
        match arg {
            SeverityArg::Critical => Severity::Critical,
            SeverityArg::High => Severity::High,
            SeverityArg::Medium => Severity::Medium,
            SeverityArg::Low => Severity::Low,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Pretty,
    Json,
    Csv,
}

fn main() -> anyhow::Result<()> {
    // Initialize logger if RUST_LOG env var is set
    env_logger::init();

    let cli = Cli::parse();

    // Handle glob patterns
    let paths = resolve_paths(&cli.path)?;
    if paths.is_empty() {
        return Err(codedebt::error::handle_path_error(&cli.path).into());
    }

    let mut scanner = CodeDebtScanner::new();

    if let Some(extensions) = cli.extensions {
        let exts: Vec<String> = extensions
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        scanner = scanner.with_file_extensions(exts);
    }

    if let Some(ignore_dirs) = cli.ignore {
        let dirs: Vec<String> = ignore_dirs
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        scanner = scanner.with_ignore_dirs(dirs);
    }

    // Configure enhanced intelligence features
    if cli.git_blame {
        scanner = scanner.with_git_blame(true);
    }

    if cli.detect_duplicates {
        scanner = scanner.with_duplicate_detection(true);
    }

    // Add progress reporter if requested
    if cli.progress && !cli.watch && !cli.interactive {
        scanner = scanner.with_progress_reporter(Box::new(
            codedebt::progress::TerminalProgressReporter::new(true),
        ));
    }

    // Handle watch mode
    if cli.watch {
        let watch_paths: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let watcher = codedebt::watch::CodeDebtWatcher::new(scanner, watch_paths);
        return watcher.watch();
    }

    let start = std::time::Instant::now();
    let mut all_items = Vec::new();

    // Scan all paths
    for path in &paths {
        match scanner.scan(path) {
            Ok(items) => all_items.extend(items),
            Err(e) => eprintln!("Error scanning {}: {}", path.display(), e),
        }
    }

    let duration = start.elapsed();

    // Apply filters
    let mut filtered_items = scanner.filter_by_severity(&all_items, cli.severity.into());

    // Apply age filter if specified
    if let Some(max_age) = cli.max_age {
        if !cli.git_blame {
            eprintln!("Warning: --max-age requires --git-blame to be enabled");
        } else {
            filtered_items = scanner.filter_by_age(&filtered_items, max_age);
        }
    }

    // Apply duplicate filter if specified
    if let Some(min_duplicates) = cli.min_duplicates {
        if !cli.detect_duplicates {
            eprintln!("Warning: --min-duplicates requires --detect-duplicates to be enabled");
        } else {
            filtered_items = scanner.find_duplicates(&filtered_items, min_duplicates);
        }
    }

    // Handle interactive mode
    if cli.interactive {
        let mut interactive = codedebt::interactive::InteractiveMode::new(filtered_items);
        return interactive.run();
    }

    match cli.format {
        OutputFormat::Pretty => {
            if cli.summary {
                print_summary(&scanner, &filtered_items);
            } else {
                print_pretty(&filtered_items);
            }

            // Show additional information if requested
            if cli.file_types {
                print_file_type_distribution(&scanner, &all_items);
            }

            if cli.age_distribution {
                if !cli.git_blame {
                    eprintln!("Warning: --age-distribution requires --git-blame to be enabled");
                } else {
                    print_age_distribution(&scanner, &all_items);
                }
            }

            println!(
                "\n{} Scanned {} in {:.2}ms",
                "⚡".bright_yellow(),
                format_paths(&paths),
                duration.as_secs_f64() * 1000.0
            );
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&filtered_items)?);
        }
        OutputFormat::Csv => {
            print_csv(&filtered_items);
        }
    }

    Ok(())
}

fn resolve_paths(pattern: &str) -> anyhow::Result<Vec<PathBuf>> {
    // Check if it's a glob pattern
    if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
        let mut paths = HashSet::new();
        for path in glob(pattern)?.flatten() {
            if path.is_dir() {
                paths.insert(path);
            } else if path.is_file() {
                // For files, add their parent directory
                if let Some(parent) = path.parent() {
                    paths.insert(parent.to_path_buf());
                }
            }
        }
        Ok(paths.into_iter().collect())
    } else {
        // Regular path
        let path = PathBuf::from(pattern);
        if path.exists() && path.is_dir() {
            Ok(vec![path])
        } else if path.exists() && path.is_file() {
            // For a single file, scan its parent directory
            if let Some(parent) = path.parent() {
                Ok(vec![parent.to_path_buf()])
            } else {
                Ok(vec![])
            }
        } else {
            Ok(vec![])
        }
    }
}

fn format_paths(paths: &[PathBuf]) -> String {
    if paths.len() == 1 {
        paths[0].display().to_string()
    } else {
        format!("{} directories", paths.len())
    }
}

fn print_pretty(items: &[codedebt::CodeDebtItem]) {
    if items.is_empty() {
        println!("{} No code debt found!", "✅".green());
        return;
    }

    println!("{} {} code debt items found:\n", "🔍".cyan(), items.len());

    for item in items {
        let severity_icon = match item.severity {
            Severity::Critical => "🚨".red(),
            Severity::High => "⚠️ ".yellow(),
            Severity::Medium => "📝".blue(),
            Severity::Low => "💡".white(),
        };

        let severity_text = match item.severity {
            Severity::Critical => "CRITICAL".red().bold(),
            Severity::High => "HIGH".yellow().bold(),
            Severity::Medium => "MEDIUM".blue().bold(),
            Severity::Low => "LOW".white().bold(),
        };

        println!(
            "{} {} {} {}:{}:{} {}",
            severity_icon,
            severity_text,
            item.pattern_type.purple().bold(),
            item.file_path.display().to_string().cyan(),
            item.line_number.to_string().green(),
            item.column.to_string().green(),
            item.line_content.trim()
        );

        // Add enhanced information if available
        let mut details = Vec::new();

        if let Some(author) = &item.author {
            details.push(format!("👤 {}", author.dimmed()));
        }

        if let Some(age_days) = item.age_days {
            let age_str = if age_days == 0 {
                "today".to_string()
            } else if age_days == 1 {
                "1 day ago".to_string()
            } else if age_days < 30 {
                format!("{} days ago", age_days)
            } else if age_days < 365 {
                format!("{} months ago", age_days / 30)
            } else {
                format!("{} years ago", age_days / 365)
            };
            details.push(format!("📅 {}", age_str.dimmed()));
        }

        if item.duplicate_count > 1 {
            details.push(format!(
                "🔄 {} duplicates",
                item.duplicate_count.to_string().yellow()
            ));
        }

        if !details.is_empty() {
            println!("    {}", details.join(" • "));
        }
    }
}

fn print_summary(scanner: &CodeDebtScanner, items: &[codedebt::CodeDebtItem]) {
    let summary = scanner.get_summary(items);

    println!("{} Code Debt Summary:", "📊".cyan());
    println!("{}", "=".repeat(40));

    let mut total = 0;
    for (pattern, count) in &summary {
        println!("{:15} {:>5}", pattern.purple(), count.to_string().yellow());
        total += count;
    }

    println!("{}", "-".repeat(40));
    println!(
        "{:15} {:>5}",
        "TOTAL".bold(),
        total.to_string().bold().yellow()
    );
}

fn print_csv(items: &[codedebt::CodeDebtItem]) {
    println!("file_path,line_number,column,severity,pattern_type,line_content,author,age_days,duplicate_count");
    for item in items {
        println!(
            "{},{},{},{:?},{},\"{}\",\"{}\",{},{}",
            item.file_path.display(),
            item.line_number,
            item.column,
            item.severity,
            item.pattern_type,
            item.line_content.replace('"', "\"\""),
            item.author.as_deref().unwrap_or(""),
            item.age_days.unwrap_or(-1),
            item.duplicate_count
        );
    }
}

fn print_file_type_distribution(scanner: &CodeDebtScanner, items: &[codedebt::CodeDebtItem]) {
    let distribution = scanner.get_file_type_summary(items);
    if distribution.is_empty() {
        return;
    }

    println!("\n{} File Type Distribution:", "📊".cyan());
    println!("{}", "═".repeat(40));

    let mut sorted_types: Vec<_> = distribution.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));

    for (file_type, count) in sorted_types {
        println!(
            "{:15} {:>5}",
            file_type.purple(),
            count.to_string().yellow()
        );
    }
}

fn print_age_distribution(scanner: &CodeDebtScanner, items: &[codedebt::CodeDebtItem]) {
    let distribution = scanner.get_age_distribution(items);
    if distribution.is_empty() {
        return;
    }

    println!("\n{} Age Distribution:", "📅".cyan());
    println!("{}", "═".repeat(40));

    // Define order for age buckets
    let order = [
        "This week",
        "This month",
        "Last 3 months",
        "This year",
        "Over a year",
        "Unknown age",
    ];

    for bucket in order.iter() {
        if let Some(count) = distribution.get(*bucket) {
            println!("{:15} {:>5}", bucket.purple(), count.to_string().yellow());
        }
    }
}
