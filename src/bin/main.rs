use clap::{Parser, ValueEnum};
use codedebt::{CodeDebtScanner, Severity};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "codedebt")]
#[command(about = "Ultra-fast code debt detection tool")]
#[command(version)]
struct Cli {
    /// Directory to scan
    #[arg(default_value = ".")]
    path: PathBuf,

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
    let cli = Cli::parse();

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

    let start = std::time::Instant::now();
    let all_items = scanner.scan(&cli.path)?;
    let duration = start.elapsed();

    let items = scanner.filter_by_severity(&all_items, cli.severity.into());

    match cli.format {
        OutputFormat::Pretty => {
            if cli.summary {
                print_summary(&scanner, &items);
            } else {
                print_pretty(&items);
            }
            println!(
                "\n{} Scanned in {:.2}ms",
                "⚡".bright_yellow(),
                duration.as_secs_f64() * 1000.0
            );
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        OutputFormat::Csv => {
            print_csv(&items);
        }
    }

    Ok(())
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
    println!("file_path,line_number,column,severity,pattern_type,line_content");
    for item in items {
        println!(
            "{},{},{},{:?},{},\"{}\"",
            item.file_path.display(),
            item.line_number,
            item.column,
            item.severity,
            item.pattern_type,
            item.line_content.replace('"', "\"\"")
        );
    }
}
