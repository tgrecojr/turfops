use crate::models::{Application, ApplicationType};
use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};

/// FRAC (Fungicide Resistance Action Committee) class groupings
/// relevant to residential cool-season turf management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FracClass {
    /// FRAC 1 — Thiophanates (thiophanate-methyl, Cleary's 3336)
    Frac1,
    /// FRAC 3 — DMIs/Triazoles (propiconazole, myclobutanil)
    Frac3,
    /// FRAC 7 — SDHI (fluxapyroxad, penthiopyrad)
    Frac7,
    /// FRAC 11 — Strobilurins (azoxystrobin, pyraclostrobin)
    Frac11,
    /// FRAC 12 — Phenylpyrroles (fludioxonil)
    Frac12,
    /// FRAC 14 — Aromatics (PCNB)
    Frac14,
    /// FRAC M3 — Chlorothalonil (Daconil) — multi-site
    FracM3,
    /// FRAC M5 — Dithiocarbamates (mancozeb) — multi-site
    FracM5,
}

impl FracClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            FracClass::Frac1 => "FRAC 1 (Thiophanates)",
            FracClass::Frac3 => "FRAC 3 (DMIs)",
            FracClass::Frac7 => "FRAC 7 (SDHI)",
            FracClass::Frac11 => "FRAC 11 (Strobilurins)",
            FracClass::Frac12 => "FRAC 12 (Phenylpyrroles)",
            FracClass::Frac14 => "FRAC 14 (Aromatics)",
            FracClass::FracM3 => "FRAC M3 (Chlorothalonil)",
            FracClass::FracM5 => "FRAC M5 (Mancozeb)",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            FracClass::Frac1 => "1",
            FracClass::Frac3 => "3",
            FracClass::Frac7 => "7",
            FracClass::Frac11 => "11",
            FracClass::Frac12 => "12",
            FracClass::Frac14 => "14",
            FracClass::FracM3 => "M3",
            FracClass::FracM5 => "M5",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_uppercase().as_str() {
            "1" => Some(FracClass::Frac1),
            "3" => Some(FracClass::Frac3),
            "7" => Some(FracClass::Frac7),
            "11" => Some(FracClass::Frac11),
            "12" => Some(FracClass::Frac12),
            "14" => Some(FracClass::Frac14),
            "M3" => Some(FracClass::FracM3),
            "M5" => Some(FracClass::FracM5),
            _ => None,
        }
    }

    pub fn common_products(&self) -> &'static [&'static str] {
        match self {
            FracClass::Frac1 => &["thiophanate-methyl", "Cleary's 3336"],
            FracClass::Frac3 => &["propiconazole", "Banner MAXX", "myclobutanil", "Eagle"],
            FracClass::Frac7 => &["fluxapyroxad", "Xzemplar", "penthiopyrad", "Velista"],
            FracClass::Frac11 => &["azoxystrobin", "Heritage", "pyraclostrobin", "Insignia"],
            FracClass::Frac12 => &["fludioxonil", "Medallion"],
            FracClass::Frac14 => &["PCNB", "Turfcide"],
            FracClass::FracM3 => &["chlorothalonil", "Daconil"],
            FracClass::FracM5 => &["mancozeb"],
        }
    }

    /// Multi-site fungicides have low/no resistance risk and are excluded
    /// from rotation calculations.
    pub fn is_multisite(&self) -> bool {
        matches!(self, FracClass::FracM3 | FracClass::FracM5)
    }
}

