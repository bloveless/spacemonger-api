-- Add migration script here
ALTER TABLE daemon_flight_plans RENAME TO daemon_flight_plan;
ALTER TABLE daemon_users RENAME TO daemon_user;
ALTER TABLE daemon_users_ships RENAME TO daemon_user_ship;
