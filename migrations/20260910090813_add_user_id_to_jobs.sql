-- Add migration script here
ALTER TABLE jobs
    ADD COLUMN user_id UUID REFERENCES users(id);

-- ALTER TABLE jobs
--     ADD CONSTRAINT jobs_owner_check
--         CHECK (
--             (user_id IS NOT NULL AND api_key_id IS NULL)
--                 OR
--             (user_id IS NULL AND api_key_id IS NOT NULL)
--             );