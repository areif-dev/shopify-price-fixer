use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shopify_price_fixer::Config;
use simple_mass::Mass;
use std::env;

const QUERY: &str = r#"
query GetProductData($after: String) {
  products(first: 20, after: $after, query: "status:active") {
    edges {
      node {
        id
        title
        description
        vendor
        tags
        media(first: 50) {
          edges {
            node {
              id
              mediaContentType
              alt
              ... on MediaImage {
                image {
                  url
                  altText
                  width
                  height
                }
              }
            }
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
        metafields(first: 3, keys: ["catalyst.departments", "catalyst.subcategories", "catalyst.categories"]) {
          nodes {
            namespace
            key
            value
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
        variants(first: 100) {
          edges {
            node {
              id
              price
              inventoryQuantity
              sku
              barcodes(first: 10) {
                edges {
                  node {
                    value
                  }
                }
                pageInfo {
                  hasNextPage
                  endCursor
                }
              }
              inventoryItem {
                measurement {
                  weight {
                    value
                    unit
                  }
                }
              }
            }
          }
          pageInfo {
            hasNextPage
            endCursor
          }
        }
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
"#;

#[derive(Debug)]
pub struct ShopifyResponse {
    pub products: Vec<ShopifyProduct>,
    pub end_cursor: Option<String>,
}

impl<'de> Deserialize<'de> for ShopifyResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let product_nodes =
            edge_nodes(&value, "/data/products/edges").map_err(serde::de::Error::custom)?;
        let products = product_nodes
            .into_iter()
            .map(ShopifyProduct::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;
        let has_next: bool = decode(&value, "/data/products/pageInfo/hasNextPage")
            .map_err(serde::de::Error::custom)?;
        let raw_curser: String = decode(&value, "/data/products/pageInfo/endCursor")
            .map_err(serde::de::Error::custom)?;
        let end_cursor = match (has_next, raw_curser.trim().is_empty()) {
            (_, true) => None,
            (false, _) => None,
            _ => Some(raw_curser),
        };
        Ok(Self {
            products,
            end_cursor,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ShopifyProduct {
    pub id: String,
    pub title: String,
    pub description: String,
    pub vendor: String,
    pub tags: Vec<String>,
    pub media: Vec<ShopifyMedia>,
    pub variants: Vec<ShopifyProductVariant>,
    pub departments: Vec<String>,
    pub categories: Vec<String>,
    pub subcategories: Vec<String>,
}

impl TryFrom<&serde_json::Value> for ShopifyProduct {
    type Error = String;
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let media_nodes = edge_nodes(value, "/media/edges")?;
        let media: Vec<ShopifyMedia> = media_nodes
            .into_iter()
            .map(ShopifyMedia::try_from)
            .filter_map(Result::ok)
            .collect();

        let variants_nodes = edge_nodes(value, "/variants/edges")?;
        let variants: Vec<ShopifyProductVariant> = variants_nodes
            .into_iter()
            .map(ShopifyProductVariant::try_from)
            .collect::<Result<Vec<ShopifyProductVariant>, _>>()?;
        Ok(Self {
            id: decode(value, "/id")?,
            title: decode(value, "/title")?,
            description: decode(value, "/description")?,
            vendor: decode(value, "/vendor")?,
            tags: decode(value, "/tags")?,
            media,
            variants,
            departments: text_list_metafield(value, "catalyst.departments")?,
            categories: text_list_metafield(value, "catalyst.categories")?,
            subcategories: text_list_metafield(value, "catalyst.subcategories")?,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopifyMedia {
    pub id: String,
    pub media_content_type: String,
    pub alt: Option<String>,
    pub image_url: Option<String>,
}

impl TryFrom<&serde_json::Value> for ShopifyMedia {
    type Error = String;
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        Ok(Self {
            id: decode(value, "/id")?,
            media_content_type: decode(value, "/mediaContentType")?,
            alt: decode(value, "/alt").ok(),
            image_url: decode(value, "/image/url").ok(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ShopifyProductVariant {
    pub id: String,
    // Preserve the API's decimal string rather than using a float.
    pub price: String,
    pub quantity_available: Option<i64>,
    pub sku: Option<String>,
    pub barcodes: Vec<String>,
    pub mass: Option<Mass>,
}

impl TryFrom<&serde_json::Value> for ShopifyProductVariant {
    type Error = String;
    fn try_from(value: &serde_json::Value) -> Result<Self, Self::Error> {
        let weight_value: f64 = decode(value, "/inventoryItem/measurement/weight/value")
            .map_err(|e| format!("Missing mass value: {e:?}"))?;
        let weight_unit: String = decode(value, "/inventoryItem/measurement/weight/unit")
            .map_err(|e| format!("Missing mass unit: {e:?}"))?;

        let barcode_nodes = edge_nodes(value, "/barcodes/edges")?;
        let barcodes = barcode_nodes
            .into_iter()
            .map(|value| decode::<String>(value, "/value"))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            id: decode(value, "/id")?,
            price: decode(value, "/price")?,
            quantity_available: decode(value, "/inventoryQuantity")?,
            sku: decode(value, "/sku")?,
            barcodes,
            mass: format!("{weight_value} {weight_unit}").parse().ok(),
        })
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: &Value, pointer: &str) -> Result<T, String> {
    let selected = value
        .pointer(pointer)
        .ok_or(format!("Missing pointer {pointer}"))?;

    serde_json::from_value(selected.clone())
        .map_err(|e| format!("Failed to parse {value:?} due to {e:?}"))
}

fn edge_nodes<'a>(value: &'a Value, pointer: &str) -> Result<Vec<&'a Value>, String> {
    let edges = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or(format!("Expected edges array: {pointer}"))?;

    edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            edge.get("node")
                .filter(|node| !node.is_null())
                .ok_or(format!("Missing node at {pointer}/{index}"))
        })
        .collect()
}

fn text_list_metafield(product: &Value, key: &str) -> Result<Vec<String>, String> {
    let fields = product
        .pointer("/metafields/nodes")
        .and_then(Value::as_array)
        .ok_or(String::from("Missing metafields.nodes"))?;

    let field = fields.iter().find(|field| {
        field["namespace"].as_str() == Some("catalyst") && field["key"].as_str() == Some(key)
    });

    match field {
        // An unset metafield becomes an empty list.
        None => Ok(Vec::new()),
        Some(field) => {
            let raw = field["value"]
                .as_str()
                .ok_or(format!("Expected string for {key}"))?;

            serde_json::from_str(raw).map_err(|e| format!("{e:?} Invalid text list: {key}"))
        }
    }
}

async fn fetch_products(
    client: &Client,
    endpoint: &str,
    token: &str,
) -> Result<Vec<ShopifyProduct>, String> {
    let mut products = Vec::new();
    let mut end_cursor: Option<String> = None;
    loop {
        let response = client
            .post(endpoint)
            .header("X-Shopify-Access-Token", token)
            .header("Content-Type", "application/json")
            .body(json!({"query": QUERY, "variables": {"after": end_cursor}}).to_string())
            .send()
            .await
            .map_err(|e| format!("GraphQL request failed {e:?}"))?;
        let status = response.status();
        if !status.is_success() {
            println!(
                "{:?}",
                response
                    .text()
                    .await
                    .map_err(|e| format!("Parsing shopify response text {e:?}"))?
            );
            return Err(format!(
                "Shopify returned HTTP {status}. Check credentials, scopes, or rate limits."
            ));
        }
        let response: ShopifyResponse = serde_json::from_str(
            &response
                .text()
                .await
                .map_err(|e| format!("{e:?} no text response from Shopify"))?,
        )
        .map_err(|e| format!("{e:?} parsing ShopifyResponse"))?;
        let mut these_products = response.products;
        end_cursor = response.end_cursor;
        products.append(&mut these_products);
        if let None = end_cursor {
            return Ok(products);
        };
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open("./config.json")
        .map_err(|e| format!("Missing config: {e:?}"))?;
    let config: Config =
        serde_json::from_reader(file).map_err(|e| format!("Reading config file: {e:?}"))?;

    // Use your permanent *.myshopify.com hostname, not the storefront domain.
    let shop = config.business_url;
    if !shop.ends_with(".myshopify.com") || shop.contains('/') {
        return Err(String::from(
            "SHOPIFY_SHOP must be a hostname like your-store.myshopify.com",
        ));
    }
    let token = config.shopify_access_token;
    let version = config.api_version;
    let output = env::var("OUTPUT_FILE").unwrap_or_else(|_| "products.json".into());
    let endpoint = format!("https://{shop}/admin/api/{version}/graphql.json");
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Building shopify client: {e:?}"))?;
    let products = fetch_products(&client, &endpoint, &token).await?;
    println!("{}", products.len());
    let output_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&output)
        .map_err(|e| format!("Opening {output}: {e:?}"))?;
    serde_json::to_writer(output_file, &products)
        .map_err(|e| format!("Writing products to {output}: {e:?}"))?;
    Ok(())
}
