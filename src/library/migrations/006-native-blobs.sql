CREATE TABLE native_blobs (
    sha256 TEXT PRIMARY KEY NOT NULL CHECK(length(sha256)=64),
    size INTEGER NOT NULL CHECK(size >= 0 AND size <= 134217728)
);
CREATE TABLE native_links (
    owner_kind TEXT NOT NULL CHECK(owner_kind IN ('attempt','example','bundle')),
    owner_id TEXT NOT NULL,
    direction TEXT NOT NULL CHECK(direction IN ('in','out','bundle')),
    occurrence INTEGER NOT NULL CHECK(occurrence >= 0),
    sha256 TEXT NOT NULL REFERENCES native_blobs(sha256),
    media_type TEXT NOT NULL,
    artifact TEXT NOT NULL CHECK(json_valid(artifact)),
    CHECK((owner_kind='bundle' AND direction='bundle') OR (owner_kind IN ('attempt','example') AND direction IN ('in','out'))),
    PRIMARY KEY(owner_kind,owner_id,direction,occurrence)
);
CREATE INDEX native_links_digest ON native_links(sha256);
CREATE TRIGGER native_attempt_owner BEFORE INSERT ON native_links
WHEN NEW.owner_kind='attempt' AND NOT EXISTS(SELECT 1 FROM attempts WHERE id=NEW.owner_id)
BEGIN SELECT RAISE(ABORT,'native blob attempt owner is missing'); END;
CREATE TRIGGER native_example_owner BEFORE INSERT ON native_links
WHEN NEW.owner_kind='example' AND NOT EXISTS(SELECT 1 FROM examples WHERE id=NEW.owner_id)
BEGIN SELECT RAISE(ABORT,'native blob example owner is missing'); END;
CREATE TRIGGER native_attempt_deleted AFTER DELETE ON attempts
BEGIN DELETE FROM native_links WHERE owner_kind='attempt' AND owner_id=OLD.id; END;
CREATE TRIGGER native_example_deleted AFTER DELETE ON examples
BEGIN DELETE FROM native_links WHERE owner_kind='example' AND owner_id=OLD.id; END;
