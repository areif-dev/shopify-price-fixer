use std::io::Write;
use std::path::PathBuf;
use std::{cmp, fs, io};

use clap::Parser;
use shopify_price_fixer as fixer;

use reqwest::header::HeaderMap;
use reqwest::header::InvalidHeaderValue;

/// Prompts the user for a file path to an ABC 2-10 report
///
/// # Returns
///
/// If no errors are encountered, return the string value entered by the user. This should be a
/// path to a valid file
///
/// # Errors
///
/// Will return an Err(std::io::Error) if the input operation fails
fn user_input_file_path() -> io::Result<PathBuf> {
    print!("Enter the path to the exported ABC report: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(PathBuf::from(input.trim()))
}

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
    id: u64,
    new_price: u32,
) -> Result<String, InvalidHeaderValue> {
    let (client, headers) = create_client_with_headers(config, "application/json".to_string())?;
    let body = format!(
        "{{
            \"variant\": {{
                \"price\": \"{}\"
            }}
        }}",
        (new_price as f32 / 100.0)
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

    let mut file_path = match cli.report {
        Some(p) => p,
        None => user_input_file_path()?,
    };

    let file: String;
    loop {
        match fs::read_to_string(file_path) {
            Ok(f) => {
                file = f;
                break;
            }
            Err(_) => {
                println!("You have entered a path that does not exist");
                file_path = user_input_file_path()?;
            }
        }
    }

    // Something is probably very wrong if the binary has no parent directory, but if it doesn't,
    // switch everything to use the current working directory to be safe(r)
    let parent_dir = match std::env::current_exe()?.parent() {
        Some(p) => p.to_owned(),
        None => PathBuf::from("."),
    };

    let config_path = match cli.config {
        Some(p) => p,
        None => parent_dir.join("config.json"),
    };
    let config = shopify_price_fixer::Config::read_config(&config_path)?;
    let log_to_stdout = !cli.write_logs;
    let abc_products = shopify_price_fixer::parse_report_1_15(&file);
    let shopify_products = shopify_price_fixer::all_shopify_products(&config).await?;

    for (sku, (shopify_price, shopify_id)) in shopify_products {
        let abc_price = match abc_products.get(&sku) {
            Some(p) => p,
            None => {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::NotFound,
                    format!("NOT FOUND Item {} Shopify: {} cents", sku, shopify_price),
                )?;
                continue;
            }
        };

        match shopify_price.cmp(abc_price) {
            cmp::Ordering::Less => {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::Adjusted,
                    format!(
                        "ADJUSTING Item {} ABC: {} cents, Shopify: {} cents",
                        sku, abc_price, shopify_price
                    ),
                )?;

                // Dry run means that no prices should actually be changed, so skip the update step
                if cli.dry_run {
                    continue;
                }

                match update_shopify_price(&config, shopify_id, abc_price.to_owned()).await {
                    Ok(_) => (),
                    Err(e) => fixer::log(
                        log_to_stdout,
                        fixer::Log::Error,
                        format!("ERROR updating product with id {}: {:?}", shopify_id, e),
                    )?,
                };
            }
            cmp::Ordering::Greater => {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::Greater,
                    format!(
                        "NOT ADJUSTING Item {} ABC: {} cents, Shopify: {} cents",
                        sku, abc_price, shopify_price
                    ),
                )?;
            }
            cmp::Ordering::Equal => {
                fixer::log(
                    log_to_stdout,
                    fixer::Log::Equal,
                    format!(
                        "NOT ADJUSTING Item {} ABC: {} cents, Shopify: {} cents",
                        sku, abc_price, shopify_price
                    ),
                )?;
            }
        }
    }

    Ok(())
}
