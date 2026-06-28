//! [`StockOption`] — a buyable choice drawn ONLY from the supplier catalog.
//!
//! A thin *contextual* selection wrapper: it names which catalog SKU satisfies which spec and
//! whether it is currently buyable. It stores NO intrinsic SKU data — length, pack, price,
//! supplier all live on [`materials::SupplierSku`] / [`materials::PriceQuote`] and are read
//! through `sku_ref` by the [`StockCatalog`](crate::ports::StockCatalog) port, so the flyweight is
//! never copied.

use materials::{SkuKey, SpecKey};

/// A buyable choice the solver may pick — the constraint set, no imaginary stock.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StockOption {
    /// → [`materials::SupplierSku`] flyweight. Source of truth for usable length, pack, price.
    pub sku_ref: SkuKey,
    /// → the [`materials::StockSpec`] this option satisfies. Must equal `Demand.spec_ref`.
    pub spec_ref: SpecKey,
    /// Derived buyability gate from the SKU's availability at solve time: an out-of-stock SKU
    /// cannot be an option.
    pub buyable: bool,
}

impl StockOption {
    /// A buyable option matching `spec_ref` to `sku_ref`.
    pub fn buyable(sku_ref: SkuKey, spec_ref: SpecKey) -> StockOption {
        StockOption {
            sku_ref,
            spec_ref,
            buyable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buyable_option_matches_its_spec() {
        let o = StockOption::buyable(SkuKey::from("HD-2x4-8-SPF"), SpecKey::from("SPF-STUD"));
        assert!(o.buyable);
        assert_eq!(o.spec_ref, SpecKey::from("SPF-STUD"));
    }
}
