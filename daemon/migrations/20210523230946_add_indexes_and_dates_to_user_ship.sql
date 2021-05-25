ALTER INDEX daemon_users_ships_user_id_ship_id RENAME TO daemon_user_ship_user_id_ship_id;
CREATE INDEX daemon_user_ship_user_id ON daemon_user_ship(user_id);

ALTER TABLE daemon_user_ship ADD COLUMN modified_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL;
ALTER TABLE daemon_user_ship ADD COLUMN created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL;
