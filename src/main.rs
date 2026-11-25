use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use abc_product::AbcProduct;
use clap::Parser;
use shopify_price_fixer::product::{match_identifiers, parse_identifiers};
use shopify_price_fixer::{self as fixer, BarcodeWrapper};

/// Reads export identifiers from a newline-separated file.
///
/// Blank lines are ignored and repeated identifiers are collapsed. Each line is
/// parsed as a GTIN when possible, otherwise it is treated as an ABC SKU.
///
/// # Arguments
/// * `file` - The path to the export identifiers file.
///
/// # Returns
/// The unique GTIN or SKU identifiers from the file.
///
/// # Errors
/// Returns an [`std::io::Error`] if the file cannot be read.
fn fetch_export_identifiers(file: &PathBuf) -> Result<HashSet<BarcodeWrapper>, std::io::Error> {
    let text = fs::read_to_string(file)?;
    Ok(parse_identifiers(&text))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = fixer::Cli::parse();
    let item_data_path = cli.item_data;
    let posted_data_path = cli.posted_data;

    // Something is probably very wrong if the binary has no parent directory, but if it doesn't,
    // switch everything to use the current working directory to be safe(r)
    let parent_dir = match std::env::current_exe()?.parent() {
        Some(p) => p.to_owned(),
        None => PathBuf::from("."),
    };

    // Attempt to remove any existing log files. If no logs exist, ignore the resulting error
    let _ = fs::remove_dir_all(parent_dir.join("logs"));

    let abc_products = AbcProduct::from_db_export(&item_data_path, &posted_data_path)?;
    let export_identifiers = fetch_export_identifiers(&cli.export_identifiers)?;
    let matches = match_identifiers(&abc_products, &export_identifiers);

    let logs_dir = parent_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;

    if !matches.missing.is_empty() {
        let contents: String = matches
            .missing
            .iter()
            .map(|identifier| format!("{identifier}\n"))
            .collect();
        fs::write(logs_dir.join("missing-identifiers.txt"), contents)?;
        for identifier in &matches.missing {
            eprintln!("MISSING IDENTIFIER: {identifier}");
        }
    }

    if !matches.duplicate_gtins.is_empty() {
        let contents = format!("{:#?}\n", matches.duplicate_gtins);
        fs::write(logs_dir.join("duplicate-gtins.txt"), contents)?;
        for (gtin, products) in &matches.duplicate_gtins {
            eprintln!(
                "DUPLICATE GTIN: {gtin} is shared by {} ABC products",
                products.len()
            );
        }
    }

    let _acceptable_products = matches.matched;

    Ok(())
}
