-- Replace the ASC index from migration 2 with a DESC version for recent-first queries.
DROP INDEX IF EXISTS idx_environmental_cache_timestamp;
CREATE INDEX idx_environmental_cache_timestamp ON environmental_cache(timestamp DESC);
