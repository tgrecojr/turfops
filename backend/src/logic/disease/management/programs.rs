//! Static per-disease management knowledge: cultural practices and fungicide class
//! efficacy. Ratings summarize university extension guidance (chiefly Univ. of
//! Kentucky PPA-1, "Chemical Control of Turfgrass Diseases", and NC State TurfFiles).
//! No application rates are given anywhere — rates are product-specific and the label
//! governs.

use crate::models::{Disease, Efficacy, FracClass};

pub(super) struct OptionSpec {
    pub class: FracClass,
    pub efficacy: Efficacy,
    /// Useful once symptoms are present, not just as a protectant.
    pub curative: bool,
    /// Never auto-recommended (e.g. not labeled for residential lawns).
    pub restricted: bool,
    pub note: Option<&'static str>,
}

pub(super) struct ProgramSpec {
    pub cultural: &'static [&'static str],
    /// Ordered by preference; the first eligible top-efficacy option is recommended.
    pub options: &'static [OptionSpec],
    pub preventative_interval: &'static str,
    pub curative_interval: &'static str,
    pub notes: &'static [&'static str],
}

const fn opt(class: FracClass, efficacy: Efficacy, curative: bool) -> OptionSpec {
    OptionSpec {
        class,
        efficacy,
        curative,
        restricted: false,
        note: None,
    }
}

const RESIDENTIAL_NOTE: &str =
    "Chlorothalonil is not labeled for residential lawns in the US — listed only so a \
     logged application counts toward protection.";

const CHLOROTHALONIL: OptionSpec = OptionSpec {
    class: FracClass::FracM3,
    efficacy: Efficacy::Good,
    curative: false,
    restricted: true,
    note: Some(RESIDENTIAL_NOTE),
};

const BROWN_PATCH: ProgramSpec = ProgramSpec {
    cultural: &[
        "Hold nitrogen to 0.5 lb N/1000 sq ft or less per application while nights stay \
         above 60°F — lush growth feeds brown patch.",
        "Water only between 4 and 7 AM, deeply and infrequently, so leaves dry by mid-morning.",
        "Mow at 3.5–4 in with sharp blades, only when the turf is dry.",
        "Improve air movement and morning sun where you can; prune low limbs over trouble spots.",
    ],
    options: &[
        opt(FracClass::Frac11, Efficacy::Excellent, true),
        opt(FracClass::Frac7, Efficacy::Good, true),
        opt(FracClass::Frac3, Efficacy::Good, true),
        opt(FracClass::Frac12, Efficacy::Good, false),
        opt(FracClass::Frac1, Efficacy::Fair, true),
        CHLOROTHALONIL,
    ],
    preventative_interval: "14–28 days",
    curative_interval: "14 days",
    notes: &[
        "Tall fescue usually recovers from brown patch once nights cool — fungicide protects \
         appearance and thin or newly seeded turf rather than saving an established lawn.",
    ],
};

const DOLLAR_SPOT: ProgramSpec = ProgramSpec {
    cultural: &[
        "Keep nitrogen adequate — dollar spot is a low-fertility disease. If it has been more \
         than ~6 weeks since feeding, 0.25–0.5 lb N/1000 sq ft helps turf outgrow it.",
        "Water deeply in the early morning; drought-stressed turf is more susceptible.",
        "Knock dew off early (mow, drag a hose) to shorten leaf wetness.",
        "Mow with sharp blades and avoid mowing wet turf, which spreads the fungus.",
    ],
    options: &[
        opt(FracClass::Frac3, Efficacy::Excellent, true),
        opt(FracClass::Frac7, Efficacy::Excellent, true),
        OptionSpec {
            class: FracClass::Frac1,
            efficacy: Efficacy::Good,
            curative: true,
            restricted: false,
            note: Some("Resistance to thiophanate-methyl is widespread; don't lean on it."),
        },
        CHLOROTHALONIL,
    ],
    preventative_interval: "14–28 days",
    curative_interval: "14 days",
    notes: &[
        "Strobilurins (FRAC 11, e.g. azoxystrobin) are weak on dollar spot — don't count a \
         brown patch spray as dollar spot protection.",
        "The Smith-Kerns 20% threshold was developed for golf turf; on a home lawn, treat it \
         as a prompt to scout before spraying.",
    ],
};

