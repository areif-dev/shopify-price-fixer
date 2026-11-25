use abc_product::{AbcProduct, AbcProductsBySku};
use gtin::Gtin;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::BarcodeWrapper;

/// ABC products that share a barcode, keyed by their primary SKU.
pub type DuplicateProducts = HashMap<String, AbcProduct>;

/// The result of matching export identifiers against ABC products.
#[derive(Debug, Default)]
pub struct IdentifierMatches {
    /// ABC products that matched exactly one identifier, keyed by their primary SKU.
    pub matched: HashMap<String, AbcProduct>,
    /// Identifiers that did not match any ABC product.
    pub missing: HashSet<BarcodeWrapper>,
    /// GTINs shared by multiple ABC products, keyed by GTIN and then primary SKU.
    pub duplicate_gtins: HashMap<Gtin, HashMap<String, AbcProduct>>,
}

/// Parses newline-separated export identifiers into [`BarcodeWrapper`]s.
///
/// Lines are trimmed and blank lines are skipped. Each remaining line is
/// interpreted as a [`Gtin`] when possible, falling back to a SKU string.
///
/// # Arguments
/// * `text` - The raw contents of an export identifiers file.
///
/// # Returns
/// A set of unique identifiers, so repeated lines are collapsed.
///
/// # Example
/// ```
/// use shopify_price_fixer::BarcodeWrapper;
/// use shopify_price_fixer::product::parse_identifiers;
///
/// let identifiers = parse_identifiers("0123456789012\n\n  ABC-123  \n");
/// assert!(identifiers.contains(&BarcodeWrapper::from("0123456789012".to_string())));
/// assert!(identifiers.contains(&BarcodeWrapper::from("ABC-123".to_string())));
/// ```
pub fn parse_identifiers(text: &str) -> HashSet<BarcodeWrapper> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| BarcodeWrapper::from(line.to_string()))
        .collect()
}

/// Matches export identifiers against ABC products.
///
/// GTIN identifiers are matched against every ABC product UPC. Identifiers that
/// cannot be interpreted as a GTIN are treated as primary SKUs. GTINs shared by
/// multiple ABC products are reported as ambiguous instead of matching any of
/// those products.
///
/// # Arguments
/// * `abc_products` - The ABC products to match against, keyed by primary SKU.
/// * `identifiers` - The deduplicated GTIN or SKU identifiers to match.
///
/// # Returns
/// The unambiguously matched products, the identifiers that matched nothing, and
/// any GTINs that are shared by multiple ABC products.
///
/// # Example
/// ```
/// use std::collections::HashSet;
///
/// use abc_product::{AbcProductBuilder, AbcProductsBySku};
/// use shopify_price_fixer::BarcodeWrapper;
/// use shopify_price_fixer::product::match_identifiers;
///
/// let product = AbcProductBuilder::new()
///     .with_sku("ABC-123")
///     .with_desc("Test product")
///     .with_list("1.00".parse().unwrap())
///     .with_cost("0.50".parse().unwrap())
///     .with_stock(1.0)
///     .with_sale_gl(1)
///     .build()
///     .unwrap();
/// let abc_products: AbcProductsBySku = [(product.sku(), product)].into();
/// let identifiers = HashSet::from([BarcodeWrapper::from("ABC-123".to_string())]);
///
/// let result = match_identifiers(&abc_products, &identifiers);
/// assert!(result.matched.contains_key("ABC-123"));
/// ```
pub fn match_identifiers(
    abc_products: &AbcProductsBySku,
    identifiers: &HashSet<BarcodeWrapper>,
) -> IdentifierMatches {
    let mut upc_map = map_upcs(abc_products);
    let mut matches = IdentifierMatches::default();

    for identifier in identifiers {
        match identifier {
            BarcodeWrapper::Gtin(gtin) => match upc_map.remove(gtin) {
                None => {
                    matches.missing.insert(identifier.clone());
                }
                Some((duplicates, _)) if !duplicates.is_empty() => {
                    matches.duplicate_gtins.insert(*gtin, duplicates);
                }
                Some((_, product)) => {
                    matches.matched.insert(product.sku(), product);
                }
            },
            BarcodeWrapper::String(sku) => match abc_products.get(sku) {
                Some(product) => {
                    matches.matched.insert(product.sku(), product.clone());
                }
                None => {
                    matches.missing.insert(identifier.clone());
                }
            },
        }
    }

    matches
}

pub fn map_upcs(
    existing_map: &HashMap<String, AbcProduct>,
) -> HashMap<Gtin, (DuplicateProducts, AbcProduct)> {
    let mut upc_map = HashMap::new();
    for product in existing_map.values() {
        for upc in product.upcs() {
            match upc_map.entry(upc) {
                Entry::Vacant(entry) => {
                    entry.insert((DuplicateProducts::new(), product.clone()));
                }
                Entry::Occupied(mut entry) => {
                    let (duplicates, first) = entry.get_mut();
                    if product.sku() != first.sku() {
                        duplicates.insert(product.sku(), product.clone());
                        duplicates.insert(first.sku(), first.clone());
                    }
                }
            }
        }
    }
    upc_map
}

