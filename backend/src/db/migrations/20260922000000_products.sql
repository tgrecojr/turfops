-- Product inventory: what is on the shelf, with user-confirmed facts the recommendation
-- engine can match on (category, FRAC classes, herbicide timing, targets, amendment kind)
-- and an optional LLM-generated profile. No quantities by design — stock is a hand-set status.
CREATE TABLE IF NOT EXISTS products (
    id BIGSERIAL PRIMARY KEY,
    lawn_profile_id BIGINT NOT NULL REFERENCES lawn_profiles(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    brand TEXT,
    category TEXT NOT NULL,
    form TEXT NOT NULL,
    stock_status TEXT NOT NULL DEFAULT 'InStock',
    -- The user's label rate (suggested by the LLM, confirmed by the user). Pre-fills the
    -- application form only; never appears in advice text.
    label_rate_per_1000sqft DOUBLE PRECISION,
    label_rate_unit TEXT,
    nitrogen_pct DOUBLE PRECISION,
    phosphorus_pct DOUBLE PRECISION,
    potassium_pct DOUBLE PRECISION,
    frac_classes TEXT[] NOT NULL DEFAULT '{}',
    herbicide_timing TEXT,
    targets TEXT[] NOT NULL DEFAULT '{}',
    amendment_kind TEXT,
    -- LLM profile (informational). NULL = entered by hand.
    profile JSONB,
    profile_generated_at TIMESTAMPTZ,
    profile_model TEXT,
    notes TEXT,
    archived BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_product_category CHECK (
        category IN ('Fertilizer', 'Supplement', 'Fungicide', 'Herbicide', 'InsectControl',
                     'Seed', 'SoilAmendment', 'Surfactant', 'Other')
    ),
    CONSTRAINT chk_product_form CHECK (
        form IN ('Granular', 'Liquid', 'WaterSoluble', 'Seed', 'Other')
    ),
    CONSTRAINT chk_product_stock_status CHECK (stock_status IN ('InStock', 'Low', 'Out')),
    CONSTRAINT chk_product_label_rate_unit CHECK (
        label_rate_unit IS NULL OR label_rate_unit IN ('Lb', 'Oz', 'FlOz')
    ),
    CONSTRAINT chk_product_label_rate CHECK (
        (label_rate_per_1000sqft IS NULL) = (label_rate_unit IS NULL)
        AND (label_rate_per_1000sqft IS NULL OR label_rate_per_1000sqft > 0)
    ),
    CONSTRAINT chk_product_herbicide_timing CHECK (
        herbicide_timing IS NULL OR herbicide_timing IN ('PreEmergent', 'PostEmergent', 'Both')
    ),
    CONSTRAINT chk_product_amendment_kind CHECK (
        amendment_kind IS NULL OR amendment_kind IN ('CalciticLime', 'DolomiticLime', 'Sulfur',
                                                     'Gypsum', 'Humic', 'Compost', 'Other')
    ),
    CONSTRAINT chk_product_npk CHECK (
        (nitrogen_pct IS NULL OR nitrogen_pct BETWEEN 0 AND 100)
        AND (phosphorus_pct IS NULL OR phosphorus_pct BETWEEN 0 AND 100)
        AND (potassium_pct IS NULL OR potassium_pct BETWEEN 0 AND 100)
    ),
    CONSTRAINT chk_product_profile_provenance CHECK (
        (profile IS NULL) = (profile_generated_at IS NULL)
        AND (profile IS NULL) = (profile_model IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_products_lawn_profile_id ON products(lawn_profile_id);
