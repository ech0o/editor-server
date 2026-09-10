-- Add migration script here
ALTER TABLE sessions
    ADD COLUMN token_hash BYTEA NOT NULL UNIQUE;