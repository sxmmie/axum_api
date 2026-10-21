-- Add up migration script here
CREATE TABLE role (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE CHECK (name IN ('user', 'admin'))
);

INSERT INTO roles (name) VALUES ('user'), ('admin');

CREATE TABLE user_roles (
    user_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
	role_id    BIGINT      NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
	granted_by BIGINT      REFERENCES users(id) ON DELETE SET NULL,
	granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_user ON user_roles(user_id)

-- Audit rows outlive the grants they describe (SET NULL, not CASCADE) — this table is evidence.
CREATE TABLE role_audit (
    id BIGSERIAL PRIMARY KEY,
    actor_id   BIGINT      NOT NULL REFERENCES users(id),
	target_id  BIGINT      NOT NULL REFERENCES users(id),
	role       TEXT        NOT NULL,
	action     TEXT        NOT NULL CHECK (action IN ('grant', 'revoke')),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Backfill: everyone existing gets the base role, or the first deploy 403s every user.
INSERT INTO user_roles (user_id, role_id)
SELECT u.id, r.id FROM users u CROSS JOIN roles r WHERE r.name = 'user'
ON CONFLICT DO NOTHING;
