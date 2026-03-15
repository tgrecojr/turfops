-- Persist recommendation dismissed/addressed states across restarts
CREATE TABLE IF NOT EXISTS recommendation_states (
    id TEXT PRIMARY KEY,
    dismissed BOOLEAN NOT NULL DEFAULT FALSE,
    addressed BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
