use std::path::PathBuf;
use std::{cmp, fs};

use clap::Parser;
use shopify_price_fixer::product::{map_upcs, AbcProduct, ShopifyProduct};
use shopify_price_fixer::{self as fixer, product, FixerError};

use reqwest::header::HeaderMap;
use reqwest::header::InvalidHeaderValue;

/// Initialize a reqwest client and its HeaderMap for sending HTTP requests
///
/// # Arguments
///
/// * `content-type` - The MIME type of the data being sent in the request. The price fixer only
/// uses "application/json" and "application/graphql", but any valid contenty type should work
///
/// # Returns
///
/// If successful, return a tuple containing (reqwest::Client, reqwest::header::HeaderMap)
///
/// # Errors
///
/// * The thread will panic if the config file does not exist or is missing information
/// * Will return Err(reqwest::header::InvalidHeaderValue) if `content-type` is an invalid MIME
/// type or if the API_ACCESS_TOKEN cannot be parsed
fn create_client_with_headers(
    config: &shopify_price_fixer::Config,
    content_type: String,
) -> Result<(reqwest::Client, HeaderMap), InvalidHeaderValue> {
    let access_token = &config.shopify_access_token;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", content_type.parse()?);
    headers.insert("X-Shopify-Access-Token", access_token.parse()?);

    Ok((client, headers))
}

/// Send a put request to a given shopify product to change its price
///
/// # Arguments
///
/// * `id` - The unique shopify id for the product to update
/// * `new_price` - The value to set as the new price for the shopify item in cents. So $1.99 would
/// be 199
///
/// # Returns
///
/// Will return the response text of the request
///
/// # Errors
///
/// Thread will panic if sending the request fails or if fetching the response fails
async fn update_shopify_listing(
    config: &fixer::Config,
    shopify_product: &ShopifyProduct,
    abc_product: &AbcProduct,
) -> Result<String, FixerError> {
    let (client, headers) = create_client_with_headers(config, "application/json".to_string()).or(
        Err(FixerError::Custom(
            "Found InvalidHeaderValue when updating shopify product".to_string(),
        )),
    )?;
    let new_price = abc_product.list().max(shopify_product.price);
    let query = serde_json::json!({
        "query": r#"
            mutation productVariantsBulkUpdate($productId: ID!, $variants: [ProductVariantsBulkInput!]!) {{
              productVariantsBulkUpdate(productId: $productId, variants: $variants) {{
                product {{
                  id
                }}
                productVariants {{
                  id
                  sku 
                  price 
                }}
                userErrors {{
                  field
                  message
                }}
              }}
            }}
    "#,
        "variables": {
            "productId": shopify_product.product_id,
            "variants": [
              {
                "id": shopify_product.id,
                "sku": abc_product.sku(),
                "price": format!("{:0.2}", (new_price as f64) / 100.0)
              }
            ]
        }
    });

    let url = format!(
        "https://{}/admin/api/{}/graphql.json",
        config.business_url, config.api_version
    );

    let res = client
        .put(url)
        .headers(headers)
        .body(query.to_string())
        .send()
        .await?
        .text()
        .await?;

    Ok(res)
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
    let (upc_map, duplicates) = map_upcs(&abc_products);
    for dup in duplicates {
        fixer::log(
            log_to_stdout,
            fixer::Log::DuplicateAbcUpcs,
            format!("DUPLICATE UPC {}", dup.to_string()),
        )?;
    }
    let shopify_products = fixer::product::fetch_shopify_products(&config).await?;

    for shopify_product in shopify_products {
        let abc_product = match abc_products.get(&shopify_product.sku) {
            Some(p) => p,
            None => {
                let barcode = match &shopify_product.barcode {
                    Some(u) => u.to_string(),
                    None => "".to_string(),
                };
                match upc_map.get(&barcode) {
                    Some(p) => p,
                    None => {
                        fixer::log(
                            log_to_stdout,
                            fixer::Log::NotFound,
                            format!(
                                "NOT FOUND Item {} Shopify: {} cents",
                                &shopify_product.sku, &shopify_product.price
                            ),
                        )?;
                        continue;
                    }
                }
            }
        };

        fixer::log(
            log_to_stdout,
            fixer::Log::Adjusted,
            format!(
                "ADJUSTING Item {} ABC: {} cents, Shopify: {} cents, {} stock",
                &shopify_product.sku,
                &abc_product.list(),
                &shopify_product.price,
                &abc_product.stock(),
            ),
        )?;

        // Dry run means that no prices should actually be changed, so skip the update step
        if cli.dry_run {
            continue;
        }

        if &shopify_product.sku != &abc_product.sku()
            || &shopify_product.price != &abc_product.list()
        {
            match update_shopify_listing(&config, &shopify_product, &abc_product).await {
                Ok(_) => {
                    fixer::log(
                        log_to_stdout,
                        fixer::Log::Adjusted,
                        format!(
                            "ADJUSTING Shopify {} {} cents, ABC {} {} cents",
                            &shopify_product.sku,
                            &shopify_product.price,
                            &abc_product.sku(),
                            &abc_product.list(),
                        ),
                    )?;
                }
                Err(e) => fixer::log(
                    log_to_stdout,
                    fixer::Log::Error,
                    format!(
                        "ERROR updating product with id {}: {:?}",
                        &shopify_product.id, e
                    ),
                )?,
            }
        }
    }

    Ok(())
}
