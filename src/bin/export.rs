//
//#[tokio::main]
//async fn main() -> Result<(), String> {
//    let config = Config::read()?;
//
//    // Use your permanent *.myshopify.com hostname, not the storefront domain.
//    let shop = config.business_url;
//    if !shop.ends_with(".myshopify.com") || shop.contains('/') {
//        return Err(String::from(
//            "SHOPIFY_SHOP must be a hostname like your-store.myshopify.com",
//        ));
//    }
//    let token = config.shopify_access_token;
//    let version = config.api_version;
//    let output = env::var("OUTPUT_FILE").unwrap_or_else(|_| "products.json".into());
//    let endpoint = format!("https://{shop}/admin/api/{version}/graphql.json");
//    let client = Client::builder()
//        .timeout(std::time::Duration::from_secs(60))
//        .build()
//        .map_err(|e| format!("Building shopify client: {e:?}"))?;
//    let products = fetch_products(&client, &endpoint, &token).await?;
//    println!("{}", products.len());
//    let output_file = std::fs::OpenOptions::new()
//        .write(true)
//        .create(true)
//        .truncate(true)
//        .open(&output)
//        .map_err(|e| format!("Opening {output}: {e:?}"))?;
//    serde_json::to_writer(output_file, &products)
//        .map_err(|e| format!("Writing products to {output}: {e:?}"))?;
//    Ok(())
//}
