CREATE TABLE assets (
    id TEXT PRIMARY KEY NOT NULL,
    sha256 TEXT NOT NULL UNIQUE CHECK(length(sha256) = 64),
    size INTEGER NOT NULL CHECK(size > 0),
    width INTEGER NOT NULL CHECK(width > 0),
    height INTEGER NOT NULL CHECK(height > 0),
    media_type TEXT NOT NULL CHECK(media_type IN ('image/png', 'image/jpeg'))
);
