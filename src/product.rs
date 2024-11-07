use crate::{create_client_with_headers, upc::Upc, Config, FixerError};
use serde::{ser::Error, Deserialize};
use std::{collections::HashMap, num::ParseFloatError};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub data: Data,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub product_variants: ProductVariants,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProductVariants {
    pub edges: Vec<Edge>,
    pub page_info: PageInfo,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub node: Node,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: String,
    pub sku: Option<String>,
    pub display_name: String,
    pub price: String,
    pub barcode: Option<String>,
    pub available_for_sale: bool,
    pub product: Product,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: String,
    pub status: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: String,
    pub start_cursor: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub requested_query_cost: u32,
    pub actual_query_cost: u32,
    pub throttle_status: ThrottleStatus,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ThrottleStatus {
    pub maximum_available: u32,
    pub currently_available: u32,
    pub restore_rate: u32,
}

#[derive(Debug)]
pub struct ShopifyProduct {
    pub id: String,
    pub sku: String,
    pub display_name: String,
    pub price: i64,
    pub barcode: Option<Upc>,
    pub available_for_sale: bool,
    pub product_id: String,
    pub is_active: bool,
}

impl TryFrom<Node> for ShopifyProduct {
    type Error = FixerError;

    fn try_from(value: Node) -> Result<Self, Self::Error> {
        let barcode = match value.barcode {
            Some(b) => Upc::try_from(b).ok(),
            None => None,
        };
        let price = price_from_str(&value.price).or(Err(FixerError::Custom(format!(
            "Could not parse float from {} for Node with id {}",
            &value.price, &value.id
        ))))?;
        let sku = &value
            .sku
            .ok_or(FixerError::Custom(format!(
                "Missing SKU for Node with id {}",
                &value.id,
            )))?
            .to_uppercase();
        Ok(Self {
            id: value.id,
            sku: sku.to_owned(),
            display_name: value.display_name,
            price,
            barcode,
            available_for_sale: value.available_for_sale,
            product_id: value.product.id,
            is_active: value.product.status == "ACTIVE",
        })
    }
}

pub async fn fetch_shopify_products(
    config: &Config,
) -> Result<(Vec<ShopifyProduct>, Vec<Node>), FixerError> {
    let (client, headers) =
        create_client_with_headers(config, "application/json").or(Err(FixerError::Custom(
            "Encountered InvalidHeaderValue when building client to fetch shopify products"
                .to_string(),
        )))?;
    let mut failed_nodes = Vec::new();
    let mut products = Vec::new();
    let mut has_next_page = true;
    let mut cursor = None;

    while has_next_page {
        let query = serde_json::json!({
            "query": format!(
                r#"
                {{
                    productVariants(first: 250{}) {{
                        edges {{
                            node {{
                                id
                                sku
                                displayName
                                price
                                barcode
                                availableForSale
                                product {{
                                    id 
                                    status
                                }}
                            }}
                        }}
                        pageInfo {{
                            hasNextPage
                            endCursor
                            startCursor
                        }}
                    }}
                }}"#,
                cursor.map_or("".to_string(), |c| format!(" after: \"{}\"", c))
            )
        });

        let url = format!(
            "https://{}/admin/api/2024-10/graphql.json",
            config.business_url
        );

        let response = client
            .post(&url)
            .headers(headers.to_owned())
            .body(query.to_string())
            .send()
            .await?;

        let text = response.text().await?;
        let graphql: Response = serde_json::from_str(&text)?;
        has_next_page = graphql.data.product_variants.page_info.has_next_page;
        cursor = Some(graphql.data.product_variants.page_info.end_cursor);

        for edge in graphql.data.product_variants.edges {
            match ShopifyProduct::try_from(edge.node.clone()) {
                Ok(p) => products.push(p),
                Err(_) => failed_nodes.push(edge.node),
            }
        }
    }

    Ok((products, failed_nodes))
}

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
        let upcs: Vec<Upc> = Upc::from_abc_upc_list(upc_str)
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
        existing_record.sku = existing_record.sku.to_uppercase();
        products.insert(sku, existing_record);
    }
    Ok(products)
}

pub fn map_upcs(existing_map: &HashMap<String, AbcProduct>) -> HashMap<String, (bool, AbcProduct)> {
    let mut upc_map = HashMap::new();
    for (_sku, product) in existing_map {
        for upc in product.upcs.iter() {
            let dup = match upc_map.get(&upc.to_string()) {
                Some(_) => true,
                None => false,
            };
            upc_map.insert(upc.to_string(), (dup, product.to_owned()));
        }
    }
    upc_map
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
