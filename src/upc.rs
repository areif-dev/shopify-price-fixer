use serde::{Deserialize, Deserializer};

#[derive(Debug)]
pub enum UpcError {
    InvalidLength,
    NonNumericCharacter,
}

impl std::fmt::Display for UpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for UpcError {}

#[derive(Debug, Clone, Deserialize)]
pub struct Upc {
    upc: [u8; 12],
}

// Custom deserialization function for `Upc`
pub fn deserialize_optional_upc<'de, D>(deserializer: D) -> Result<Option<Upc>, D::Error>
where
    D: Deserializer<'de>,
{
    let barcode_str_opt = Option::<String>::deserialize(deserializer)?;
    match barcode_str_opt {
        Some(s) => Upc::try_from(s).map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

impl Upc {
    fn normalize(unformatted: &str) -> Option<Upc> {
        let mut upc: Vec<u8> = unformatted
            .chars()
            .filter_map(|c| {
                if let Some(d) = c.to_digit(10) {
                    return Some(d as u8);
                }
                None
            })
            .collect();

        // If there are more than 12 bytes to the upc, ignore the first several digits, as these
        // are most likely leading and extra 0s
        if upc.len() > 12 {
            upc = Vec::from(upc.get(upc.len() - 12..)?);
        }
        Upc::fix_check_digit(&upc)
    }

    fn fix_check_digit(upc: &[u8]) -> Option<Upc> {
        let mut fixed = [0u8; 12];
        let mut sum1 = 0;
        let mut sum2 = 0;
        for i in 0..11 {
            let digit = upc.get(i)?;
            if i % 2 == 0 {
                sum1 += digit;
            } else {
                sum2 += digit;
            }
            fixed[i] = *digit;
        }
        let sum_total = sum1 * 3 + sum2;
        let mut checkd = 10 - sum_total % 10;
        if checkd == 10 {
            checkd = 0;
        }
        fixed[11] = checkd;
        Some(Upc { upc: fixed })
    }

    pub fn from_abc_upc_list(abc_upc_list: &str) -> Vec<Option<Upc>> {
        abc_upc_list.split(",").map(|s| Upc::normalize(s)).collect()
    }

    pub fn try_from_str_like<S>(string_like: S) -> Result<Upc, UpcError>
    where
        S: Into<String>,
    {
        let s: String = string_like.into();

        let mut upc_bytes = Vec::new();
        for c in s.chars() {
            match c.to_digit(10) {
                Some(digit) => upc_bytes.push(digit as u8),
                None => return Err(UpcError::NonNumericCharacter),
            }
        }
        // If there are more than 12 bytes to the upc, ignore the first several digits, as these
        // are most likely leading and extra 0s
        if upc_bytes.len() > 12 {
            upc_bytes = Vec::from(
                upc_bytes
                    .get(upc_bytes.len() - 12..)
                    .ok_or(UpcError::InvalidLength)?,
            );
        }
        let upc = Upc::fix_check_digit(&upc_bytes).ok_or(UpcError::InvalidLength)?;
        Ok(upc)
    }
}

impl ToString for Upc {
    fn to_string(&self) -> String {
        let bytes = self.upc;
        bytes.map(|c| c.to_string()).join("")
    }
}

impl TryFrom<String> for Upc {
    type Error = UpcError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Upc::try_from_str_like(value)
    }
}

impl TryFrom<&str> for Upc {
    type Error = UpcError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Upc::try_from_str_like(value)
    }
}
