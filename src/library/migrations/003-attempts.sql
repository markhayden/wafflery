CREATE TABLE attempts (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    submission TEXT NOT NULL,
    record TEXT NOT NULL
);
CREATE TABLE attempt_assets (
    attempt_id TEXT NOT NULL REFERENCES attempts(id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES assets(id),
    PRIMARY KEY(attempt_id, asset_id)
);
