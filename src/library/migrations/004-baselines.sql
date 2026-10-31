CREATE TABLE baselines (
    tool TEXT NOT NULL,
    action TEXT NOT NULL,
    attempt_id TEXT NOT NULL REFERENCES attempts(id) ON DELETE CASCADE,
    PRIMARY KEY(tool, action)
);