impl std::fmt::Display for FracClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Attempt to identify a FRAC class from a product name string.
/// Matches against known active ingredients and trade names (case-insensitive).
pub fn frac_class_for_product(name: &str) -> Option<FracClass> {
    let lower = name.to_lowercase();

    // FRAC 1
    if lower.contains("thiophanate") || lower.contains("3336") || lower.contains("cleary") {
        return Some(FracClass::Frac1);
    }

    // FRAC 3
    if lower.contains("propiconazole")
        || lower.contains("banner maxx")
        || lower.contains("myclobutanil")
        || lower.contains("eagle")
    {
        return Some(FracClass::Frac3);
    }

    // FRAC 7
    if lower.contains("fluxapyroxad")
        || lower.contains("xzemplar")
        || lower.contains("penthiopyrad")
        || lower.contains("velista")
    {
        return Some(FracClass::Frac7);
    }

    // FRAC 11
    if lower.contains("azoxystrobin")
        || lower.contains("heritage")
        || lower.contains("pyraclostrobin")
        || lower.contains("insignia")
    {
        return Some(FracClass::Frac11);
    }

    // FRAC 12
    if lower.contains("fludioxonil") || lower.contains("medallion") {
        return Some(FracClass::Frac12);
    }

    // FRAC 14
    if lower.contains("pcnb") || lower.contains("turfcide") {
        return Some(FracClass::Frac14);
    }

    // FRAC M3
    if lower.contains("chlorothalonil") || lower.contains("daconil") {
        return Some(FracClass::FracM3);
    }

    // FRAC M5
    if lower.contains("mancozeb") {
        return Some(FracClass::FracM5);
    }

    None
}

/// Result of analyzing a season's fungicide application history for rotation concerns.
#[derive(Debug, Clone)]
pub struct FungicideRotationAdvice {
    pub total_apps_this_season: usize,
    pub last_class: Option<FracClass>,
    pub consecutive_same_class: usize,
    pub recommended_next: Option<FracClass>,
    pub rotation_warning: Option<String>,
}

/// Analyze fungicide application history for the current season and produce
/// FRAC-class-aware rotation advice.
///
/// Filters to fungicide apps in the current year, resolves product names to
/// FRAC classes, detects consecutive same-class usage (resistance risk at 2+),
/// and recommends the next class to rotate to.
pub fn analyze_fungicide_rotation(history: &[Application]) -> FungicideRotationAdvice {
    let current_year = Local::now().date_naive().year();

    let season_apps: Vec<_> = history
        .iter()
        .filter(|app| {
            app.application_type == ApplicationType::Fungicide
                && app.application_date.year() == current_year
        })
        .collect();

    let total_apps_this_season = season_apps.len();

    if total_apps_this_season == 0 {
        return FungicideRotationAdvice {
            total_apps_this_season: 0,
            last_class: None,
            consecutive_same_class: 0,
            recommended_next: None,
            rotation_warning: None,
        };
    }

    // Resolve each app's FRAC class (skip multi-site for rotation purposes)
    let resolved: Vec<Option<FracClass>> = season_apps
        .iter()
        .map(|app| {
            app.product_name
                .as_deref()
                .and_then(frac_class_for_product)
                .filter(|c| !c.is_multisite())
        })
        .collect();

    // Last single-site class used
    let last_class = resolved.iter().rev().find_map(|c| *c);

    // Count consecutive same-class from the end (most recent first),
    // skipping unknown/multisite entries (None) so they don't break the chain
    let consecutive_same_class = if let Some(last) = last_class {
        resolved
            .iter()
            .rev()
            .filter(|c| c.is_some())
            .take_while(|c| **c == Some(last))
            .count()
    } else {
        0
    };

    // Recommend a different class to rotate to
    let recommended_next = last_class.and_then(recommend_rotation);

    // Build warning if consecutive same-class >= 2
    let rotation_warning = if consecutive_same_class >= 2 {
        let last = last_class.unwrap(); // safe: consecutive >= 2 means last_class is Some
        let mut warning = format!(
            "Your last {} applications were {} — resistance risk increases with consecutive \
             same-class use.",
            consecutive_same_class, last
        );
        if let Some(next) = &recommended_next {
            let products = next.common_products();
            let product_example = products.first().copied().unwrap_or("(see label)");
            warning.push_str(&format!(" Rotate to {} (e.g., {}).", next, product_example));
        }
        Some(warning)
    } else if total_apps_this_season >= 3 && last_class.is_some() {
        // General heads-up at 3+ apps even without consecutive same-class
        let last = last_class.unwrap();
        Some(format!(
            "You have applied fungicide {} times this season (last used: {}). \
             Track FRAC classes to avoid resistance.",
            total_apps_this_season, last
        ))
    } else {
        None
    };

    FungicideRotationAdvice {
        total_apps_this_season,
        last_class,
        consecutive_same_class,
        recommended_next,
        rotation_warning,
    }
}

