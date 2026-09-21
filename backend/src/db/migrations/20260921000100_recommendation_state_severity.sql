-- "Addressed" / "Dismissed" used to hide a recommendation id forever. Record the severity
-- that was answered so an escalation (Info → Warning, High → Severe) can bring the alert
-- back; expiry uses the existing updated_at. NULL = answered before this was tracked.
ALTER TABLE recommendation_states ADD COLUMN IF NOT EXISTS severity TEXT;
