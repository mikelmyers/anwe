// -----------------------------------------------------------------
// ANWE v0.1 -- TYPED UNCERTAINTY
//
// Values carry confidence intervals. Not a wrapper. Native.
//
// 0.87 ± 0.12 is a first-class type.
// "I believe X with 75% confidence" is not a log message —
// it's the actual data structure.
//
// Arithmetic on uncertain values propagates uncertainty.
// Addition: margins combine in quadrature (root sum of squares).
// Multiplication: relative margins add.
// Comparison: overlapping ranges are indeterminate.
//
// This is how a cognitive system represents partial knowledge.
// Not as a definite value it secretly doubts, but as a range
// it honestly reports.
// -----------------------------------------------------------------

use core::fmt;

/// A value with typed uncertainty.
///
/// Stored as value ± margin with a confidence level.
/// Compact: 8 bytes total (fits in a u64 or two u32s).
///
/// The margin represents one standard deviation.
/// The confidence is how much of the system stands behind this.
#[derive(Clone, Copy, PartialEq)]
pub struct Uncertain {
    /// The central estimate.
    pub value: f32,
    /// The margin of uncertainty (± this amount).
    pub margin: f32,
}

impl Uncertain {
    /// Create an uncertain value: value ± margin.
    pub fn new(value: f32, margin: f32) -> Self {
        Uncertain {
            value,
            margin: margin.abs(),
        }
    }

    /// Create an exact value (zero uncertainty).
    pub fn exact(value: f32) -> Self {
        Uncertain { value, margin: 0.0 }
    }

    /// Create a maximally uncertain value.
    /// "I have no idea, but my best guess is X."
    pub fn guess(value: f32) -> Self {
        Uncertain {
            value,
            margin: value.abs().max(0.5),
        }
    }

    /// Lower bound of the confidence interval.
    #[inline]
    pub fn low(&self) -> f32 {
        self.value - self.margin
    }

    /// Upper bound of the confidence interval.
    #[inline]
    pub fn high(&self) -> f32 {
        self.value + self.margin
    }

    /// Confidence: 1.0 - relative_margin.
    /// Exact values have confidence 1.0.
    /// Highly uncertain values approach 0.0.
    #[inline]
    pub fn confidence(&self) -> f32 {
        if self.value == 0.0 {
            if self.margin == 0.0 { 1.0 } else { 0.0 }
        } else {
            (1.0 - self.margin / self.value.abs()).max(0.0)
        }
    }

    /// Do two uncertain values have overlapping ranges?
    /// If they overlap, they might be the same value.
    pub fn overlaps(&self, other: &Uncertain) -> bool {
        self.low() <= other.high() && other.low() <= self.high()
    }

    /// Is this value definitely greater than another?
    /// Only true if our lower bound exceeds their upper bound.
    pub fn definitely_greater(&self, other: &Uncertain) -> bool {
        self.low() > other.high()
    }

    /// Is this value definitely less than another?
    pub fn definitely_less(&self, other: &Uncertain) -> bool {
        self.high() < other.low()
    }

    /// Narrow the uncertainty by incorporating new evidence.
    /// Bayesian-style update: weighted average of estimates.
    pub fn refine(&self, observation: Uncertain) -> Uncertain {
        // Inverse-variance weighting
        let w1 = if self.margin > 0.0 {
            1.0 / (self.margin * self.margin)
        } else {
            f32::MAX / 2.0
        };
        let w2 = if observation.margin > 0.0 {
            1.0 / (observation.margin * observation.margin)
        } else {
            f32::MAX / 2.0
        };

        let total_w = w1 + w2;
        let new_value = (w1 * self.value + w2 * observation.value) / total_w;
        let new_margin = (1.0 / total_w).sqrt();

        Uncertain::new(new_value, new_margin)
    }

    /// Widen the uncertainty (e.g., due to time passing or noise).
    pub fn widen(&self, additional_margin: f32) -> Uncertain {
        Uncertain::new(
            self.value,
            (self.margin * self.margin + additional_margin * additional_margin).sqrt(),
        )
    }

    /// Pack into a u64 for storage in signal data fields.
    /// Value in upper 32 bits, margin in lower 32 bits.
    pub fn pack(&self) -> u64 {
        let v = self.value.to_bits() as u64;
        let m = self.margin.to_bits() as u64;
        (v << 32) | m
    }

    /// Unpack from a u64.
    pub fn unpack(packed: u64) -> Self {
        let v = f32::from_bits((packed >> 32) as u32);
        let m = f32::from_bits(packed as u32);
        Uncertain::new(v, m)
    }
}

// ─── ARITHMETIC ──────────────────────────────────────────────
//
// Uncertainty propagates through operations.
// This is not optional. If you add two uncertain values,
// the result is uncertain. The math enforces this.

impl core::ops::Add for Uncertain {
    type Output = Uncertain;

    /// Addition: values add, margins combine in quadrature.
    fn add(self, rhs: Uncertain) -> Uncertain {
        Uncertain {
            value: self.value + rhs.value,
            margin: (self.margin * self.margin + rhs.margin * rhs.margin).sqrt(),
        }
    }
}

