CREATE TABLE todos (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT  NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    completed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Every list/get/delete query filters by user_id (ownership check), and
-- list additionally orders by created_at — this composite index serves
-- both without a separate sort step.
CREATE todos_user_id_created_at_idx ON todos (user_id, created_at DESC);

CREATE TRIGGER todos_set_updated_at
    BEFORE UPDATE ON todos
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at()
