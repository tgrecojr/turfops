-- Link an application to the shelf product it used. product_name stays as a copy taken at
-- log time so history survives a rename or delete; product_id is what the inventory reads.
ALTER TABLE applications ADD COLUMN IF NOT EXISTS product_id BIGINT REFERENCES products(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_applications_product_id ON applications(product_id);
