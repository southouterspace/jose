//! [`StockCatalog`] — the port the solver reads SKU facts through.
//!
//! Keeps flyweight discipline mechanical: [`StockOption`](crate::StockOption) stores no length or
//! price, so the solver must resolve them by `sku_ref` against this port. The composition root
//! implements it over the materials catalog ([`materials::SupplierSku`] / `PriceQuote`); tests
//! implement it with a small in-memory map.

use geometry_kernel::Tick;
use materials::SkuKey;

/// Resolves the intrinsic, by-reference facts of a buyable SKU for the solver.
pub trait StockCatalog {
    /// Usable stock length of the SKU in ticks (after any supplier-side trim), or `None` if the
    /// SKU is unknown.
    fn stock_length(&self, sku: &SkuKey) -> Option<Tick>;

    /// Pack size the SKU is sold in (sticks per pack); `1` if sold individually.
    fn pack_size(&self, sku: &SkuKey) -> u32 {
        let _ = sku;
        1
    }

    /// Effective unit price for buying `count` sticks of the SKU (USD, break-tier aware), or
    /// `None` if the SKU is unknown / unpriced. Default leaves the plan unpriced.
    fn unit_price(&self, sku: &SkuKey, count: u32) -> Option<f64> {
        let _ = (sku, count);
        None
    }
}
