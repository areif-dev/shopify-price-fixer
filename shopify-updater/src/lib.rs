use std::collections::HashMap;

pub fn parse_report_1_15(taboutput_1_15: &str) -> HashMap<String, u32> {
    let lines = taboutput_1_15.lines().map(|line| line.split('\t'));
    let mut prices = HashMap::new();
    for l in lines {
        let line: Vec<&str> = l.collect();
        let sku = match line.get(0) {
            Some(s) => s,
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
