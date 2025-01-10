use std::path::PathBuf;
use std::{cmp, fs};

use clap::Parser;
use shopify_price_fixer::product::{
    map_upcs, AbcProduct, ShopifyProduct, UpdateShopifyPriceResponse,
};
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
async fn update_shopify_price(
    config: &fixer::Config,
    shopify_product: &ShopifyProduct,
    abc_product: &AbcProduct,
) -> Result<UpdateShopifyPriceResponse, FixerError> {
    let (client, headers) = create_client_with_headers(config, "application/json".to_string()).or(
        Err(FixerError::Custom(
            "Found InvalidHeaderValue when updating shopify product".to_string(),
        )),
    )?;
    let new_price = abc_product.list().max(shopify_product.price);
    let query = serde_json::json!({
        "query": r#"
            mutation productVariantsBulkUpdate($productId: ID!, $variants: [ProductVariantsBulkInput!]!) { 
                productVariantsBulkUpdate(productId: $productId, variants: $variants) { 
                    product { 
                        id 
                        status 
                    } 
                    productVariants { 
                        id 
                        sku 
                        price 
                    } 
                    userErrors {
                        field 
                        message 
                    } 
                }
            };"#,
        "variables": {
            "productId": shopify_product.product_id,
            "variants": [
              {
                "id": shopify_product.id,
                "inventoryItem": {
                    "sku": abc_product.sku(),
                },
                "price": format!("{:0.2}", (new_price as f64) / 100.0)
              }
            ],
        }
    });

    let url = format!(
        "https://{}/admin/api/{}/graphql.json",
        config.business_url, config.api_version
    );

    let res = client
        .post(url)
        .headers(headers)
        .body(query.to_string())
        .send()
        .await?
        .text()
        .await?;
    Ok(serde_json::from_str(&res)?)
}

async fn update_shopify_inventory(
    config: &fixer::Config,
    shopify_product: &ShopifyProduct,
    abc_product: &AbcProduct,
) -> Result<(), FixerError> {
    let (client, headers) = create_client_with_headers(config, "application/json".to_string()).or(
        Err(FixerError::Custom(
            "Found InvalidHeaderValue when updating shopify inventory".to_string(),
        )),
    )?;
    let tracked_query = serde_json::json!({
        "query": r#"
            mutation inventoryItemUpdate($id: ID!, $input: InventoryItemInput!) {
                inventoryItemUpdate(id: $id, input: $input) {
                    inventoryItem {
                        id 
                        tracked 
                        unitCost {
                            amount
                        }
                    }
                    userErrors {
                        message
                    }
                }
            }
        "#,
        "variables": {
            "id": shopify_product.inventory_item_id,
            "input": {
                "tracked": true,
                "cost": abc_product.cost() as f64 / 100.0
            }
        }
    });
    let query = serde_json::json!({
        "query": r#"
            mutation InventorySet($input: InventorySetQuantitiesInput!) {
                inventorySetQuantities(input: $input) {
                    inventoryAdjustmentGroup {
                        createdAt
                        changes {
                            item {
                                sku 
                            }
                            quantityAfterChange
                        }
                        reason
                    }
                    userErrors {
                        message
                    }
                }
            }"#,
        "variables": {
            "input": {
                "ignoreCompareQuantity": true,
                "name": "on_hand",
                "reason": "correction",
                "quantities": [{
                    "inventoryItemId": shopify_product.inventory_item_id,
                    "locationId": "gid://shopify/Location/5535957028",
                    "quantity": if abc_product.stock() > 0.0 { abc_product.stock() as i64 } else { 0 },
                }]
            },
        }
    });

    let url = format!(
        "https://{}/admin/api/{}/graphql.json",
        config.business_url, config.api_version
    );

    let tracked_res = client
        .post(url.clone())
        .headers(headers.clone())
        .body(tracked_query.to_string())
        .send()
        .await?
        .text()
        .await?;
    println!("{}", tracked_res);

    let res = client
        .post(url)
        .headers(headers)
        .body(query.to_string())
        .send()
        .await?
        .text()
        .await?;
    println!("{}", res);
    Ok(())
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
    let upc_map = map_upcs(&abc_products);
    let (shopify_products, failed_nodes) = fixer::product::fetch_shopify_products(&config).await?;
    for node in failed_nodes {
        match ShopifyProduct::try_from(node) {
            Ok(_) => continue,
            Err(e) => fixer::log(
                log_to_stdout,
                fixer::Log::Error,
                format!("FAILED NODE because of {}", e),
            )?,
        }
    }

    for shopify_product in shopify_products {
        if !&shopify_product.is_active {
            continue;
        }

        let abc_product = match abc_products.get(&shopify_product.sku) {
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
        };

        let mut skip_price = false;
        let mut skip_inventory = false;
        if &shopify_product.sku.to_uppercase() == &abc_product.sku().to_uppercase() {
            if &shopify_product.price == &abc_product.list() {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::Equal,
                    format!(
                        "NOT ADJUSTING EQUAL {:?}, {:?}",
                        &shopify_product, &abc_product
                    ),
                )?;
                skip_price = true;
            } else if &shopify_product.price > &abc_product.list() {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::Greater,
                    format!(
                        "NOT ADJUSTING GREATER {:?}, {:?}",
                        &shopify_product, &abc_product
                    ),
                )?;
                skip_price = true;
            }
            if shopify_product.stock == abc_product.stock() as i64 {
                skip_inventory = true;
            }
        }

        fixer::log(
            log_to_stdout,
            fixer::Log::Adjusted,
            format!("ADJUSTING {:?}, {:?}", &shopify_product, &abc_product),
        )?;

        // Dry run means that no prices should actually be changed, so skip the update step
        if cli.dry_run {
            continue;
        }

        if !skip_inventory {
            if let Err(e) = update_shopify_inventory(&config, &shopify_product, &abc_product).await
            {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::Error,
                    format!(
                        "ERROR updating inventory for product with id {:?}: {:?}",
                        &shopify_product, e
                    ),
                )?;
            }
        }

        if !skip_price {
            match update_shopify_price(&config, &shopify_product, &abc_product).await {
                Ok(m) => {
                    println!("{:?}", m);
                }

                Err(e) => fixer::log(
                    log_to_stdout,
                    fixer::Log::Error,
                    format!(
                        "ERROR updating product with id {:?}: {:?}",
                        &shopify_product, e
                    ),
                )?,
            }
        }
    }

    Ok(())
}
