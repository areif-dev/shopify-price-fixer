use crate::FixerError;
use chrono::Datelike;
use ean13::Ean13;
use serde::{ser::Error, Serialize};
use std::{collections::HashMap, fs::File, num::ParseFloatError};

fn price_from_str(price_str: &str) -> Result<i64, ParseFloatError> {
    let price_str: String = price_str
        .chars()
        .filter(|c| c.is_digit(10) || c == &'.')
        .collect();
    let fprice: f64 = price_str.parse()?;
    let iprice: i64 = (fprice * 100.0).round() as i64;
    Ok(iprice)
}

pub fn parse_abc_item_files(
    item_path: &str,
    posted_path: &str,
) -> Result<HashMap<String, AbcProduct>, csv::Error> {
    let mut item_data = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(item_path)?;
    let mut posted_data = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(posted_path)?;

    let mut i = 0;
    let mut products = HashMap::new();
    while let Some(row) = item_data.records().next() {
        i += 1;
        let row = row?;
        let sku = row
            .get(0)
            .ok_or(csv::Error::custom(format!(
                "Cannot deserialize sku in row {}",
                i
            )))?
            .to_string();
        let desc = row
            .get(1)
            .ok_or(csv::Error::custom(format!(
                "Cannot deserialize desc in row {}",
                i
            )))?
            .to_string();
        let upc_str: String = row
            .get(43)
            .ok_or(csv::Error::custom(format!(
                "Cannot fetch upcs in row {}",
                i
            )))?
            .chars()
            .filter(|c| c.is_digit(10) || *c == ',')
            .collect();
        let upcs: Vec<Ean13> = upc_str
            .split(",")
            .filter_map(|s| {
                if s.len() == 11 {
                    // Some ABC UPCs leave out the check digit, so make one up and let [`Ean13::from_str_nonstrict`] fix it
                    Ean13::from_str_nonstrict(&format!("{}0", s)).ok()
                } else if s.len() < 11 {
                    // Anything less than 11 characters long is probably a dead upc
                    None
                } else {
                    // Anything 12 characters and up has a chance of being a good upc
                    Ean13::from_str_nonstrict(s).ok()
                }
            })
            .collect();
        let list = row.get(6).ok_or(csv::Error::custom(format!(
            "Cannot fetch list price from row {}",
            i
        )))?;
        let list = price_from_str(list).or(Err(csv::Error::custom(format!(
            "Cannot parse a price in cents for list in row {}",
            i
        ))))?;
        let cost = row.get(8).ok_or(csv::Error::custom(format!(
            "Cannot fetch cost from row {}",
            i
        )))?;
        let cost = price_from_str(cost).or(Err(csv::Error::custom(format!(
            "Cannot parse a price in cents for cost in row {}",
            i
        ))))?;

        products.insert(
            sku.clone(),
            AbcProduct {
                sku,
                desc,
                upcs,
                list,
                cost,
                stock: 0.0,
                last_sold: None,
            },
        );
    }

    let mut i = 0;
    while let Some(row) = posted_data.records().next() {
        i += 1;
        let row = row?;
        let sku = row
            .get(0)
            .ok_or(csv::Error::custom(format!(
                "Cannot deserialize sku in row {} of posted items",
                i
            )))?
            .to_string();
        let stock_str = row
            .get(19)
            .ok_or(csv::Error::custom(format!(
                "Cannot deserialize stock in row {} of posted items",
                i
            )))?
            .to_string();
        let stock: f64 = stock_str.parse().or(Err(csv::Error::custom(format!(
            "Cannot parse f64 from stock_str in row {} of posted items",
            i
        ))))?;
        let last_sold_str: String = row
            .get(1)
            .ok_or(csv::Error::custom(format!(
                "Cannot deserialize last_sold in row {} of posted items",
                i
            )))?
            .to_string();
        let last_sold = chrono::NaiveDate::parse_from_str(&last_sold_str, "%Y-%m-%d").ok();
        let mut existing_record = products
            .get(&sku)
            .ok_or(csv::Error::custom(format!(
                "Cannot find existing product for item with sku {} in row {} of posted_data",
                &sku, i
            )))?
            .clone();
        existing_record.stock = stock;
        existing_record.sku = existing_record.sku.to_uppercase();
        existing_record.last_sold = last_sold;
        products.insert(sku, existing_record);
    }
    Ok(products)
}

pub type DuplicateProducts = Vec<AbcProduct>;

