-- One-time cleanup mirroring ProductFacts::scoped_to_category (models/product/profile.rs):
-- facts that cannot apply to a product's category were stored from the LLM's guesses
-- (a herbicide with a FRAC class, a fungicide with nutrient / species targets). They never
-- matched a need — the category gate came first — but they are noise, not facts.
-- `Other` is left alone: the user is not second-guessed on a catch-all.

UPDATE products SET frac_classes = '{}'
WHERE category NOT IN ('Fungicide', 'Other') AND frac_classes <> '{}';

UPDATE products SET herbicide_timing = NULL
WHERE category NOT IN ('Herbicide', 'Other') AND herbicide_timing IS NOT NULL;

UPDATE products SET amendment_kind = NULL
WHERE category NOT IN ('SoilAmendment', 'Other') AND amendment_kind IS NOT NULL;

UPDATE products SET targets = '{}'
WHERE category IN ('Fungicide', 'SoilAmendment', 'Surfactant') AND targets <> '{}';

UPDATE products SET targets = ARRAY(
    SELECT t FROM unnest(targets) AS t
    WHERE t IN ('Grubs', 'SurfaceInsects', 'Ants', 'Termites', 'Ticks', 'Mosquitoes')
)
WHERE category = 'InsectControl';

UPDATE products SET targets = ARRAY(
    SELECT t FROM unnest(targets) AS t
    WHERE t IN ('Broadleaf', 'Crabgrass', 'Nutsedge', 'Poa', 'Moss')
)
WHERE category = 'Herbicide';

UPDATE products SET targets = ARRAY(
    SELECT t FROM unnest(targets) AS t
    WHERE t IN ('Iron', 'Manganese', 'Magnesium', 'Zinc', 'Boron', 'Copper', 'Calcium')
)
WHERE category IN ('Fertilizer', 'Supplement');

UPDATE products SET targets = ARRAY(
    SELECT t FROM unnest(targets) AS t
    WHERE t IN ('Kbg', 'Ttf', 'Prg', 'FineFescue', 'Bermuda', 'Zoysia')
)
WHERE category = 'Seed';
