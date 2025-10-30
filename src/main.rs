use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use abc_product::AbcProduct;
use clap::Parser;
use ean13::Ean13;
use shopify_price_fixer::product::{abc_products_to_nmr_csv, map_upcs};
use shopify_price_fixer::{self as fixer, product};

fn fetch_existing_skus(file: &PathBuf) -> Result<HashSet<String>, std::io::Error> {
    let text = fs::read_to_string(file)?;
    Ok(text.lines().map(|l| l.trim().to_string()).collect())
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

    let abc_products = AbcProduct::from_db_export(&item_data_path, &posted_data_path)?;
    let previously_uploaded_skus = fetch_existing_skus(&cli.existing_upcs)?;

    let mut missing_skus = Vec::new();
    let mut acceptable_products = Vec::new();
    for sku in previously_uploaded_skus {
        let product = match abc_products.get(&sku) {
            Some(p) => p,
            None => {
                missing_skus.push(sku.clone());
                eprintln!("MISSING UPC: {:?}", sku);
                continue;
            }
        };
        acceptable_products.push(product.clone());
    }

    let written_products = abc_products_to_nmr_csv(acceptable_products.into_iter())?
        .iter()
        .map(|p| p.sku.clone())
        .collect::<Vec<String>>()
        .join("\n");
    fs::write("existing.txt", written_products.as_bytes())?;

    Ok(())
}