pub fn map_upcs(
    existing_map: &HashMap<String, AbcProduct>,
) -> HashMap<Ean13, (DuplicateProducts, AbcProduct)> {
    let mut upc_map = HashMap::new();
    for (_sku, product) in existing_map {
        for upc in product.upcs.iter() {
            if let Some((dup, prod)) = upc_map.insert(upc.clone(), (Vec::new(), product.to_owned()))
            {
                let mut dup = dup;
                dup.push(product.to_owned());
                dup.push(prod.clone());
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
    wtr.write_record(&["name", "upc", "price", "qty", "sku", "weight"])
        .map_err(|e| {
            FixerError::Custom(format!(
                "Could not write headers to nmr.csv because of {:?}",
                e
            ))
        })?;
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
            Err(_) => {
                continue;
            }
        };
        wtr.write_record(&[
            &nmr_product.name,
            &nmr_product.upc.to_string(),
            &nmr_product.price,
            &nmr_product.qty,
        ])
        .map_err(|e| {
            FixerError::Custom(format!(
                "Failed to write a record to nmr.csv because of {:?}",
                e
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

#[derive(Debug, Clone)]
pub struct AbcProduct {
    sku: String,
    desc: String,
    upcs: Vec<Ean13>,
    list: i64,
    cost: i64,
    stock: f64,
    last_sold: Option<chrono::NaiveDate>,
}

impl AbcProduct {
    pub fn sku(&self) -> String {
        self.sku.clone()
    }

    pub fn desc(&self) -> String {
        self.desc.clone()
    }

    pub fn upcs(&self) -> Vec<Ean13> {
        self.upcs.to_vec()
    }

    pub fn list(&self) -> i64 {
        self.list
    }

    pub fn cost(&self) -> i64 {
        self.cost
    }

    pub fn stock(&self) -> f64 {
        self.stock
    }

    pub fn last_sold(&self) -> Option<chrono::NaiveDate> {
        self.last_sold
    }
}

pub struct AbcProductBuilder {
    sku: Option<String>,
    desc: Option<String>,
    upcs: Vec<Ean13>,
    list: Option<i64>,
    cost: Option<i64>,
    stock: Option<f64>,
    last_sold: Option<chrono::NaiveDate>,
}

impl AbcProductBuilder {
    pub fn new() -> Self {
        AbcProductBuilder {
            sku: None,
            desc: None,
            upcs: Vec::new(),
            list: None,
            cost: None,
            stock: None,
            last_sold: None,
        }
    }

    pub fn with_sku(self, sku: &str) -> Self {
        AbcProductBuilder {
            sku: Some(sku.to_string()),
            ..self
        }
    }

    pub fn with_desc(self, desc: &str) -> Self {
        AbcProductBuilder {
            desc: Some(desc.to_string()),
            ..self
        }
    }

    pub fn with_upcs(self, upcs: Vec<Ean13>) -> Self {
        AbcProductBuilder { upcs, ..self }
    }

    pub fn add_upc(self, upc: Ean13) -> Self {
        let mut new_upcs = self.upcs.to_vec();
        new_upcs.push(upc);
        AbcProductBuilder {
            upcs: new_upcs,
            ..self
        }
    }

    pub fn with_list(self, list: i64) -> Self {
        AbcProductBuilder {
            list: Some(list),
            ..self
        }
    }

    pub fn with_cost(self, cost: i64) -> Self {
        AbcProductBuilder {
            cost: Some(cost),
            ..self
        }
    }

    pub fn with_stock(self, stock: f64) -> Self {
        AbcProductBuilder {
            stock: Some(stock),
            ..self
        }
    }

    pub fn build(self) -> Option<AbcProduct> {
        Some(AbcProduct {
            sku: self.sku.clone()?,
            desc: self.desc.clone()?,
            upcs: self.upcs,
            list: self.list?,
            cost: self.cost?,
            stock: self.stock?,
            last_sold: self.last_sold,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct NmrProduct {
    name: String,
    pub upc: Ean13,
    price: String,
    qty: String,
    sku: String,
    weight: usize,
}

impl TryFrom<AbcProduct> for NmrProduct {
    type Error = FixerError;
    fn try_from(value: AbcProduct) -> Result<Self, Self::Error> {
        let upc = value
            .upcs()
            .get(0)
            .ok_or(FixerError::Custom(format!("Missing upc for {:?}", value)))?
            .clone();
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
            upc,
            price: ((value.list() as f32) / 100.0).to_string(),
            qty,
        })
    }
}
