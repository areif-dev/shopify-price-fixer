use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use clap::Parser;
use ean13::Ean13;
use shopify_price_fixer::product::{abc_products_to_nmr_csv, map_upcs};
use shopify_price_fixer::{self as fixer, product};

fn fetch_existing_upcs(file: &PathBuf) -> Result<Vec<Ean13>, std::io::Error> {
    let text = fs::read_to_string(file)?;
    let existing_set: HashSet<&str> = text.lines().collect();
    let mut existing = Vec::new();
    for elem in existing_set {
        let trimmed = elem.trim();
        existing.push(Ean13::from_str(trimmed).map_err(|e| std::io::Error::other(e))?);
    }

    Ok(existing)
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
    match fs::remove_dir_all(parent_dir.join("logs")) {
        _ => (),
    }

    let abc_products = product::parse_abc_item_files(&item_data_path, &posted_data_path)?;
    let upc_map = map_upcs(&abc_products);
    let previously_uploaded_upcs = fetch_existing_upcs(&cli.existing_upcs)?;

    let mut missing_upcs = Vec::new();
    let mut acceptable_products = Vec::new();
    for ean in previously_uploaded_upcs {
        let (dups, product) = match upc_map.get(&ean) {
            Some(p) => p,
            None => {
                missing_upcs.push(ean.clone());
                eprintln!("MISSING UPC: {:?}", ean);
                continue;
            }
        };

        // Many "duplicate" upcs will exist for the exact same ABC inventory listing because the
        // UPC will be entered both with and without a check digit. This block removes those false
        // duplicates
        let dup_skus: HashSet<String> = dups.iter().map(|dup| dup.sku()).collect();
        if dup_skus.len() > 1 {
            eprintln!("DUPLICATE UPC: {:?}", dups);
            missing_upcs.push(ean);
            continue;
        }
        acceptable_products.push(product.clone());
    }

    let written_products = abc_products_to_nmr_csv(acceptable_products.into_iter())?
        .iter()
        .map(|p| p.upc.to_string())
        .collect::<Vec<String>>()
        .join("\n");
    fs::write("existing.txt", written_products.as_bytes())?;

    Ok(())
}