#[cfg(test)]
mod tests {
    use super::*;
    use abc_product::AbcProductBuilder;

    fn gtin(code: &str) -> Gtin {
        Gtin::nonstrict_new(code)
    }

    fn product(sku: &str, upcs: Vec<Gtin>) -> AbcProduct {
        AbcProductBuilder::new()
            .with_sku(sku)
            .with_desc("Test product")
            .with_upcs(upcs)
            .with_list("1.00".parse().unwrap())
            .with_cost("0.50".parse().unwrap())
            .with_stock(1.0)
            .with_sale_gl(1)
            .build()
            .unwrap()
    }

    #[test]
    fn parse_identifiers_trims_skips_blanks_and_parses_gtins() {
        let upc = gtin("0123456789012");
        let identifiers = parse_identifiers(&format!("\n  {upc}  \n\nABC-123\n  \n12345\n"));

        assert_eq!(identifiers.len(), 3);
        assert!(identifiers.contains(&BarcodeWrapper::Gtin(upc)));
        assert!(identifiers.contains(&BarcodeWrapper::String("ABC-123".to_string())));
        assert!(identifiers.contains(&BarcodeWrapper::String("12345".to_string())));
    }

    #[test]
    fn matches_gtins_and_primary_skus_and_reports_missing_identifiers() {
        let upc = gtin("0123456789012");
        let first = product("SKU-1", vec![upc]);
        let second = product("SKU-2", vec![]);
        let abc_products: AbcProductsBySku = [(first.sku(), first), (second.sku(), second)].into();
        let identifiers = HashSet::from([
            BarcodeWrapper::Gtin(upc),
            BarcodeWrapper::from("SKU-2".to_string()),
            BarcodeWrapper::from("MISSING-SKU".to_string()),
            BarcodeWrapper::Gtin(gtin("0987654321098")),
        ]);

        let result = match_identifiers(&abc_products, &identifiers);

        assert_eq!(result.matched.len(), 2);
        assert!(result.matched.contains_key("SKU-1"));
        assert!(result.matched.contains_key("SKU-2"));
        assert_eq!(result.missing.len(), 2);
        assert!(result.duplicate_gtins.is_empty());
    }

    #[test]
    fn collapses_gtin_and_sku_matches_for_the_same_product() {
        let upc = gtin("0123456789012");
        let first = product("SKU-1", vec![upc]);
        let abc_products: AbcProductsBySku = [(first.sku(), first)].into();
        let identifiers = HashSet::from([
            BarcodeWrapper::Gtin(upc),
            BarcodeWrapper::from("SKU-1".to_string()),
        ]);

        let result = match_identifiers(&abc_products, &identifiers);

        assert_eq!(result.matched.len(), 1);
        assert!(result.matched.contains_key("SKU-1"));
    }

    #[test]
    fn reports_gtins_shared_by_multiple_products_as_ambiguous() {
        let upc = gtin("0123456789012");
        let first = product("SKU-1", vec![upc]);
        let second = product("SKU-2", vec![upc]);
        let abc_products: AbcProductsBySku = [(first.sku(), first), (second.sku(), second)].into();
        let identifiers = HashSet::from([BarcodeWrapper::Gtin(upc)]);

        let result = match_identifiers(&abc_products, &identifiers);

        assert!(result.matched.is_empty());
        assert!(result.missing.is_empty());
        assert_eq!(result.duplicate_gtins.len(), 1);
        let products = result.duplicate_gtins.get(&upc).unwrap();
        assert_eq!(products.len(), 2);
        assert!(products.contains_key("SKU-1"));
        assert!(products.contains_key("SKU-2"));
    }

    #[test]
    fn deduplicates_products_in_ambiguous_gtin_reports() {
        let upc = gtin("0123456789012");
        let products = ["SKU-1", "SKU-2", "SKU-3"].map(|sku| product(sku, vec![upc]));
        let abc_products: AbcProductsBySku = products
            .into_iter()
            .map(|product| (product.sku(), product))
            .collect();
        let identifiers = HashSet::from([BarcodeWrapper::Gtin(upc)]);

        let result = match_identifiers(&abc_products, &identifiers);

        assert_eq!(result.duplicate_gtins.len(), 1);
        let products = result.duplicate_gtins.get(&upc).unwrap();
        assert_eq!(products.len(), 3);
        for sku in ["SKU-1", "SKU-2", "SKU-3"] {
            assert!(products.contains_key(sku));
        }
    }
}
