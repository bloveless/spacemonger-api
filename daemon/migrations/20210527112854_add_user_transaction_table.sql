-- Add migration script here
CREATE TABLE daemon_user_transaction (
     user_id uuid NOT NULL
    ,ship_id VARCHAR(50) NOT NULL
    ,type VARCHAR(50) NOT NULL
    ,good_symbol VARCHAR(50) NOT NULL
    ,price_per_unit INT NOT NULL
    ,quantity INT NOT NULL
    ,total INT NOT NULL
    ,location_symbol VARCHAR(100) NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
)
