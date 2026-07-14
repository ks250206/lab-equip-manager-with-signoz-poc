-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'admin')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_token_hash BYTEA NOT NULL,
    refresh_token_hash BYTEA NOT NULL,
    access_expires_at TIMESTAMPTZ NOT NULL,
    refresh_expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sessions_access_hash ON sessions (access_token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_refresh_hash ON sessions (refresh_token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions (user_id);

CREATE TABLE IF NOT EXISTS equipment (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    location TEXT NOT NULL DEFAULT '',
    image_object_key TEXT,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS reservations (
    id UUID PRIMARY KEY,
    equipment_id UUID NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    starts_at TIMESTAMPTZ NOT NULL,
    ends_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'cancelled')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (ends_at > starts_at)
);

CREATE INDEX IF NOT EXISTS idx_reservations_equipment_time
    ON reservations (equipment_id, starts_at, ends_at)
    WHERE status = 'active';
