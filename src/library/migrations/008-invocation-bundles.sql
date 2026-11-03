CREATE TABLE invocation_bundles (
    id TEXT PRIMARY KEY NOT NULL CHECK(length(id)=64),
    record TEXT NOT NULL CHECK(json_valid(record))
);
CREATE TABLE bundle_assets (
    bundle_id TEXT NOT NULL REFERENCES invocation_bundles(id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES assets(id),
    PRIMARY KEY(bundle_id,asset_id)
);
CREATE TABLE bundle_attempts (
    bundle_id TEXT NOT NULL REFERENCES invocation_bundles(id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES attempts(id) ON DELETE CASCADE,
    PRIMARY KEY(bundle_id,attempt_id)
);
CREATE INDEX bundle_assets_asset ON bundle_assets(asset_id);
