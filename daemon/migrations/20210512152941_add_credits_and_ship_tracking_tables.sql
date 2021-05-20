CREATE TABLE daemon_users_credits (
     user_id uuid NOT NULL
    ,credits INT NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX daemon_users_credits_user_id ON daemon_users_credits (user_id);

CREATE TABLE daemon_users_ships (
     user_id uuid NOT NULL
    ,ship_id VARCHAR(50) NOT NULL
    ,type VARCHAR(50) NOT NULL
    ,class VARCHAR(50) NOT NULL
    ,max_cargo INT NOT NULL
    ,speed INT NOT NULL
    ,manufacturer VARCHAR(50) NOT NULL
    ,plating INT NOT NULL
    ,weapons INT NOT NULL
);

CREATE INDEX daemon_users_ships_user_id_ship_id ON daemon_users_ships (user_id, ship_id);
