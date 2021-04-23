CREATE TABLE daemon_users (
     id UUID DEFAULT uuid_generate_v4()
    ,username VARCHAR(100) NOT NULL UNIQUE
    ,token VARCHAR(100) NOT NULL
    ,assignment VARCHAR(50) NOT NULL
    ,system_symbol VARCHAR(50) NULL
    ,location_symbol VARCHAR(50) NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE daemon_market_data (
     id SERIAL NOT NULL PRIMARY KEY
    ,location_symbol VARCHAR(100) NOT NULL
    ,good_symbol VARCHAR(100) NOT NULL
    ,price_per_unit INT NOT NULL
    ,volume_per_unit INT NOT NULL
    ,quantity_available INT NOT NULL
    ,purchase_price_per_unit INT NOT NULL
    ,sell_price_per_unit INT NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_market_data_location_symbol_good_symbol ON daemon_market_data (
    location_symbol, good_symbol
);

CREATE TABLE daemon_flight_plans (
     id VARCHAR(100) NOT NULL PRIMARY KEY
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

CREATE TABLE daemon_system_info (
     system_symbol VARCHAR(100) NOT NULL
    ,system_name VARCHAR(100) NOT NULL
    ,location_symbol VARCHAR(100) NOT NULL
    ,location_name VARCHAR(100) NOT NULL
    ,location_type VARCHAR(100) NOT NULL
    ,x INT NOT NULL
    ,y INT NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
    ,PRIMARY KEY(system_symbol, location_symbol)
);
