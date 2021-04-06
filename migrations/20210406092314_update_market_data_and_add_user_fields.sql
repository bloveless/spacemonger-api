ALTER TABLE users RENAME COLUMN location to location_symbol;
ALTER TABLE users ADD COLUMN system_symbol VARCHAR(50) NULL;


ALTER TABLE market_data RENAME COLUMN planet_symbol to location_symbol;
ALTER TABLE market_data ADD COLUMN spread INT NOT NULL;
ALTER TABLE market_data ADD COLUMN purchase_price_per_unit INT NOT NULL;
ALTER TABLE market_data ADD COLUMN sell_price_per_unit INT NOT NULL;
ALTER TABLE market_data RENAME COLUMN available to quantity_available;
