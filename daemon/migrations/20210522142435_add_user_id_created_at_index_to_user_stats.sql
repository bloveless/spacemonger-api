-- Add migration script here
CREATE INDEX IF NOT EXISTS daemon_user_stats_user_id_created_at on daemon_user_stats (user_id, created_at);
