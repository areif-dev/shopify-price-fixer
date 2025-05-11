use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use clap::Parser;
use ean13::Ean13;
use serde::Deserialize;
use shopify_price_fixer::product::{abc_products_to_nmr_csv, map_upcs};
use shopify_price_fixer::{self as fixer, product};

#[derive(Debug, Deserialize)]
struct CsvRecord {
    upc: Ean13,
}

fn fetch_existing_upcs<R>(rdr: R) -> Result<Vec<Ean13>, csv::Error>
where
    R: std::io::Read,
{
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .from_reader(rdr);
    let mut existing = Vec::new();
    for result in rdr.deserialize() {
        let record: CsvRecord = result?;
        existing.push(record.upc);
    }
    Ok(existing)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = fixer::Cli::parse();
    let item_data_path = cli.item_data;
    let posted_data_path = cli.posted_data;
    let existing_csv = cli.existing_products;

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
    let rdr = std::fs::File::open(existing_csv)?;
    let previously_uploaded_upcs = fetch_existing_upcs(rdr)?;

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

    abc_products_to_nmr_csv(acceptable_products.into_iter()).unwrap();

    Ok(())
}
