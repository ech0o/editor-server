-- Add migration script here
CREATE TABLE oauth_codes (
                             code_hash BYTEA PRIMARY KEY,
                             user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                             expires_at TIMESTAMPTZ NOT NULL,
                             used_at TIMESTAMPTZ
);

CREATE INDEX idx_oauth_codes_expires_at
    ON oauth_codes(expires_at);