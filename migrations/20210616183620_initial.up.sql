CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

--

CREATE TABLE public.daemon_flight_plan (
     id VARCHAR(100) NOT NULL PRIMARY KEY
    ,user_id uuid NOT NULL
    ,ship_id VARCHAR(100) NOT NULL
    ,origin VARCHAR(100) NOT NULL
    ,destination VARCHAR(100) NOT NULL
    ,distance INT NOT NULL
    ,fuel_consumed INT NOT NULL
    ,fuel_remaining INT NOT NULL
    ,time_remaining_in_seconds INT NOT NULL
    ,arrives_at TIMESTAMP WITH TIME ZONE NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

--

CREATE TABLE public.daemon_http_log (
     request jsonb NOT NULL
    ,response jsonb
    ,error VARCHAR(512)
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

--

CREATE TABLE public.daemon_system (
     system VARCHAR(100) NOT NULL PRIMARY KEY
    ,name VARCHAR(100) NOT NULL
);

--

CREATE TABLE public.daemon_location (
     system VARCHAR(100) NOT NULL
    ,location VARCHAR(100) NOT NULL
    ,location_name VARCHAR(100) NOT NULL
    ,location_type VARCHAR(100) NOT NULL
    ,x INT NOT NULL
    ,y INT NOT NULL
    ,created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
    ,PRIMARY KEY(system, location)
);

--

CREATE TABLE public.daemon_marketplace (
     location VARCHAR(100) NOT NULL
    ,good VARCHAR(100) NOT NULL
    ,purchase_price_per_unit INT NOT NULL
    ,sell_price_per_unit INT NOT NULL
    ,volume_per_unit INT NOT NULL
    ,quantity_available INT NOT NULL
    ,created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX idx_daemon_marketplace_location_good ON public.daemon_marketplace (location, good);

--

CREATE TABLE public.daemon_marketplace_latest (
     location VARCHAR(100) NOT NULL
    ,good VARCHAR(100) NOT NULL
    ,purchase_price_per_unit INT NOT NULL
    ,sell_price_per_unit INT NOT NULL
    ,volume_per_unit INT NOT NULL
    ,quantity_available INT NOT NULL
    ,created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX uq_daemon_marketplace_location_good ON public.daemon_marketplace_latest (location, good);

--

CREATE TABLE public.daemon_user (
     id uuid DEFAULT public.uuid_generate_v4()
    ,username VARCHAR(100) NOT NULL
    ,token VARCHAR(100) NOT NULL
    ,new_ship_role_data JSONB
    ,created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
);

ALTER TABLE ONLY public.daemon_user
    ADD CONSTRAINT uq_daemon_users_username_key UNIQUE (username);

--

CREATE TABLE public.daemon_user_ship (
     user_id uuid NOT NULL
    ,ship_id VARCHAR(50) NOT NULL
    ,type VARCHAR(50) NOT NULL
    ,class VARCHAR(50) NOT NULL
    ,max_cargo INT NOT NULL
    ,loading_speed INT NOT NULL
    ,speed INT NOT NULL
    ,manufacturer VARCHAR(50) NOT NULL
    ,plating INT NOT NULL
    ,weapons INT NOT NULL
    ,role_data JSONB
    ,location VARCHAR(50) NOT NULL
    ,modified_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX idx_daemon_user_ship_user_id ON public.daemon_user_ship USING btree (user_id);

CREATE UNIQUE INDEX idx_daemon_user_ship_user_id_ship_id ON public.daemon_user_ship USING btree (user_id, ship_id);

--

CREATE TABLE public.daemon_user_stats (
     user_id uuid NOT NULL
    ,credits INT NOT NULL
    ,created_at timestamp with time zone DEFAULT CURRENT_TIMESTAMP NOT NULL
    ,ship_count INT DEFAULT 0 NOT NULL
    ,ships jsonb
);

CREATE INDEX idx_daemon_user_stats_user_id ON public.daemon_user_stats USING btree (user_id);

CREATE INDEX idx_daemon_user_stats_user_id_created_at ON public.daemon_user_stats USING btree (user_id, created_at);

--

CREATE TABLE public.daemon_user_transaction (
     user_id uuid NOT NULL
    ,ship_id VARCHAR(50) NOT NULL
    ,type VARCHAR(50) NOT NULL
    ,good VARCHAR(50) NOT NULL
    ,price_per_unit INT NOT NULL
    ,quantity INT NOT NULL
    ,total INT NOT NULL
    ,location VARCHAR(100) NOT NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);
