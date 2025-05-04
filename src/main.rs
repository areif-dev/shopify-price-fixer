use std::fs;
use std::path::PathBuf;

use clap::Parser;
use shopify_price_fixer::product::{abc_products_to_nmr_csv, map_upcs};
use shopify_price_fixer::{self as fixer, product};

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

    let log_to_stdout = !cli.write_logs;
    let config = match shopify_price_fixer::Config::read_config(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            fixer::log(
                log_to_stdout,
                fixer::Log::Error,
                format!(
                    "Encountered {} while trying to read config file at {:?}",
                    e, &cli.config
                ),
            )?;
            return Err(e)?;
        }
    };
    let abc_products = match product::parse_abc_item_files(&item_data_path, &posted_data_path) {
        Ok(p) => p,
        Err(e) => {
            fixer::log(
                log_to_stdout,
                fixer::Log::Error,
                format!(
                    "Failed to parse abc products from data files with error: {}",
                    e
                ),
            )?;
            return Err(e)?;
        }
    };
    let upc_map = map_upcs(&abc_products);
    let existing_nmr_products = 

    let mut nmr_abc_products = Vec::new();
    for shopify_product in shopify_products {
        if !&shopify_product.is_active {
            continue;
        }

        nmr_abc_products.push(
            match abc_products.get(&shopify_product.sku) {
                Some(p) => p,
                None => {
                    let barcode = match &shopify_product.barcode {
                        Some(u) => u.to_string(),
                        None => "".to_string(),
                    };
                    match upc_map.get(&barcode) {
                        Some((dup, product)) => {
                            if *dup {
                                fixer::log(
                                    log_to_stdout,
                                    fixer::Log::DuplicateAbcUpcs,
                                    format!("DUPLICATE UPC {:?}", &shopify_product),
                                )?;
                                continue;
                            } else {
                                product
                            }
                        }
                        None => {
                            fixer::log(
                                log_to_stdout,
                                fixer::Log::NotFound,
                                format!("NOT FOUND {:?}", &shopify_product),
                            )?;
                            continue;
                        }
                    }
                }
            }
            .to_owned(),
        );
        abc_products_to_nmr_csv(nmr_abc_products.as_slice()).unwrap();
    }

    Ok(())
}
