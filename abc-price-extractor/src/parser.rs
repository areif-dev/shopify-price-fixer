pub fn skus_from_214(taboutput_214: &[u8]) -> Result<Vec<String>, csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b'\t')
        .flexible(true)
        .from_reader(taboutput_214);

    let mut skus: Vec<String> = Vec::new();
    for result in rdr.records() {
        let row = result?;
        match row.get(4) {
            Some("") | None | Some(" ") => (),
            Some(s) => {
                if s.trim() == "" {
                    continue;
                }
                skus.push(s.to_string());
            }
        }
    }

    Ok(skus)
}
