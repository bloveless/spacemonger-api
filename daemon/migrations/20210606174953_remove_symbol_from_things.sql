-- Add migration script here
ALTER TABLE daemon_market_data RENAME COLUMN location_symbol TO location;
ALTER TABLE daemon_market_data RENAME COLUMN good_symbol TO good;
ALTER TABLE daemon_system_info RENAME COLUMN system_symbol TO system;
ALTER TABLE daemon_system_info RENAME COLUMN location_symbol TO location;
ALTER TABLE daemon_user RENAME COLUMN new_ship_system_symbol TO new_ship_system;
ALTER TABLE daemon_user_ship RENAME COLUMN system_symbol TO system;
ALTER TABLE daemon_user_transaction RENAME COLUMN good_symbol TO good;
ALTER TABLE daemon_user_transaction RENAME COLUMN location_symbol TO location;
