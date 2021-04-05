CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
     id UUID DEFAULT uuid_generate_v4()
    ,username VARCHAR(100) NOT NULL UNIQUE
    ,token VARCHAR(100) NOT NULL
    ,assignment VARCHAR(50) NOT NULL
    ,location VARCHAR(50) NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE market_data (
     planet_symbol VARCHAR(100) NOT NULL
    ,good_symbol VARCHAR(100) NOT NULL
    ,price_per_unit INT NOT NULL
    ,volume_per_unit INT NOT NULL
    ,available INT NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE flight_plans (
     flight_plan_id VARCHAR(100) NOT NULL PRIMARY KEY
    ,user_id UUID NOT NULL
    ,ship_id VARCHAR(100) NOT NULL
    ,origin VARCHAR(100) NOT NULL
    ,destination VARCHAR(100) NOT NULL
    ,ship_cargo_volume INT NOT NULL
    ,ship_cargo_volume_max INT NOT NULL
    ,distance INT NOT NULL
    ,fuel_consumed INT NOT NULL
    ,fuel_remaining INT NOT NULL
    ,time_remaining_in_seconds INT NOT NULL
    ,arrives_at TIMESTAMP WITH TIME ZONE NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE system_info (
     system VARCHAR(100) NOT NULL
    ,system_name VARCHAR(100) NOT NULL
    ,location VARCHAR(100) NOT NULL
    ,location_name VARCHAR(100) NOT NULL
    ,location_type VARCHAR(100) NOT NULL
    ,x INT NOT NULL
    ,y INT NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX idx_system_info_system_location
    ON system_info (system, location);

ALTER TABLE system_info
    ADD CONSTRAINT unique_system_info_system_location
    UNIQUE USING INDEX idx_system_info_system_location;
