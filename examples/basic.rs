use codedebt::{CodeDebtScanner, Severity};

fn main() -> anyhow::Result<()> {
    // Create scanner with default patterns
    let scanner = CodeDebtScanner::new();
    
    // Scan current directory
    let items = scanner.scan(".")?;
    
    // Filter by severity
    let critical_items = scanner.filter_by_severity(&items, Severity::Critical);
    
    // Print results
    println!("Found {} total code debt items", items.len());
    println!("Found {} critical items", critical_items.len());
    
    // Get summary
    let summary = scanner.get_summary(&items);
    for (pattern_type, count) in summary {
        println!("{}: {}", pattern_type, count);
    }
    
    Ok(())
}