-- Add migration script here
ALTER TABLE jobs
ADD COLUMN api_key_id UUID REFERENCES api_keys(id);

CREATE INDEX idx_jobs_api_key_id
    ON jobs(api_key_id);