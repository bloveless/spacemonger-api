-- Add migration script here
ALTER TABLE daemon_user_stats ADD COLUMN ship_count INT NOT NULL DEFAULT(0);
ALTER TABLE daemon_user_stats ADD COLUMN ships jsonb NULL;
