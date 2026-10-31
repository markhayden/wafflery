CREATE TABLE examples (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    record TEXT NOT NULL
);
CREATE TABLE example_assets (
    example_id TEXT NOT NULL REFERENCES examples(id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES assets(id),
    PRIMARY KEY(example_id, asset_id)
);
