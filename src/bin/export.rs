use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::{env, fs::File, io::BufWriter};

const QUERY: &str = r#"
query GetProductData($after: String) {
  products(first: 10, after: $after) {
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
                nodes {
                  type
                  value
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

fn required_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("Set {name} in your environment"))
}

// This starter paginates products only. Never silently export truncated
// nested connections: stop and explain which connection needs pagination.
fn ensure_complete(connection: &Value, label: &str) -> Result<()> {
    let more = connection["pageInfo"]["hasNextPage"]
        .as_bool()
        .ok_or_else(|| anyhow!("Missing pageInfo for {label}"))?;
    if more {
        bail!(
            "{label} exceeds the nested page limit; implement nested pagination before exporting"
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Use your permanent *.myshopify.com hostname, not the storefront domain.
    let shop = required_env("SHOPIFY_SHOP")?;
    if !shop.ends_with(".myshopify.com") || shop.contains('/') {
        bail!("SHOPIFY_SHOP must be a hostname like your-store.myshopify.com");
    }
    let token = required_env("SHOPIFY_ACCESS_TOKEN")?;
    let version = required_env("SHOPIFY_API_VERSION")?;
    let output = env::var("OUTPUT_FILE").unwrap_or_else(|_| "products.json".into());
    let endpoint = format!("https://{shop}/admin/api/{version}/graphql.json");
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let mut after: Option<String> = None;
    let mut products: Vec<Value> = Vec::new();
    loop {
        let response = client
            .post(&endpoint)
            .header("X-Shopify-Access-Token", &token)
            .json(&json!({"query": QUERY, "variables": {"after": after}}))
            .send()
            .await
            .context("GraphQL request failed")?;
        let status = response.status();
        if !status.is_success() {
            bail!("Shopify returned HTTP {status}. Check credentials, scopes, or rate limits.");
        }
        let body: Value = response.json().await.context("Invalid JSON response")?;
        if let Some(errors) = body.get("errors") {
            if errors.as_array().map_or(true, |items| !items.is_empty()) {
                bail!("GraphQL errors: {errors}");
            }
        }
        let connection = &body["data"]["products"];
        let edges = connection["edges"]
            .as_array()
            .ok_or_else(|| anyhow!("Missing products.edges"))?;
        for edge in edges {
            let mut product = edge["node"].clone();
            let id = product["id"].as_str().unwrap_or("unknown").to_owned();
            for name in ["media", "metafields", "variants"] {
                ensure_complete(&product[name], &format!("{id}: {name}"))?;
            }
            let variants = product["variants"]["edges"]
                .as_array()
                .ok_or_else(|| anyhow!("Missing variants for {id}"))?;
            for variant in variants {
                ensure_complete(
                    &variant["node"]["barcodes"],
                    &format!("{id}: variant barcodes"),
                )?;
            }
            // Convert the three JSON-encoded metafield lists to JSON arrays.
            if let Some(fields) = product["metafields"]["nodes"].as_array_mut() {
                for field in fields {
                    let raw = field["value"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing metafield value for {id}"))?;
                    let parsed: Vec<String> = serde_json::from_str(raw)
                        .with_context(|| format!("Invalid text-list metafield for {id}"))?;
                    field["value"] = json!(parsed);
                }
            }
            products.push(product);
        }
        eprintln!("Collected {} products", products.len());
        let more = connection["pageInfo"]["hasNextPage"]
            .as_bool()
            .ok_or_else(|| anyhow!("Missing product pageInfo"))?;
        if !more {
            break;
        }
        let cursor = connection["pageInfo"]["endCursor"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing next-page cursor"))?;
        if after.as_deref() == Some(cursor) {
            bail!("Repeated pagination cursor");
        }
        after = Some(cursor.to_owned());
        // Basic pacing only; add cost-aware throttling/retries for production.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    let writer = BufWriter::new(File::create(&output)?);
    serde_json::to_writer_pretty(writer, &products)?;
    println!("Saved {} products to {output}", products.len());
    Ok(())
}
