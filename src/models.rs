fn fix_check_digit(upc: &[u8]) -> Option<[u8; 12]> {
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
    Some(fixed)
}

#[derive(Debug)]
pub enum UpcError {
    InvalidLength,
    NonNumericCharacter,
}

pub struct Upc {
    upc: [u8; 12],
}

impl Upc {
    pub fn try_from_str_like<S>(string_like: S) -> Result<Upc, UpcError>
    where
        S: AsRef<str>,
    {
        let s = string_like.as_ref();

        let mut upc_bytes = [0u8; 12];
        for (i, c) in s.chars().enumerate() {
            match c.to_digit(10) {
                Some(digit) => upc_bytes[i] = digit as u8,
                None => return Err(UpcError::NonNumericCharacter),
            }
        }
        let upc_bytes = fix_check_digit(&upc_bytes).ok_or(UpcError::InvalidLength)?;
        Ok(Upc { upc: upc_bytes })
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

pub struct AbcProduct {
    sku: String,
    desc: String,
    upc: Upc,
    list: i64,
    cost: i64,
    stock: f64,
}

