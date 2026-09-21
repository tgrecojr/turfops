use crate::models::Application;
use serde::{Deserialize, Serialize};

/// FRAC (Fungicide Resistance Action Committee) class groupings
/// relevant to residential cool-season turf management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FracClass {
    /// FRAC 1 — Thiophanates (thiophanate-methyl, Cleary's 3336)
    Frac1,
    /// FRAC 3 — DMIs/Triazoles (propiconazole, myclobutanil)
    Frac3,
    /// FRAC 4 — Phenylamides (mefenoxam) — Pythium-specific
    Frac4,
    /// FRAC 7 — SDHI (fluxapyroxad, penthiopyrad)
    Frac7,
    /// FRAC 11 — Strobilurins (azoxystrobin, pyraclostrobin)
    Frac11,
    /// FRAC 12 — Phenylpyrroles (fludioxonil)
    Frac12,
    /// FRAC 14 — Aromatics (PCNB)
    Frac14,
    /// FRAC 21 — QiI (cyazofamid) — Pythium-specific
    Frac21,
    /// FRAC 28 — Carbamates (propamocarb) — Pythium-specific
    Frac28,
    /// FRAC P07 — Phosphonates (fosetyl-Al, phosphites) — Pythium preventative
    FracP07,
    /// FRAC M3 — Dithiocarbamates (mancozeb) — multi-site
    FracM3,
    /// FRAC M5 — Chloronitriles (chlorothalonil, Daconil) — multi-site
    FracM5,
}

