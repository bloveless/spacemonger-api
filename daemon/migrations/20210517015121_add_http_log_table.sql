-- Add migration script here
CREATE TABLE daemon_http_log (
     request jsonb NOT NULL
    ,response jsonb NULL
    ,error varchar(512) NULL
    ,created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);
