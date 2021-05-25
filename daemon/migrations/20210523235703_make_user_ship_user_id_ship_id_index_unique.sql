DROP INDEX daemon_user_ship_user_id_ship_id;
CREATE UNIQUE INDEX daemon_user_ship_user_id_ship_id ON daemon_user_ship(user_id, ship_id);
