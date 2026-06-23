//! Shared I/Q quality math used by both the RX task (trend history) and the
//! Lab IQ panels, so the displayed IRR and the logged/trended IRR can never drift
//! apart.

/// Image Rejection Ratio from I/Q amplitude and phase imbalance.
///
/// Exact formula for a direct-conversion quadrature receiver:
///   IRR = 10·log₁₀( (1 + α² + 2α·cosθ) / (1 + α² − 2α·cosθ) )
/// where α = linear amplitude ratio, θ = phase error in radians.
/// Returns 99.9 dB when imbalances are negligible (den ≈ 0 → IRR → ∞).
pub fn image_rejection_db(amp_imbalance_db: f32, phase_imbalance_deg: f32) -> f64 {
    let alpha = 10f64.powf(amp_imbalance_db as f64 / 20.0);
    let theta = phase_imbalance_deg as f64 * std::f64::consts::PI / 180.0;
    let num = 1.0 + alpha * alpha + 2.0 * alpha * theta.cos();
    let den = 1.0 + alpha * alpha - 2.0 * alpha * theta.cos();
    if den <= 1e-12 { return 99.9; }
    10.0 * (num / den).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irr_perfect_balance_is_high() {
        // α=1, θ=0 → den=0 → clamped to 99.9 dB
        let irr = image_rejection_db(0.0, 0.0);
        assert!((irr - 99.9).abs() < 0.01, "got {irr:.1}");
    }

    #[test]
    fn irr_amp_only_0_5db() {
        // 0.5 dB amplitude imbalance, 0° phase → IRR ≈ 32 dB
        let irr = image_rejection_db(0.5, 0.0);
        assert!(irr > 30.0 && irr < 35.0, "expected ~32 dB, got {irr:.1}");
    }

    #[test]
    fn irr_phase_only_2deg() {
        // 0 dB amplitude, 2° phase imbalance ≈ 35.2 dB
        let irr = image_rejection_db(0.0, 2.0);
        assert!(irr > 34.0 && irr < 37.0, "expected ~35.2 dB, got {irr:.1}");
    }

    #[test]
    fn irr_worsens_with_more_imbalance() {
        let irr_low  = image_rejection_db(0.5, 1.0);
        let irr_high = image_rejection_db(3.0, 5.0);
        assert!(irr_low > irr_high, "more imbalance should give worse IRR");
    }
}
