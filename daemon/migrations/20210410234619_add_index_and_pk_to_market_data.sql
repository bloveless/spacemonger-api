CREATE INDEX IF NOT EXISTS idx_market_data_location_symbol_good_symbol ON daemon.market_data (
    location_symbol, good_symbol
);

ALTER TABLE market_data ADD COLUMN IF NOT EXISTS id SERIAL PRIMARY KEY ;
