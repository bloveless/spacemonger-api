ALTER TABLE daemon_marketplace_latest ADD COLUMN modified_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL;
