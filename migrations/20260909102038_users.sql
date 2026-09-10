-- Add migration script here
CREATE TABLE users (
       id UUID PRIMARY KEY,
       github_id BIGINT NOT NULL UNIQUE,
       github_login TEXT NOT NULL,
       avatar_url TEXT,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);