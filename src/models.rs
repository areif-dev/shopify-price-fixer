use async_trait::async_trait;
use sqlx_axum_mvc as sam;

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

impl ToOwned for Upc {
    type Owned = Upc;

    fn to_owned(&self) -> Self::Owned {
        let bytes = self.upc;
        Self::Owned { upc: bytes }
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
    abc_product_id: i64,
    sku: String,
    desc: String,
    upc: Upc,
    list: i64,
    cost: i64,
    stock: f64,
}

impl AbcProduct {
    pub fn abc_product_id(&self) -> i64 {
        self.abc_product_id
    }

    pub fn sku(&self) -> String {
        self.sku.clone()
    }

    pub fn desc(&self) -> String {
        self.desc.clone()
    }

    pub fn upc(&self) -> Upc {
        self.upc.to_owned()
    }

    pub fn list(&self) -> i64 {
        self.list
    }

    pub fn cost(&self) -> i64 {
        self.cost
    }

    pub fn stock(&self) -> f64 {
        self.stock
    }
}

#[async_trait]
impl sam::SqliteDbModel for AbcProduct {
    type Error = sqlx::Error;

    fn table_name() -> String {
        "AbcProduct".to_string()
    }

    fn map_cols_to_vals(&self) -> sam::ColumnValueMap {
        sam::ColumnValueMap::from([
            (
                "abc_product_id".to_string(),
                sam::BasicType::Integer(self.abc_product_id),
            ),
            ("sku".to_string(), sam::BasicType::Text(self.sku.clone())),
            ("desc".to_string(), sam::BasicType::Text(self.desc.clone())),
            (
                "upc".to_string(),
                sam::BasicType::Text(self.upc.to_string()),
            ),
            ("list".to_string(), sam::BasicType::Integer(self.list)),
            ("cost".to_string(), sam::BasicType::Integer(self.cost)),
            ("stock".to_string(), sam::BasicType::Real(self.stock)),
        ])
    }

    async fn create_table(pool: &sqlx::SqlitePool) -> Result<(), Self::Error> {
        let query_str = format!(
            r"create table if not exists {} (
                abc_product_id integer primary key,
                sku text not null,
                desc text not null,
                upc text not null,
                list integer not null,
                cost integer not null,
                stock real not null
            );",
            Self::table_name()
        );

        sqlx::query(&query_str).execute(pool).await?;
        Ok(())
    }
}

pub struct AbcProductBuilder {
    abc_product_id: Option<i64>,
    sku: Option<String>,
    desc: Option<String>,
    upc: Option<Upc>,
    list: Option<i64>,
    cost: Option<i64>,
    stock: Option<f64>,
}

impl AbcProductBuilder {
    pub fn new() -> Self {
        AbcProductBuilder {
            abc_product_id: None,
            sku: None,
            desc: None,
            upc: None,
            list: None,
            cost: None,
            stock: None,
        }
    }

    pub fn with_abc_product_id(self, id: i64) -> Self {
        AbcProductBuilder {
            abc_product_id: Some(id),
            ..self
        }
    }

    pub fn with_sku(self, sku: &str) -> Self {
        AbcProductBuilder {
            sku: Some(sku.to_string()),
            ..self
        }
    }

    pub fn with_desc(self, desc: &str) -> Self {
        AbcProductBuilder {
            desc: Some(desc.to_string()),
            ..self
        }
    }

    pub fn with_upc(self, upc: Upc) -> Self {
        AbcProductBuilder {
            upc: Some(upc),
            ..self
        }
    }

    pub fn with_list(self, list: i64) -> Self {
        AbcProductBuilder {
            list: Some(list),
            ..self
        }
    }

    pub fn with_cost(self, cost: i64) -> Self {
        AbcProductBuilder {
            cost: Some(cost),
            ..self
        }
    }

    pub fn with_stock(self, stock: f64) -> Self {
        AbcProductBuilder {
            stock: Some(stock),
            ..self
        }
    }

    pub fn build(self) -> Option<AbcProduct> {
        Some(AbcProduct {
            abc_product_id: self.abc_product_id?,
            sku: self.sku.clone()?,
            desc: self.desc.clone()?,
            upc: self.upc?,
            list: self.list?,
            cost: self.cost?,
            stock: self.stock?,
        })
    }
}
