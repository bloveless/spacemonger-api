-- Add migration script here
ALTER TABLE daemon_user RENAME COLUMN assignment TO new_ship_assignment;
ALTER TABLE daemon_user RENAME COLUMN system_symbol TO new_ship_system_symbol;
ALTER TABLE daemon_user DROP COLUMN location_symbol;
