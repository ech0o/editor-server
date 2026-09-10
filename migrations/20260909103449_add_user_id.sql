ALTER TABLE api_keys
    ADD COLUMN user_id UUID REFERENCES users(id);-- Add migration script here
