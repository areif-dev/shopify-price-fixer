use std::path::PathBuf;
use std::{cmp, fs};

use clap::Parser;
use shopify_price_fixer::{self as fixer, product};

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
    id: u64,
    new_price: i64,
    new_stock: f64,
) -> Result<String, InvalidHeaderValue> {
    let (client, headers) = create_client_with_headers(config, "application/json".to_string())?;
    let body = format!(
        "{{
            \"variant\": {{
                \"price\": \"{}\"
            }}
        }}",
        (new_price as f32 / 100.0),
    );

    let url = format!("https://{}/admin/variants/{}.json", config.business_url, id);

    let res = client
        .put(url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

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
    let shopify_products = fixer::product::fetch_shopify_products(&config).await?;
    println!("{:#?}", shopify_products);

    // TODO: Create another hashmap of fixed abc upcs to their abc skus, so you can mutate any
    // shopify skus that do not match up 

    // for (sku, (shopify_price, shopify_id)) in shopify_products {
    //     let abc_product = match abc_products.get(&sku) {
    //         Some(p) => p,
    //         None => {
    //             fixer::log(
    //                 log_to_stdout,
    //                 fixer::Log::NotFound,
    //                 format!("NOT FOUND Item {} Shopify: {} cents", sku, shopify_price),
    //             )?;
    //             continue;
    //         }
    //     };

    //     fixer::log(
    //         log_to_stdout,
    //         fixer::Log::Adjusted,
    //         format!(
    //             "ADJUSTING Item {} ABC: {} cents, Shopify: {} cents, {} stock",
    //             sku,
    //             &abc_product.list(),
    //             shopify_price,
    //             &abc_product.stock(),
    //         ),
    //     )?;

    //     // Dry run means that no prices should actually be changed, so skip the update step
    //     if cli.dry_run {
    //         continue;
    //     }

    //     match update_shopify_listing(
    //         &config,
    //         shopify_id,
    //         abc_product.list().to_owned().max(shopify_price as i64),
    //         abc_product.stock(),
    //     )
    //     .await
    //     {
    //         Ok(r) => {
    //             println!("{}", r);
    //         }
    //         Err(e) => fixer::log(
    //             log_to_stdout,
    //             fixer::Log::Error,
    //             format!("ERROR updating product with id {}: {:?}", shopify_id, e),
    //         )?,
    //     };

    // match (shopify_price as i64).cmp(&abc_product.list()) {
    //     cmp::Ordering::Less => {
    //         fixer::log(
    //             log_to_stdout,
    //             fixer::Log::Adjusted,
    //             format!(
    //                 "ADJUSTING Item {} ABC: {} cents, Shopify: {} cents",
    //                 sku,
    //                 &abc_product.list(),
    //                 shopify_price
    //             ),
    //         )?;

    //         // Dry run means that no prices should actually be changed, so skip the update step
    //         if cli.dry_run {
    //             continue;
    //         }

    //         match update_shopify_listing(
    //             &config,
    //             shopify_id,
    //             abc_product.list().to_owned(),
    //             abc_product.stock(),
    //         )
    //         .await
    //         {
    //             Ok(_) => (),
    //             Err(e) => fixer::log(
    //                 log_to_stdout,
    //                 fixer::Log::Error,
    //                 format!("ERROR updating product with id {}: {:?}", shopify_id, e),
    //             )?,
    //         };
    //     }
    //     cmp::Ordering::Greater => {
    //         fixer::log(
    //             log_to_stdout,
    //             fixer::Log::Greater,
    //             format!(
    //                 "NOT ADJUSTING Item {} ABC: {} cents, Shopify: {} cents",
    //                 sku,
    //                 &abc_product.list(),
    //                 shopify_price
    //             ),
    //         )?;
    //     }
    //     cmp::Ordering::Equal => {
    //         fixer::log(
    //             log_to_stdout,
    //             fixer::Log::Equal,
    //             format!(
    //                 "NOT ADJUSTING Item {} ABC: {} cents, Shopify: {} cents",
    //                 sku,
    //                 &abc_product.list(),
    //                 shopify_price
    //             ),
    //         )?;
    //     }
    // }
    // }

    Ok(())
}
