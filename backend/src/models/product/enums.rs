//! Controlled vocabularies for the product inventory. Every variant name doubles as the
//! serde name, the database string and the JSON-schema enum value the LLM must pick from,
//! so `as_str()` / `FromStr` / serde all agree by construction.
use serde::{Deserialize, Serialize};

macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $($(#[$vmeta])* $variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn as_str(&self) -> &'static str {
                match self { $($name::$variant => stringify!($variant)),+ }
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|v| v.as_str().eq_ignore_ascii_case(s.trim()))
                    .ok_or_else(|| format!("Unknown {}: {}", stringify!($name), s))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

string_enum! {
    /// What a product is for. Drives which recommendations it can satisfy and which
    /// application types it can be logged under.
    ProductCategory {
        /// Granular / liquid N-P-K, organics, starter fertilizer
        Fertilizer,
        /// Iron, micronutrients, humic / kelp / biostimulants ("nutrient supplements")
        Supplement,
        /// Anything with a FRAC class
        Fungicide,
        /// Pre-emergent, post-emergent or non-selective; `HerbicideTiming` says which
        Herbicide,
        /// Grub preventatives / curatives, surface insects, termite, repellents
        InsectControl,
        /// Grass seed; `ProductTarget` carries the species mix
        Seed,
        /// Lime, sulfur, gypsum, compost, humic; `AmendmentKind` says which
        SoilAmendment,
        /// Wetting agents, spreader-stickers, adjuvants
        Surfactant,
        Other,
    }
}

string_enum! {
    ProductForm { Granular, Liquid, WaterSoluble, Seed, Other }
}

string_enum! {
    /// Hand-set stock level. `Out` means "I use this and need to rebuy" (feeds the shopping
    /// list); an archived product is one no longer used and is hidden everywhere.
    StockStatus { InStock, Low, Out }
}

string_enum! {
    HerbicideTiming { PreEmergent, PostEmergent, Both }
}

string_enum! {
    AmendmentKind { CalciticLime, DolomiticLime, Sulfur, Gypsum, Humic, Compost, Other }
}

string_enum! {
    /// Unit of a per-1,000 sq ft label rate.
    RateUnit { Lb, Oz, FlOz }
}

string_enum! {
    Mobility { Systemic, Contact, NotApplicable }
}

string_enum! {
    NitrogenRelease { Quick, Slow, Mixed, NotApplicable }
}

string_enum! {
    SuitableFor { Turf, Ornamentals, Both }
}

string_enum! {
    /// What a product controls, supplies or contains. A closed list so recommendations can
    /// match on it; free text would make matching impossible.
    ProductTarget {
        Grubs, SurfaceInsects, Ants, Termites, Ticks, Mosquitoes,
        Broadleaf, Crabgrass, Nutsedge, Poa, Moss,
        Iron, Manganese, Magnesium, Zinc, Boron, Copper, Calcium,
        Kbg, Ttf, Prg, FineFescue, Bermuda, Zoysia,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn as_str_matches_serde_name() {
        for c in ProductCategory::ALL {
            let json = serde_json::to_value(c).unwrap();
            assert_eq!(json.as_str().unwrap(), c.as_str());
        }
    }

    #[test]
    fn from_str_is_case_insensitive_and_rejects_unknown() {
        assert_eq!(
            ProductCategory::from_str("insectcontrol"),
            Ok(ProductCategory::InsectControl)
        );
        assert_eq!(StockStatus::from_str(" Low "), Ok(StockStatus::Low));
        assert!(ProductTarget::from_str("weeds").is_err());
    }
}
