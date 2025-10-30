use crate::FixerError;
use abc_product::AbcProduct;
use chrono::Datelike;
use ean13::Ean13;
use rust_decimal::Decimal;
use serde::Serialize;
use std::{collections::HashMap, fs::File};

pub type DuplicateProducts = Vec<AbcProduct>;

pub fn map_upcs(
    existing_map: &HashMap<String, AbcProduct>,
) -> HashMap<Ean13, (DuplicateProducts, AbcProduct)> {
    let mut upc_map = HashMap::new();
    for (_sku, product) in existing_map {
        for upc in product.upcs().iter() {
            if let Some((dup, prod)) = upc_map.insert(upc.clone(), (Vec::new(), product.to_owned()))
            {
                let mut dup = dup;
                if product.sku() != prod.sku() {
                    dup.push(product.to_owned());
                    dup.push(prod.clone());
                }
                upc_map.insert(upc.clone(), (dup, prod));
            }
        }
    }
    upc_map
}

pub fn abc_products_to_nmr_csv<I>(products: I) -> Result<Vec<NmrProduct>, FixerError>
where
    I: Iterator<Item = AbcProduct>,
{
    let file = File::create("nmr.csv").map_err(|e| {
        FixerError::Custom(format!("Failed to create nmr.csv file because of {:?}", e))
    })?;
    let mut wtr = csv::Writer::from_writer(file);
    let mut nmr_products = Vec::new();
    for product in products {
        let now = chrono::Local::now().date_naive();
        let five_years_ago = now.with_year((now.year_ce().1 - 5) as i32).unwrap();

        // Skip any product that either has never sold or has not sold for over 5 years
        match product.last_sold() {
            None => continue,
            Some(d) => {
                if d < five_years_ago {
                    continue;
                }
            }
        }
        let nmr_product = match NmrProduct::try_from(product.clone()) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{:?}", e);
                continue;
            }
        };
        wtr.serialize(&nmr_product).map_err(|e| {
            FixerError::Custom(format!(
                "Failed to serialize {:?} due to '{}'",
                &nmr_product, e
            ))
        })?;
        nmr_products.push(nmr_product);
    }
    wtr.flush().map_err(|e| {
        FixerError::Custom(format!(
            "Failed to flush csv writer to nmr.csv because of {:?}",
            e
        ))
    })?;

    // Uploading an empty file to Catalyst will result in the whole catalog being removed, so
    // remove the whole data file in that case
    if nmr_products.is_empty() {
        std::fs::remove_file("nmr.csv").map_err(|e| {
            FixerError::Custom(format!(
                "nmr_products is empty, but the data file could not be deleted due to `{}`",
                e
            ))
        })?;
    }
    Ok(nmr_products)
}

/// Convert \' and \" chars into ft and in. abbreviations, respectively
///
/// # Arguments
///
/// * `raw` - The raw string to fix abbreviations on
///
/// # Returns
///
/// The `raw` string with all \' and \" characters swapped for ft. and in. abbreviations
fn quotes_to_distance(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let mut res = String::with_capacity(chars.len());
    while i < chars.len() - 1 {
        match (chars[i], chars[i + 1]) {
            ('\'', '\'') => {
                res.push_str("ft.");
                i += 2;
                continue;
            }
            ('\"', '\"') => {
                res.push_str("in.");
                i += 2;
                continue;
            }
            ('\'', _) => {
                res.push_str("ft.");
            }
            ('\"', _) => {
                res.push_str("in.");
            }
            _ => res.push(chars[i]),
        }
        i += 1;
    }
    res
}

#[derive(Debug, Serialize)]
pub struct NmrProduct {
    name: String,
    upc: Option<Ean13>,
    price: String,
    qty: String,
    pub sku: String,
    weight: f64,
}

impl TryFrom<AbcProduct> for NmrProduct {
    type Error = FixerError;
    fn try_from(value: AbcProduct) -> Result<Self, Self::Error> {
        let upc = value.upcs().get(0).cloned();
        let qty = value.stock() as i64;
        let qty = if qty >= 0 {
            qty.to_string()
        } else {
            "0".to_string()
        };
        let name = if value.desc().len() > 0 {
            quotes_to_distance(&value.desc())
                .chars()
                .filter(|c| *c != '\\' && *c != ',')
                .collect()
        } else {
            return Err(FixerError::Custom(format!("Missing desc for {:?}", value)))?;
        };
        Ok(NmrProduct {
            name,
            sku: value.sku(),
            upc,
            price: (value.list() / Decimal::new(100, 0)).to_string(),
            qty,
            weight: value.weight().ok_or(FixerError::Custom(format!(
                "Missing weight for {:?}",
                value
            )))?,
        })
    }
}
