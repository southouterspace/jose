//! The purchasable-SKU + price-snapshot catalog: [`SupplierSku`], [`PriceQuote`],
//! [`PriceTier`]. Money is the single representation in the schema — real decimal USD (`f64`),
//! never integer cents.

use crate::keys::{SkuKey, SpecKey};
use geometry_kernel::Tick;
use reference_data::Flyweight;

/// An ISO-8601 timestamp string for the stored (not live) price-snapshot model. Kept as a thin
/// newtype to stay dependency-free.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Timestamp(pub String);

impl From<&str> for Timestamp {
    fn from(s: &str) -> Self {
        Timestamp(s.to_owned())
    }
}

/// How a SKU is sold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnitOfSale {
    /// Sold individually.
    Each,
    /// Sold as a bundle of `pack_size`.
    Bundle,
}

/// One bulk-break price step: at or above `min_qty`, the per-unit price is `price_per`. Ascending
/// tiers on a [`PriceQuote`] express the volume discounts the estimator applies.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PriceTier {
    /// Inclusive lower-bound quantity (in `unitOfSale`) at which this tier applies.
    pub min_qty: u32,
    /// Per-unit price (real USD) at this tier — never integer cents.
    pub price_per: f64,
}

/// An immutable stored price snapshot for one [`SupplierSku`], including bulk-break tiers. THE
/// single money representation in the schema — real USD. A re-quote is a new value.
#[derive(Clone, PartialEq, Debug)]
pub struct PriceQuote {
    /// ISO currency, e.g. `USD`.
    pub currency: String,
    /// Price per `unitOfSale` at qty 1 — real money (USD), never integer cents.
    pub unit_price: f64,
    /// Ascending break pricing; lets bulk/volume cost models attach without a schema change.
    pub bulk_tiers: Vec<PriceTier>,
    /// Snapshot timestamp; freshness for the stored model.
    pub as_of: Timestamp,
    /// Store/market the price applies to, e.g. `US-TX`.
    pub region: Option<String>,
}

impl PriceQuote {
    /// The applicable per-unit price for purchasing `qty` units, honoring the highest bulk tier
    /// whose `min_qty` is satisfied (else the base `unit_price`).
    pub fn price_for_quantity(&self, qty: u32) -> f64 {
        self.bulk_tiers
            .iter()
            .filter(|t| qty >= t.min_qty)
            .max_by_key(|t| t.min_qty)
            .map(|t| t.price_per)
            .unwrap_or(self.unit_price)
    }
}

/// A purchasable SKU from one supplier: the flyweight mapping a [`StockSpec`](crate::StockSpec)
/// to a real product with packaging and a price snapshot. The bottom-up estimate prices the
/// takeoff against these.
#[derive(Clone, PartialEq, Debug)]
pub struct SupplierSku {
    /// Opaque flyweight key, e.g. `HD-2x4-8-SPF`.
    pub key: SkuKey,
    /// `Home Depot` | `Lowe's` | local yard.
    pub supplier: String,
    /// Supplier's own SKU/part number.
    pub sku: String,
    /// Product page.
    pub url: Option<String>,
    /// Which [`StockSpec`](crate::StockSpec) this product satisfies — the join to the model.
    pub matches_spec: SpecKey,
    /// As-sold length in ticks (linear stock); constrains the cut optimizer. The one linear field.
    pub stock_length: Tick,
    /// How it is sold.
    pub unit_of_sale: UnitOfSale,
    /// Units per bundle — needed for packaging/waste rounding before purchase.
    pub pack_size: Option<u32>,
    /// Current price snapshot — the single money representation.
    pub price: PriceQuote,
    /// Whether the supplier reports it in stock.
    pub in_stock: Option<bool>,
    /// Freshness of the stored price.
    pub last_price_check: Option<Timestamp>,
}

impl Flyweight for SupplierSku {
    type Key = SkuKey;
    fn flyweight_key(&self) -> SkuKey {
        self.key.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote() -> PriceQuote {
        PriceQuote {
            currency: "USD".to_owned(),
            unit_price: 3.98,
            bulk_tiers: vec![
                PriceTier {
                    min_qty: 50,
                    price_per: 3.49,
                },
                PriceTier {
                    min_qty: 250,
                    price_per: 2.99,
                },
            ],
            as_of: Timestamp::from("2026-06-28"),
            region: Some("US-TX".to_owned()),
        }
    }

    #[test]
    fn bulk_tiers_apply_by_quantity() {
        let q = quote();
        assert_eq!(q.price_for_quantity(1), 3.98);
        assert_eq!(q.price_for_quantity(49), 3.98);
        assert_eq!(q.price_for_quantity(50), 3.49);
        assert_eq!(q.price_for_quantity(300), 2.99);
    }

    #[test]
    fn sku_is_a_flyweight_pointing_at_a_spec() {
        let sku = SupplierSku {
            key: SkuKey::from("HD-2x4-8-SPF"),
            supplier: "Home Depot".to_owned(),
            sku: "161640".to_owned(),
            url: None,
            matches_spec: SpecKey::from("SPF-STUD-SDRY"),
            stock_length: Tick(3072),
            unit_of_sale: UnitOfSale::Each,
            pack_size: None,
            price: quote(),
            in_stock: Some(true),
            last_price_check: Some(Timestamp::from("2026-06-28")),
        };
        assert_eq!(sku.flyweight_key(), SkuKey::from("HD-2x4-8-SPF"));
        assert_eq!(sku.matches_spec, SpecKey::from("SPF-STUD-SDRY"));
    }
}
