//! Static per-disease management knowledge: cultural practices and fungicide class
//! efficacy. No application rates are given anywhere — rates are product-specific and
//! the label governs.
//!
//! Efficacy was checked against Univ. of Kentucky PPA-1, "Chemical Control of Turfgrass
//! Diseases 2024" (Clarke, Vincelli, Koch & Chou), whose per-disease tables rate each
//! active 1–4. Mapping used here: 4 / 3.5 → Excellent, 3 / 2.5 → Good, 2 / 1.5 → Fair,
//! 1 → not listed. The rating noted beside each option is the PPA-1 value for that
//! class's common actives.
//!
//! Known limit: efficacy varies *within* a FRAC class (pyraclostrobin is 3.5 on dollar
//! spot while azoxystrobin is unlisted and can enhance it), but options are per class.
//! Where a class is split like that it is left out of the disease's options, which errs
//! toward not claiming protection.

use crate::models::{Disease, Efficacy, FracClass};

pub(super) struct OptionSpec {
    pub class: FracClass,
    pub efficacy: Efficacy,
    /// Useful once symptoms are present, not just as a protectant.
    pub curative: bool,
    /// Never auto-recommended (e.g. not labeled for residential lawns).
    pub restricted: bool,
    /// Whether a logged application still counts as protection under Severe pressure.
    pub protects_at_severe: bool,
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
        protects_at_severe: true,
        note: None,
    }
}

const RESIDENTIAL_NOTE: &str =
    "Chlorothalonil is not labeled for residential lawns in the US — listed only so a \
     logged application counts toward protection.";

// PPA-1: brown patch 3, dollar spot 3, gray leaf spot 2.5.
const CHLOROTHALONIL: OptionSpec = OptionSpec {
    class: FracClass::FracM5,
    efficacy: Efficacy::Good,
    curative: false,
    restricted: true,
    protects_at_severe: true,
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
        // azoxystrobin 4 (2-wk) / 3 (4-wk), pyraclostrobin 4
        opt(FracClass::Frac11, Efficacy::Excellent, true),
        // penthiopyrad 4, fluxapyroxad 3.5
        opt(FracClass::Frac7, Efficacy::Excellent, true),
        // propiconazole 3, myclobutanil 2.5
        opt(FracClass::Frac3, Efficacy::Good, true),
        // fludioxonil 3
        opt(FracClass::Frac12, Efficacy::Good, false),
        // thiophanate-methyl 2.5
        opt(FracClass::Frac1, Efficacy::Good, true),
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
        // propiconazole 4, myclobutanil 3
        opt(FracClass::Frac3, Efficacy::Excellent, true),
        // fluxapyroxad 4, penthiopyrad 3.5 (1–2 where SDHI-resistant isolates are present)
        OptionSpec {
            class: FracClass::Frac7,
            efficacy: Efficacy::Excellent,
            curative: true,
            restricted: false,
            protects_at_severe: true,
            note: Some("SDHI-resistant dollar spot has been reported; rotate classes."),
        },
        // thiophanate-methyl 4, but 1 where resistant — and resistance is common in the US
        OptionSpec {
            class: FracClass::Frac1,
            efficacy: Efficacy::Good,
            curative: true,
            restricted: false,
            protects_at_severe: true,
            note: Some("Resistance to thiophanate-methyl is widespread; don't lean on it."),
        },
        CHLOROTHALONIL,
    ],
    preventative_interval: "14–28 days",
    curative_interval: "14 days",
    notes: &[
        "Azoxystrobin (Heritage) has no useful dollar spot activity and can even make it \
         worse, so a Heritage spray for brown patch is not dollar spot protection. Other \
         strobilurins differ (pyraclostrobin is rated good to excellent), which is why \
         FRAC 11 as a class is not listed here.",
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
        // cyazofamid 3.5
        opt(FracClass::Frac21, Efficacy::Excellent, true),
        // propamocarb 3.5
        opt(FracClass::Frac28, Efficacy::Excellent, true),
        // mefenoxam 3, metalaxyl 2.5
        OptionSpec {
            class: FracClass::Frac4,
            efficacy: Efficacy::Good,
            curative: true,
            restricted: false,
            protects_at_severe: true,
            note: Some(
                "Pythium resistance to mefenoxam/metalaxyl is a significant risk — don't use \
                 it back to back.",
            ),
        },
        // fosetyl-Al 3 (PPA-1 lists it under its former FRAC code, 33)
        OptionSpec {
            class: FracClass::FracP07,
            efficacy: Efficacy::Good,
            curative: false,
            restricted: false,
            protects_at_severe: true,
            note: Some("Preventative only — no curative activity."),
        },
        // azoxystrobin 3, fluoxastrobin 3, pyraclostrobin 2.5. PPA-1 adds that under high
        // pressure cyazofamid, mefenoxam and propamocarb are the most efficacious, so a
        // strobilurin is not trusted as protection once pressure is Severe.
        OptionSpec {
            class: FracClass::Frac11,
            efficacy: Efficacy::Good,
            curative: false,
            restricted: false,
            protects_at_severe: false,
            note: Some(
                "Good preventative activity, but under severe pressure rely on a \
                 Pythium-specific product (cyazofamid, propamocarb, mefenoxam).",
            ),
        },
    ],
    preventative_interval: "10–21 days",
    curative_interval: "5–10 days",
    notes: &[
        "DMI, SDHI and thiophanate fungicides (FRAC 3, 7, 1) do nothing against Pythium. \
         A strobilurin (FRAC 11) gives good preventative cover at High risk, but not \
         enough to rely on when pressure is Severe.",
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
        // thiophanate-methyl 4
        opt(FracClass::Frac1, Efficacy::Excellent, true),
        // azoxystrobin / pyraclostrobin 3.5, but 1 where QoI-resistant strains are present
        OptionSpec {
            class: FracClass::Frac11,
            efficacy: Efficacy::Excellent,
            curative: true,
            restricted: false,
            protects_at_severe: true,
            note: Some(
                "Strobilurin-resistant gray leaf spot is documented — never apply twice in a row.",
            ),
        },
        // propiconazole 2
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
        // azoxystrobin 4, pyraclostrobin 4
        opt(FracClass::Frac11, Efficacy::Excellent, true),
        // penthiopyrad 4 (fluxapyroxad alone is not rated)
        opt(FracClass::Frac7, Efficacy::Excellent, true),
        // propiconazole 3, myclobutanil 2
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