impl core::ops::Sub for Uncertain {
    type Output = Uncertain;

    /// Subtraction: values subtract, margins still combine in quadrature.
    fn sub(self, rhs: Uncertain) -> Uncertain {
        Uncertain {
            value: self.value - rhs.value,
            margin: (self.margin * self.margin + rhs.margin * rhs.margin).sqrt(),
        }
    }
}

impl core::ops::Mul for Uncertain {
    type Output = Uncertain;

    /// Multiplication: relative margins add.
    fn mul(self, rhs: Uncertain) -> Uncertain {
        let value = self.value * rhs.value;
        let rel_a = if self.value != 0.0 { self.margin / self.value.abs() } else { 0.0 };
        let rel_b = if rhs.value != 0.0 { rhs.margin / rhs.value.abs() } else { 0.0 };
        let rel_combined = (rel_a * rel_a + rel_b * rel_b).sqrt();
        Uncertain {
            value,
            margin: value.abs() * rel_combined,
        }
    }
}

impl core::ops::Mul<f32> for Uncertain {
    type Output = Uncertain;

    /// Scalar multiplication: margin scales proportionally.
    fn mul(self, scalar: f32) -> Uncertain {
        Uncertain {
            value: self.value * scalar,
            margin: self.margin * scalar.abs(),
        }
    }
}

impl fmt::Debug for Uncertain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3} ± {:.3} (conf: {:.1}%)",
            self.value, self.margin, self.confidence() * 100.0)
    }
}

impl fmt::Display for Uncertain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3} ± {:.3}", self.value, self.margin)
    }
}

// ─── TESTS ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_value_has_no_uncertainty() {
        let v = Uncertain::exact(0.87);
        assert_eq!(v.margin, 0.0);
        assert_eq!(v.confidence(), 1.0);
        assert_eq!(v.low(), 0.87);
        assert_eq!(v.high(), 0.87);
    }

    #[test]
    fn uncertain_value_has_range() {
        let v = Uncertain::new(0.87, 0.12);
        assert!((v.low() - 0.75).abs() < 0.001);
        assert!((v.high() - 0.99).abs() < 0.001);
    }

    #[test]
    fn overlapping_ranges() {
        let a = Uncertain::new(0.5, 0.2);  // 0.3 - 0.7
        let b = Uncertain::new(0.6, 0.2);  // 0.4 - 0.8
        assert!(a.overlaps(&b));
        assert!(!a.definitely_greater(&b));
    }

    #[test]
    fn non_overlapping_ranges() {
        let a = Uncertain::new(1.0, 0.1);  // 0.9 - 1.1
        let b = Uncertain::new(0.5, 0.1);  // 0.4 - 0.6
        assert!(!a.overlaps(&b));
        assert!(a.definitely_greater(&b));
        assert!(b.definitely_less(&a));
    }

    #[test]
    fn addition_propagates_uncertainty() {
        let a = Uncertain::new(1.0, 0.1);
        let b = Uncertain::new(2.0, 0.1);
        let sum = a + b;
        assert!((sum.value - 3.0).abs() < 0.001);
        // sqrt(0.01 + 0.01) ≈ 0.1414
        assert!((sum.margin - 0.1414).abs() < 0.01);
    }

    #[test]
    fn multiplication_propagates_uncertainty() {
        let a = Uncertain::new(2.0, 0.1);
        let b = Uncertain::new(3.0, 0.15);
        let product = a * b;
        assert!((product.value - 6.0).abs() < 0.001);
        // Relative margins: 0.05 and 0.05, combined ≈ 0.0707
        // Absolute margin: 6.0 * 0.0707 ≈ 0.424
        assert!(product.margin > 0.3 && product.margin < 0.6);
    }

    #[test]
    fn scalar_multiplication() {
        let a = Uncertain::new(1.0, 0.1);
        let scaled = a * 2.0;
        assert!((scaled.value - 2.0).abs() < 0.001);
        assert!((scaled.margin - 0.2).abs() < 0.001);
    }

    #[test]
    fn refinement_narrows_uncertainty() {
        let prior = Uncertain::new(10.0, 2.0);
        let observation = Uncertain::new(11.0, 1.0);
        let refined = prior.refine(observation);
        // Refined value should be between 10 and 11, closer to 11
        assert!(refined.value > 10.0 && refined.value < 11.0);
        // Refined margin should be less than either input
        assert!(refined.margin < prior.margin);
        assert!(refined.margin < observation.margin);
    }

    #[test]
    fn pack_unpack_roundtrip() {
        let v = Uncertain::new(0.87, 0.12);
        let packed = v.pack();
        let unpacked = Uncertain::unpack(packed);
        assert!((unpacked.value - v.value).abs() < 0.0001);
        assert!((unpacked.margin - v.margin).abs() < 0.0001);
    }

    #[test]
    fn widen_increases_uncertainty() {
        let v = Uncertain::new(1.0, 0.1);
        let widened = v.widen(0.1);
        assert!(widened.margin > v.margin);
    }
}
