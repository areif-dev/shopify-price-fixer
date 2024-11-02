use crate::upc::Upc;
use serde::ser::Error;
use std::{collections::HashMap, num::ParseFloatError, path::PathBuf};

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
        let upc_str = row.get(43).ok_or(csv::Error::custom(format!(
            "Cannot fetch upcs in row {}",
            i
        )))?;
        let upcs = Upc::from_abc_upc_list(upc_str)
            .iter()
            .filter_map(|upc| upc.to_owned())
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
        let mut existing_record = products
            .get(&sku)
            .ok_or(csv::Error::custom(format!(
                "Cannot find existing product for item with sku {} in row {} of posted_data",
                &sku, i
            )))?
            .clone();
        existing_record.stock = stock;
        products.insert(sku, existing_record);
    }
    Ok(products)
}

#[derive(Debug, Clone)]
pub struct AbcProduct {
    sku: String,
    desc: String,
    upcs: Vec<Upc>,
    list: i64,
    cost: i64,
    stock: f64,
}

impl AbcProduct {
    pub fn sku(&self) -> String {
        self.sku.clone()
    }

    pub fn desc(&self) -> String {
        self.desc.clone()
    }

    pub fn upcs(&self) -> Vec<Upc> {
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
}

pub struct AbcProductBuilder {
    sku: Option<String>,
    desc: Option<String>,
    upcs: Vec<Upc>,
    list: Option<i64>,
    cost: Option<i64>,
    stock: Option<f64>,
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

    pub fn with_upcs(self, upcs: Vec<Upc>) -> Self {
        AbcProductBuilder { upcs, ..self }
    }

    pub fn add_upc(self, upc: Upc) -> Self {
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
        })
    }
}