impl FracClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            FracClass::Frac1 => "FRAC 1 (Thiophanates)",
            FracClass::Frac3 => "FRAC 3 (DMIs)",
            FracClass::Frac4 => "FRAC 4 (Phenylamides)",
            FracClass::Frac7 => "FRAC 7 (SDHI)",
            FracClass::Frac11 => "FRAC 11 (Strobilurins)",
            FracClass::Frac12 => "FRAC 12 (Phenylpyrroles)",
            FracClass::Frac14 => "FRAC 14 (Aromatics)",
            FracClass::Frac21 => "FRAC 21 (QiI)",
            FracClass::Frac28 => "FRAC 28 (Carbamates)",
            FracClass::FracP07 => "FRAC P07 (Phosphonates, formerly 33)",
            FracClass::FracM3 => "FRAC M3 (Mancozeb)",
            FracClass::FracM5 => "FRAC M5 (Chlorothalonil)",
        }
    }

    pub fn common_products(&self) -> &'static [&'static str] {
        match self {
            FracClass::Frac1 => &["thiophanate-methyl", "Cleary's 3336"],
            FracClass::Frac3 => &["propiconazole", "Banner MAXX", "myclobutanil", "Eagle 20EW"],
            FracClass::Frac4 => &["mefenoxam", "Subdue MAXX"],
            FracClass::Frac7 => &["fluxapyroxad", "Xzemplar", "penthiopyrad", "Velista"],
            FracClass::Frac11 => &["azoxystrobin", "Heritage", "pyraclostrobin", "Insignia"],
            FracClass::Frac12 => &["fludioxonil", "Medallion"],
            FracClass::Frac14 => &["PCNB", "Turfcide"],
            FracClass::Frac21 => &["cyazofamid", "Segway"],
            FracClass::Frac28 => &["propamocarb", "Banol"],
            FracClass::FracP07 => &["fosetyl-Al", "Signature", "phosphite"],
            FracClass::FracM3 => &["mancozeb"],
            FracClass::FracM5 => &["chlorothalonil", "Daconil"],
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

/// Name fragments (lower-case) → FRAC classes. Premixes list every class they contain and
/// come first so their trade name wins over nothing; actives follow in the historical
/// lookup order. Only products whose actives are certain belong here — an unknown product
/// resolves to nothing and the form asks the user to pick the class from the label.
const PRODUCT_PATTERNS: &[(&str, &[FracClass])] = &[
    // Premixes
    ("headway", &[FracClass::Frac11, FracClass::Frac3]), // azoxystrobin + propiconazole
    ("pillar", &[FracClass::Frac11, FracClass::Frac3]),  // pyraclostrobin + triticonazole
    ("armada", &[FracClass::Frac11, FracClass::Frac3]),  // trifloxystrobin + triadimefon
    // FRAC 1
    ("thiophanate", &[FracClass::Frac1]),
    ("3336", &[FracClass::Frac1]),
    ("cleary", &[FracClass::Frac1]),
    // FRAC 3
    ("propiconazole", &[FracClass::Frac3]),
    ("banner maxx", &[FracClass::Frac3]),
    ("myclobutanil", &[FracClass::Frac3]),
    ("eagle 20", &[FracClass::Frac3]),
    ("immunox", &[FracClass::Frac3]),
    ("tebuconazole", &[FracClass::Frac3]),
    ("triticonazole", &[FracClass::Frac3]),
    ("metconazole", &[FracClass::Frac3]),
    ("triadimefon", &[FracClass::Frac3]),
    ("bayleton", &[FracClass::Frac3]),
    ("bioadvanced fungus", &[FracClass::Frac3]), // propiconazole
    // FRAC 4
    ("mefenoxam", &[FracClass::Frac4]),
    ("metalaxyl", &[FracClass::Frac4]),
    ("subdue", &[FracClass::Frac4]),
    // FRAC 21
    ("cyazofamid", &[FracClass::Frac21]),
    ("segway", &[FracClass::Frac21]),
    // FRAC 28
    ("propamocarb", &[FracClass::Frac28]),
    ("banol", &[FracClass::Frac28]),
    // FRAC P07
    ("fosetyl", &[FracClass::FracP07]),
    ("phosphite", &[FracClass::FracP07]),
    ("signature", &[FracClass::FracP07]),
    // FRAC 7
    ("fluxapyroxad", &[FracClass::Frac7]),
    ("xzemplar", &[FracClass::Frac7]),
    ("penthiopyrad", &[FracClass::Frac7]),
    ("velista", &[FracClass::Frac7]),
    // FRAC 11
    ("azoxy", &[FracClass::Frac11]), // azoxystrobin, "Azoxy 2SC"
    ("heritage", &[FracClass::Frac11]),
    ("diseaseex", &[FracClass::Frac11]), // Scotts DiseaseEx = azoxystrobin
    ("disease ex", &[FracClass::Frac11]),
    ("pyraclostrobin", &[FracClass::Frac11]),
    ("insignia", &[FracClass::Frac11]),
    ("trifloxystrobin", &[FracClass::Frac11]),
    ("fluoxastrobin", &[FracClass::Frac11]),
    // FRAC 12
    ("fludioxonil", &[FracClass::Frac12]),
    ("medallion", &[FracClass::Frac12]),
    // FRAC 14
    ("pcnb", &[FracClass::Frac14]),
    ("turfcide", &[FracClass::Frac14]),
    // FRAC M3
    ("mancozeb", &[FracClass::FracM3]),
    // FRAC M5
    ("chlorothalonil", &[FracClass::FracM5]),
    ("daconil", &[FracClass::FracM5]),
];

/// Every FRAC class a product name resolves to (case-insensitive), in table order.
/// A premix — by trade name or by listing both actives — yields more than one.
pub fn frac_classes_for_product(name: &str) -> Vec<FracClass> {
    let lower = name.to_lowercase();
    let mut classes = Vec::new();
    for (fragment, found) in PRODUCT_PATTERNS {
        if lower.contains(fragment) {
            for class in *found {
                if !classes.contains(class) {
                    classes.push(*class);
                }
            }
        }
    }
    classes
}

/// The first FRAC class a product name resolves to.
#[cfg(test)]
pub fn frac_class_for_product(name: &str) -> Option<FracClass> {
    frac_classes_for_product(name).into_iter().next()
}

/// FRAC classes of a logged fungicide: what the user recorded from the label, otherwise
/// whatever the product name resolves to. Empty = unknown, and an unknown fungicide earns
/// no protection and no rotation credit.
pub fn classes_of(app: &Application) -> Vec<FracClass> {
    match &app.frac_classes {
        Some(recorded) if !recorded.is_empty() => recorded.clone(),
        _ => app
            .product_name
            .as_deref()
            .map(frac_classes_for_product)
            .unwrap_or_default(),
    }
}

mod rotation;
pub use rotation::*;

#[cfg(test)]
mod tests;
