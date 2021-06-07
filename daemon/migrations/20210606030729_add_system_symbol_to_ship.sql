-- Add migration script here
ALTER TABLE daemon_user_ship ADD COLUMN system_symbol varchar(100) NOT NULL DEFAULT('OE');