const PYTHIUM_BLIGHT: ProgramSpec = ProgramSpec {
    cultural: &[
        "Fix drainage and avoid evening irrigation — Pythium needs saturated turf.",
        "Skip nitrogen during hot, humid stretches; 0.25 lb N/1000 sq ft at most.",
        "Don't mow or walk wet turf when mycelium is visible; it spreads along mower tracks \
         and drainage paths.",
        "Treated seed or a preventative matters most on new seedlings, the most vulnerable turf.",
    ],
    options: &[
        opt(FracClass::Frac4, Efficacy::Excellent, true),
        opt(FracClass::Frac21, Efficacy::Excellent, true),
        opt(FracClass::Frac28, Efficacy::Good, true),
        OptionSpec {
            class: FracClass::FracP07,
            efficacy: Efficacy::Good,
            curative: false,
            restricted: false,
            note: Some("Preventative only — no curative activity."),
        },
        OptionSpec {
            class: FracClass::Frac11,
            efficacy: Efficacy::Fair,
            curative: false,
            restricted: false,
            note: Some("Suppression only; not a substitute for a Pythium-specific product."),
        },
    ],
    preventative_interval: "10–21 days",
    curative_interval: "5–10 days",
    notes: &[
        "Brown patch and dollar spot fungicides (FRAC 3, 7, 1) do nothing against Pythium — \
         it needs its own chemistry.",
        "Pythium moves fast: if you see greasy, water-soaked patches or cottony growth at \
         dawn, treat the same day.",
    ],
};

const GRAY_LEAF_SPOT: ProgramSpec = ProgramSpec {
    cultural: &[
        "Avoid quick-release nitrogen in late summer; keep it to 0.25–0.5 lb N/1000 sq ft.",
        "Water early morning only and keep seedlings from staying wet overnight.",
        "Bag clippings while the disease is active to cut down on spores.",
        "When overseeding, pick gray-leaf-spot-resistant cultivars and seed after the late \
         August peak where possible.",
    ],
    options: &[
        opt(FracClass::Frac1, Efficacy::Excellent, true),
        OptionSpec {
            class: FracClass::Frac11,
            efficacy: Efficacy::Excellent,
            curative: true,
            restricted: false,
            note: Some(
                "Strobilurin-resistant gray leaf spot is documented — never apply twice in a row.",
            ),
        },
        opt(FracClass::Frac3, Efficacy::Fair, false),
        CHLOROTHALONIL,
    ],
    preventative_interval: "14–21 days",
    curative_interval: "10–14 days",
    notes: &["Seedling turf can be lost in days; established tall fescue is far more tolerant."],
};

const RED_THREAD: ProgramSpec = ProgramSpec {
    cultural: &[
        "Feed the lawn: 0.5 lb N/1000 sq ft usually lets turf outgrow red thread in 2–3 weeks.",
        "Check soil pH and potassium — red thread is worse on under-nourished turf.",
        "Bag clippings while it is active and mow when dry.",
        "Water early morning; reduce shade and improve air flow where practical.",
    ],
    options: &[
        opt(FracClass::Frac11, Efficacy::Excellent, true),
        opt(FracClass::Frac7, Efficacy::Good, true),
        opt(FracClass::Frac3, Efficacy::Good, true),
    ],
    preventative_interval: "14–28 days",
    curative_interval: "14–21 days",
    notes: &[
        "Red thread is cosmetic — it blights leaves but does not kill crowns or roots. \
         Fungicide is rarely justified on a home lawn.",
    ],
};

pub(super) fn program_for(disease: Disease) -> &'static ProgramSpec {
    match disease {
        Disease::BrownPatch => &BROWN_PATCH,
        Disease::DollarSpot => &DOLLAR_SPOT,
        Disease::PythiumBlight => &PYTHIUM_BLIGHT,
        Disease::GrayLeafSpot => &GRAY_LEAF_SPOT,
        Disease::RedThread => &RED_THREAD,
    }
}
