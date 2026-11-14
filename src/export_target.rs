use abc_product::AbcProduct;

pub trait ExportTarget {
    fn export(products: Vec<AbcProduct>);
}
