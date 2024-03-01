use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use reqwest::header::USER_AGENT;

/// Interfaces with the Shopify REST API to keep prices up to date with the proprietary ABC
/// accounting software. This program will only change a Shopify price if the price in ABC is
/// greater than what is currently in Shopify.
#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Set this to bypass std::out and write logs to log files instead
    #[arg(short, long)]
    pub write_logs: bool,

    /// Optional. Provide the path to report 1-15
    #[arg(short, long)]
    pub report: Option<PathBuf>,

    /// Optional. Path to the config.json file. If left blank, assume ./config.json
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Set this to execute the program normally, except that no prices will actually be changed in
    /// Shopify. Useful for debugging
    #[arg(short, long = "dry")]
    pub dry_run: bool,
}

/// Stores configuration details to run the app. Inlcuding the api key and domain to send queries
/// to
#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// API key for the Shopify admin API
    pub shopify_access_token: String,

    /// The admin subdomain for the business. Like "my-business.myshopify.com". Leave off scheme.
    pub business_url: String,

    /// The publicly facing domain for the storefront. Like "mybusiness.com". Leave off scheme.
    pub storefront_url: String,

    /// The version of the admin api to use. Such as "2022-07"
    pub api_version: String,
}

impl Config {
    /// Reads the configuration from the "config.json" file and returns a `Result`.
    ///
    /// # Arguments
    ///
    /// * `path` - The path the the config file
    ///
    /// # Returns
    ///
    /// * `Result<Self, String>` - Configuration details if successful, `String` error otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error string if it fails to read or parse the configuration file.
    pub fn read_config(path: &PathBuf) -> Result<Self, String> {
        let config_str = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Err("Could not read config file".to_string()),
        };
        let config: Config = match serde_json::from_str(&config_str) {
            Ok(c) => c,
            Err(_) => return Err("Failed to parse config file. Must define business_url, storefront_url, shopify_access_token, and api_version".to_string()),
        };

        Ok(config)
    }
}

/// Parses the tabular output format (1-15) and returns a `HashMap` of SKU to price (in cents).
///
/// # Arguments
///
/// * `taboutput_1_15` - A string representing the tabular output format (1-15).
///
/// # Returns
///
/// * `HashMap<String, u32>` - A mapping of SKU to price in cents.
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

/// Fetches all Shopify products and returns a `HashMap` of SKU to tuple (price in cents, variant ID).
///
/// # Arguments
///
/// * `config` - A reference to the `Config` struct containing Shopify configuration details.
///
/// # Returns
///
/// * `Result<HashMap<String, (u32, u64)>, Box<dyn Error>>` - A mapping of SKU to tuple (price in
/// cents, variant ID) if successful, error otherwise
///
/// # Errors
///
/// Returns an error if it fails to communicate with the Shopify API or parse the response.
pub async fn all_shopify_products(
    config: &Config,
) -> Result<HashMap<String, (u32, u64)>, Box<dyn Error>> {
    let mut page = 0;
    let mut products: HashMap<String, (u32, u64)> = HashMap::new();
    let client = reqwest::Client::new();
    loop {
        let response = client
            .get(format!(
                "https://{}/products.json?limit=250&page={}",
                config.storefront_url, page
            ))
            .header(USER_AGENT, "curl/8.2.1") // Shopify blocks the default `reqwest` user agent
            .send()
            .await?;

        let response_text = response.text().await?;

        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        let products_json = match response_json["products"].as_array() {
            Some(p) => p,
            None => break,
        };

        if products_json.len() == 0 {
            break;
        }

        for product in products_json {
            let variants = match product["variants"].as_array() {
                Some(v) => v,
                None => continue,
            };

            for variant in variants {
                let sku = match variant["sku"].as_str() {
                    Some(s) => s.to_uppercase(),
                    None => continue,
                };
                let id = match variant["id"].as_u64() {
                    Some(i) => i,
                    None => continue,
                };
                let price_str = match variant["price"].as_str() {
                    Some(p) => p,
                    None => continue,
                };

                let price_f32: f32 = price_str.parse()?;
                let price_u32: u32 = (price_f32 * 100.0) as u32;
                products.insert(sku, (price_u32, id));
            }
        }
        page += 1;
    }

    Ok(products)
}

/// List of supported types of logs
#[non_exhaustive]
pub enum Log {
    /// Used for error messages. "./error.txt"
    Error,

    /// Shows which SKUs were adjusted in Shopify. "./adjusted.txt"
    Adjusted,

    /// Shows which SKUs were not adjusted because the price in ABC and Shopify are the same
    /// already. "./not_adjusted_equal.txt"
    Equal,

    /// Shows which SKUs were not adjusted because the price in Shopify is already greater than
    /// ABC. "./not_adjusted_greater.txt"
    Greater,

    /// Shows which SKUs exist in Shopify but were not found in ABC. "./not_found.txt"
    NotFound,
}

/// Handles logging info to the proper file or stdout as specified.
///
/// # Arguments
///
/// * `to_stdout` - If `true`, don't write `msg` to any file, but instead write `msg` to stdout
/// using `print!`. Otherwise, write `msg` to the appropriate log file specified by `log`
///
/// * `log` - Which `Log` `msg` should be written to. This will only be relevant if `to_stdout` is
/// `false`
///
/// * `msg` - The message to be logged
///
/// # Returns
///
/// If successful, return unit type
///
/// # Errors
///
/// Will return `std::io::Error` if `to_stdout` is `false` and the required log file could not be
/// opened or written to
pub fn log<S>(to_stdout: bool, log: Log, msg: S) -> Result<(), std::io::Error>
where
    S: Into<String>,
{
    let mut msg_str: String = msg.into();
    msg_str.push('\n');

    if to_stdout {
        print!("{}", msg_str);
        return Ok(());
    }

    // Something is probably very wrong if the binary has no parent directory, but if it doesn't,
    // switch everything to use the current working directory to be safe(r)
    let log_path_parent = match std::env::current_exe()?.parent() {
        Some(p) => p.to_owned(),
        None => PathBuf::from("."),
    };

    let log_path = match log {
        Log::Adjusted => log_path_parent.join("logs/adjusted.txt"),
        Log::Equal => log_path_parent.join("logs/not_adjusted_equal.txt"),
        Log::Error => log_path_parent.join("logs/error.txt"),
        Log::Greater => log_path_parent.join("logs/not_adjusted_greater.txt"),
        Log::NotFound => log_path_parent.join("logs/not_found.txt"),
    };

    if !log_path_parent.join("logs").exists() {
        fs::create_dir(log_path_parent.join("logs"))?;
    }

    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    log_file.write(&msg_str.as_bytes())?;

    Ok(())
}
