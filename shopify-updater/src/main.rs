use std::io::Write;
use std::{cmp, env, fs, io};

use shopify_updater::{Config, Product};

use serde_json::Value;

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
fn user_input_file_path() -> io::Result<String> {
    print!("Enter the path to the exported ABC report: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input)
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
    content_type: String,
) -> Result<(reqwest::Client, HeaderMap), InvalidHeaderValue> {
    let config = Config::read_config().unwrap();
    let access_token = config.shopify_access_token;

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
async fn update_shopify_price(config: &Config, id: &str, new_price: u32) -> String {
    let (client, headers) = match create_client_with_headers("application/json".to_string()) {
        Ok(c) => c,
        Err(e) => {
            println!("Error in update_shopify_price {:?}", e);
            return format!("Failed to create client for product id: {}", id);
        }
    };

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

    res
}

/// Try to find corresponding product(s) on shopify by their sku from ABC
///
/// # Arguments
///
/// * `sku` - The stock keeping unit that ABC uses for the product
/// * `abc_price` - The price of the product in ABC
///
/// # Returns
///
/// Will return a vector of matching variants from shopify if any are found. If no matching
/// variants are found, return None
///
/// # Errors
///
/// Thread will panic if sending or receiving the HTTP request fails. Thread will also panic if
/// building a Product object from the json response fails at any point
async fn shopify_get_product_by_sku(sku: &str, abc_price: f32) -> Option<Vec<Product>> {
    let config = Config::read_config().unwrap();

    let (client, headers) = match create_client_with_headers("application/graphql".to_string()) {
        Ok(t) => t,
        Err(e) => {
            println!(
                "Failed to create client in shopify_get_product_by_sku because of {:?}",
                e
            );
            return None;
        }
    };

    let body = format!(
        "{{ 
            productVariants(first: 10, query: \"sku:{}\") {{ 
                edges {{ 
                    node {{ 
                        id 
                        sku 
                        displayName 
                        price 
                    }} 
                }} 
            }} 
        }}",
        sku
    );

    let res = client
        .post(format!(
            "https://{}/admin/api/{}/graphql.json",
            config.business_url, config.api_version
        ))
        .headers(headers)
        .body(body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let product_values: Value = match serde_json::from_str(&res) {
        Ok(v) => v,
        Err(e) => {
            println!("Could not create product for sku: {} {:?}", sku, e);
            return None;
        }
    };

    let edges: Vec<Value> = match product_values["data"]["productVariants"]["edges"].as_array() {
        Some(v) => v.to_vec(),
        None => return None,
    };

    if edges.len() == 0 {
        return None;
    }

    let mut products: Vec<Product> = Vec::new();

    for node in edges.iter() {
        let display_name: String = node["node"]["displayName"].as_str().unwrap().to_string();
        let id: String = node["node"]["id"].as_str().unwrap()[29..].to_string();
        let shopify_price_str: String = node["node"]["price"].as_str().unwrap().to_string();
        let shopify_price: f32 = shopify_price_str.parse().unwrap();

        let product = Product {
            abc_price,
            display_name,
            id,
            shopify_price,
        };

        products.push(product);
    }

    Some(products)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file_path = match env::args().nth(1) {
        Some(p) => p,
        None => user_input_file_path()?.trim().to_string(),
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
                file_path = user_input_file_path()?.trim().to_string();
            }
        }
    }

    let config = shopify_updater::Config::read_config()?;
    let abc_products = shopify_updater::parse_report_1_15(&file);
    let shopify_products = shopify_updater::all_shopify_products(&config).await?;

    for (sku, (shopify_price, shopify_id)) in shopify_products {
        let abc_price = match abc_products.get(&sku) {
            Some(p) => p,
            None => continue,
        };

        match shopify_price.cmp(abc_price) {
            cmp::Ordering::Less => {
                update_shopify_price(&config, &shopify_id, abc_price.to_owned()).await;
            }
            cmp::Ordering::Greater => {
                println!(
                    "Item {} ABC: {} cents, Shopify: {} cents. Not adjusting",
                    sku, abc_price, shopify_price
                );
            }
            cmp::Ordering::Equal => (),
        }
    }

    println!("--- Press Enter to exit ---");
    let mut _x = String::new();
    io::stdin().read_line(&mut _x)?;

    Ok(())
}
