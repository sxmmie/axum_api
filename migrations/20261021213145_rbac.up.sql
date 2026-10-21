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
