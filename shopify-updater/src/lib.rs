use std::collections::HashMap;
use std::error::Error;
use std::fs;

/// Keeps track of product information as it relates to shopify and ABC
#[derive(Debug)]
pub struct Product {
    pub id: String,
    pub display_name: String,
    pub abc_price: f32,
    pub shopify_price: f32,
}

/// Stores configuration details to run the app. Inlcuding the api key and domain to send queries
/// to
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// API key for the Shopify admin API
    pub shopify_access_token: String,

    /// The admin subdomain for the business. Like "my-business.myshopify.com"
    pub business_url: String,

    /// The publicly facing domain for the storefront. Like "mybusiness.com"
    pub storefront_url: String,

    /// The version of the admin api to use. Such as "2022-07"
    pub api_version: String,
}

impl Config {
    pub fn read_config() -> Result<Self, String> {
        let config_str = match fs::read_to_string("./config.json") {
            Ok(c) => c,
            Err(_) => return Err("Could not read config file".to_string()),
        };
        let config: Config = match serde_json::from_str(&config_str) {
            Ok(c) => c,
            Err(_) => return Err("Failed to parse config file. Must define business_domain, shopify_access_token, and api_version".to_string()),
        };

        Ok(config)
    }
}

pub fn parse_report_1_15(taboutput_1_15: &str) -> HashMap<String, u32> {
    let lines = taboutput_1_15.lines().map(|line| line.split('\t'));
    let mut prices = HashMap::new();
    for l in lines {
        let line: Vec<&str> = l.collect();
        let sku = match line.get(0) {
            Some(s) => s.to_uppercase(),
            None => continue,
        };

        let price_str = match line.get(5) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let price_f32: f32 = match price_str.parse() {
            Ok(f) => f,
            Err(_) => continue,
        };

        let price_u32: u32 = (price_f32 * 100.0) as u32;
        prices.insert(sku.to_string(), price_u32);
    }

    prices
}

pub async fn all_shopify_products(
    config: &Config,
) -> Result<HashMap<String, (u32, String)>, Box<dyn Error>> {
    let mut page = 0;
    let mut products: HashMap<String, (u32, String)> = HashMap::new();
    loop {
        let response = reqwest::get(format!(
            "{}/products.json?limit=250&page={}",
            config.storefront_url, page
        ))
        .await?;

        let response_json: serde_json::Value = serde_json::from_str(&response.text().await?)?;
        if response_json["products"].as_array().unwrap().len() == 0 {
            break;
        }

        for product in response_json["products"].as_array().unwrap() {
            for variant in product["variants"].as_array().unwrap() {
                let price_str = variant["price"].as_str().unwrap();
                let price_f32: f32 = price_str.parse()?;
                let price_u32: u32 = (price_f32 * 100.0) as u32;
                products.insert(
                    variant["sku"].as_str().unwrap().to_uppercase(),
                    (price_u32, variant["id"].as_str().unwrap().to_string()),
                );
            }
        }
        page += 1;
    }

    Ok(products)
}
