//! Strongly-typed identifiers and quantities.
//!
//! Newtypes over `u64` eliminate primitive obsession: the type system forbids passing a
//! [`Qty`] where a [`Price`] is expected. Prices are integer **ticks** (1 tick = 0.01 quote
//! units), which makes ordering exact and branch-free — no floating-point epsilon can ever
//! produce a wrong trade.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Number of quote-currency cents in one price tick. Documented invariant: `Price` values are
/// counts of these ticks, so a `Price(12_345)` means 123.45 quote units.
pub const TICK_CENTS: u64 = 1;

macro_rules! u64_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(pub u64);

        impl $name {
            /// The underlying raw value.
            #[inline]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl From<u64> for $name {
            #[inline]
            fn from(v: u64) -> Self {
                $name(v)
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

u64_newtype!(
    /// Unique identifier for an order, assigned by the client/gateway.
    OrderId
);
u64_newtype!(
    /// Identifies the owning account, used for self-trade prevention.
    AccountId
);
u64_newtype!(
    /// Price expressed in integer ticks (see [`TICK_CENTS`]).
    Price
);

/// A quantity (lots/shares/contracts). Arithmetic is checked to uphold the conservation and
/// non-overflow invariants the matcher relies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Qty(pub u64);

impl Qty {
    pub const ZERO: Qty = Qty(0);

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Saturating-free checked addition. Returns `None` on overflow so callers never silently
    /// wrap (a wrapped quantity would violate conservation).
    #[inline]
    pub fn checked_add(self, rhs: Qty) -> Option<Qty> {
        self.0.checked_add(rhs.0).map(Qty)
    }

    /// Checked subtraction. Returns `None` if `rhs > self` (would underflow).
    #[inline]
    pub fn checked_sub(self, rhs: Qty) -> Option<Qty> {
        self.0.checked_sub(rhs.0).map(Qty)
    }

    /// The smaller of two quantities — the fillable amount between a taker and a maker.
    #[inline]
    pub fn min(self, rhs: Qty) -> Qty {
        Qty(self.0.min(rhs.0))
    }
}

impl From<u64> for Qty {
    #[inline]
    fn from(v: u64) -> Self {
        Qty(v)
    }
}

impl core::fmt::Display for Qty {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Monotonic sequence number assigned by the engine on acceptance. Establishes time priority:
/// at one price level, the lowest `Seq` fills first (FIFO).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Seq(pub u64);

impl Seq {
    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequence value, panicking only on `u64` exhaustion (unreachable in
    /// practice: at 1B orders/sec it would take ~585 years).
    #[inline]
    pub fn next(self) -> Seq {
        Seq(self.0.wrapping_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qty_checked_add_detects_overflow() {
        assert_eq!(Qty(u64::MAX).checked_add(Qty(1)), None);
        assert_eq!(Qty(2).checked_add(Qty(3)), Some(Qty(5)));
    }

    #[test]
    fn qty_checked_sub_detects_underflow() {
        assert_eq!(Qty(3).checked_sub(Qty(5)), None);
        assert_eq!(Qty(5).checked_sub(Qty(3)), Some(Qty(2)));
    }

    #[test]
    fn qty_min_returns_smaller() {
        assert_eq!(Qty(5).min(Qty(3)), Qty(3));
        assert_eq!(Qty(2).min(Qty(9)), Qty(2));
    }

    #[test]
    fn seq_next_increments() {
        assert_eq!(Seq(0).next(), Seq(1));
    }

    #[test]
    fn price_ordering_is_exact() {
        assert!(Price(100) < Price(101));
        assert_eq!(Price::from(42).get(), 42);
    }
}