/// Recommend a single-site FRAC class to rotate to, given the last class used.
/// Prioritizes the most common residential turf classes: 11, 3, 1, 7.
fn recommend_rotation(last: FracClass) -> Option<FracClass> {
    // Rotation order for common residential turf fungicides
    let rotation_order = [
        FracClass::Frac11,
        FracClass::Frac3,
        FracClass::Frac1,
        FracClass::Frac7,
    ];

    rotation_order.iter().find(|c| **c != last).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_code_round_trip() {
        let classes = [
            FracClass::Frac1,
            FracClass::Frac3,
            FracClass::Frac7,
            FracClass::Frac11,
            FracClass::Frac12,
            FracClass::Frac14,
            FracClass::FracM3,
            FracClass::FracM5,
        ];
        for class in &classes {
            let code = class.code();
            let parsed = FracClass::from_code(code).unwrap();
            assert_eq!(*class, parsed, "Round-trip failed for {}", code);
        }
    }

    #[test]
    fn from_code_case_insensitive() {
        assert_eq!(FracClass::from_code("m3"), Some(FracClass::FracM3));
        assert_eq!(FracClass::from_code("M5"), Some(FracClass::FracM5));
        assert_eq!(FracClass::from_code(" 11 "), Some(FracClass::Frac11));
    }

    #[test]
    fn from_code_invalid() {
        assert_eq!(FracClass::from_code("99"), None);
        assert_eq!(FracClass::from_code(""), None);
        assert_eq!(FracClass::from_code("abc"), None);
    }

    #[test]
    fn product_lookup() {
        assert_eq!(
            frac_class_for_product("Heritage TL"),
            Some(FracClass::Frac11)
        );
        assert_eq!(
            frac_class_for_product("Banner MAXX"),
            Some(FracClass::Frac3)
        );
        assert_eq!(
            frac_class_for_product("Daconil Action"),
            Some(FracClass::FracM3)
        );
        assert_eq!(
            frac_class_for_product("Cleary's 3336"),
            Some(FracClass::Frac1)
        );
        assert_eq!(frac_class_for_product("random stuff"), None);
    }

    #[test]
    fn multisite_identification() {
        assert!(FracClass::FracM3.is_multisite());
        assert!(FracClass::FracM5.is_multisite());
        assert!(!FracClass::Frac1.is_multisite());
        assert!(!FracClass::Frac11.is_multisite());
    }

    // --- analyze_fungicide_rotation tests ---

    fn make_fungicide_app(product: Option<&str>, days_ago: i64) -> Application {
        use chrono::Utc;
        let date = Local::now().date_naive() - chrono::Duration::days(days_ago);
        Application {
            id: None,
            lawn_profile_id: 1,
            application_type: ApplicationType::Fungicide,
            product_name: product.map(|s| s.to_string()),
            application_date: date,
            rate_per_1000sqft: None,
            coverage_sqft: None,
            notes: None,
            weather_snapshot: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn rotation_empty_history() {
        let advice = analyze_fungicide_rotation(&[]);
        assert_eq!(advice.total_apps_this_season, 0);
        assert!(advice.last_class.is_none());
        assert_eq!(advice.consecutive_same_class, 0);
        assert!(advice.recommended_next.is_none());
        assert!(advice.rotation_warning.is_none());
    }

    #[test]
    fn rotation_single_known_product() {
        let apps = vec![make_fungicide_app(Some("Heritage TL"), 10)];
        let advice = analyze_fungicide_rotation(&apps);
        assert_eq!(advice.total_apps_this_season, 1);
        assert_eq!(advice.last_class, Some(FracClass::Frac11));
        assert_eq!(advice.consecutive_same_class, 1);
        assert_eq!(advice.recommended_next, Some(FracClass::Frac3));
        assert!(advice.rotation_warning.is_none()); // only 1 consecutive
    }

    #[test]
    fn rotation_consecutive_same_class() {
        let apps = vec![
            make_fungicide_app(Some("Heritage TL"), 30),
            make_fungicide_app(Some("Insignia"), 14),
        ];
        let advice = analyze_fungicide_rotation(&apps);
        assert_eq!(advice.total_apps_this_season, 2);
        assert_eq!(advice.last_class, Some(FracClass::Frac11));
        assert_eq!(advice.consecutive_same_class, 2);
        assert!(advice.rotation_warning.is_some());
        let warning = advice.rotation_warning.unwrap();
        assert!(warning.contains("FRAC 11"));
        assert!(warning.contains("Rotate to"));
    }

    #[test]
    fn rotation_mixed_classes_no_warning() {
        let apps = vec![
            make_fungicide_app(Some("Heritage TL"), 30), // FRAC 11
            make_fungicide_app(Some("Banner MAXX"), 14), // FRAC 3
        ];
        let advice = analyze_fungicide_rotation(&apps);
        assert_eq!(advice.total_apps_this_season, 2);
        assert_eq!(advice.last_class, Some(FracClass::Frac3));
        assert_eq!(advice.consecutive_same_class, 1);
        assert!(advice.rotation_warning.is_none());
    }

    #[test]
    fn rotation_unknown_products_handled() {
        let apps = vec![
            make_fungicide_app(Some("Mystery Spray"), 30),
            make_fungicide_app(Some("Unknown Product"), 14),
        ];
        let advice = analyze_fungicide_rotation(&apps);
        assert_eq!(advice.total_apps_this_season, 2);
        assert!(advice.last_class.is_none()); // can't resolve either
        assert_eq!(advice.consecutive_same_class, 0);
        assert!(advice.rotation_warning.is_none());
    }

    #[test]
    fn rotation_multisite_excluded() {
        // Multi-site fungicides should not count for rotation
        let apps = vec![
            make_fungicide_app(Some("Heritage TL"), 30),    // FRAC 11
            make_fungicide_app(Some("Daconil Action"), 14), // FRAC M3 (multi-site)
        ];
        let advice = analyze_fungicide_rotation(&apps);
        assert_eq!(advice.total_apps_this_season, 2);
        // Last single-site class should be FRAC 11 (Daconil is excluded)
        assert_eq!(advice.last_class, Some(FracClass::Frac11));
        assert_eq!(advice.consecutive_same_class, 1);
    }

    #[test]
    fn rotation_three_apps_general_warning() {
        let apps = vec![
            make_fungicide_app(Some("Heritage TL"), 42),  // FRAC 11
            make_fungicide_app(Some("Banner MAXX"), 21),  // FRAC 3
            make_fungicide_app(Some("Cleary's 3336"), 7), // FRAC 1
        ];
        let advice = analyze_fungicide_rotation(&apps);
        assert_eq!(advice.total_apps_this_season, 3);
        assert_eq!(advice.last_class, Some(FracClass::Frac1));
        assert_eq!(advice.consecutive_same_class, 1); // no consecutive same-class
                                                      // Should still get a general warning at 3+ apps
        assert!(advice.rotation_warning.is_some());
        let warning = advice.rotation_warning.unwrap();
        assert!(warning.contains("3 times this season"));
    }

    #[test]
    fn rotation_recommend_avoids_last_class() {
        // If last was FRAC 11, should recommend FRAC 3 (next in rotation order)
        assert_eq!(
            recommend_rotation(FracClass::Frac11),
            Some(FracClass::Frac3)
        );
        // If last was FRAC 3, should recommend FRAC 11
        assert_eq!(
            recommend_rotation(FracClass::Frac3),
            Some(FracClass::Frac11)
        );
        // If last was FRAC 1, should recommend FRAC 11
        assert_eq!(
            recommend_rotation(FracClass::Frac1),
            Some(FracClass::Frac11)
        );
    }
}
