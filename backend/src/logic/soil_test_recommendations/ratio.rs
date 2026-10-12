//! The N-P-K advice's ratio, example analysis and wording.

use crate::models::{NutrientLevel, SoilTest};

/// What the test says about P and K: the levels it reports, and which it did not test.
pub(super) fn levels_text(
    test: &SoilTest,
    p_level: NutrientLevel,
    k_level: NutrientLevel,
) -> String {
    let mut reported = Vec::new();
    if let Some(p_ppm) = test.phosphorus_ppm {
        reported.push(format!(
            "{} P ({p_ppm:.0} ppm)",
            p_level.as_str().to_lowercase()
        ));
    }
    if let Some(k_ppm) = test.potassium_ppm {
        reported.push(format!(
            "{} K ({k_ppm:.0} ppm)",
            k_level.as_str().to_lowercase()
        ));
    }
    let untested = match (test.phosphorus_ppm, test.potassium_ppm) {
        (None, None) => "Phosphorus and potassium were not tested, so no P₂O₅ or K₂O is advised.",
        (None, Some(_)) => "Phosphorus was not tested, so no P₂O₅ is advised.",
        (Some(_), None) => "Potassium was not tested, so no K₂O is advised.",
        (Some(_), Some(_)) => "",
    };
    let shows = if reported.is_empty() {
        String::new()
    } else {
        format!("Your soil test shows {}.", reported.join(" and "))
    };
    [shows.as_str(), untested]
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// "0.75 lbs N, 0.25 lbs P₂O₅ and 0.50 lbs K₂O" — the non-zero amounts only.
pub(super) fn amounts_text(n: f64, p: f64, k: f64) -> String {
    let parts: Vec<String> = [(n, "N"), (p, "P₂O₅"), (k, "K₂O")]
        .iter()
        .filter(|(amount, _)| *amount > 0.0)
        .map(|(amount, nutrient)| format!("{amount:.2} lbs {nutrient}"))
        .collect();
    match parts.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => parts.join(""),
    }
}

/// Pounds of the example analysis per 1,000 sq ft (to 0.1 lb) that supply the advised N,
/// or with no N the advised P₂O₅, or else the advised K₂O.
// @spec SOIL-NPK-009, SOIL-NPK-010
pub(super) fn example_product_rate(product_ratio: &str, n: f64, p: f64, k: f64) -> f64 {
    let pct = |i: usize| -> f64 {
        product_ratio
            .split('-')
            .nth(i)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0)
    };
    [(n, pct(0)), (p, pct(1)), (k, pct(2))]
        .into_iter()
        .find(|(amount, product_pct)| *amount > 0.0 && *product_pct > 0.0)
        // Rounded once here so the page and the explanation show the same tenth.
        .map_or(0.0, |(amount, product_pct)| {
            (amount / (product_pct / 100.0) * 10.0).round() / 10.0
        })
}

// @spec SOIL-NPK-007, SOIL-NPK-008
/// Suggest a simple N-P-K ratio string and a common product ratio.
pub(super) fn suggest_ratio(n: f64, p: f64, k: f64) -> (String, String) {
    // Normalize to smallest non-zero value to get a simple ratio
    let min_nonzero = [n, p, k]
        .iter()
        .copied()
        .filter(|v| *v > 0.0)
        .fold(f64::MAX, f64::min);

    let rn = if n > 0.0 {
        (n / min_nonzero).round() as u32
    } else {
        0
    };
    let rp = if p > 0.0 {
        (p / min_nonzero).round() as u32
    } else {
        0
    };
    let rk = if k > 0.0 {
        (k / min_nonzero).round() as u32
    } else {
        0
    };

    let ratio = format!("{}-{}-{}", rn, rp, rk);

    // Map to common commercial product ratios
    let product = match (rn > 0, rp > 0, rk > 0) {
        (true, true, true) => {
            if rn >= 2 * rk {
                "24-4-12"
            } else {
                "12-4-8"
            }
        }
        (true, true, false) => "15-5-0",
        (true, false, true) => {
            if rn >= 2 * rk {
                "24-0-11"
            } else {
                "16-0-8"
            }
        }
        (true, false, false) => "46-0-0",
        (false, true, true) => "0-5-10",
        (false, true, false) => "0-20-0",
        (false, false, true) => "0-0-50",
        (false, false, false) => "N/A",
    };

    (ratio, product.to_string())
}
