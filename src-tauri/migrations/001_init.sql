CREATE TABLE permission_grants (
 id INTEGER PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('FILE','DIRECTORY')),
 path TEXT NOT NULL, path_key TEXT NOT NULL, identity TEXT NOT NULL,
 active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0,1)), created_at INTEGER NOT NULL,
 UNIQUE(kind,path_key,identity)
);
CREATE TABLE file_records (
 id INTEGER PRIMARY KEY, identity TEXT NOT NULL UNIQUE, path TEXT NOT NULL,
 path_key TEXT NOT NULL UNIQUE, name TEXT NOT NULL, extension TEXT NOT NULL,
 size INTEGER NOT NULL CHECK(size>=0), modified_at INTEGER NOT NULL,
 registered_at INTEGER NOT NULL, status TEXT NOT NULL CHECK(status IN ('READY','MISSING','UNAUTHORIZED','CHANGED','ERROR'))
);
CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL, name_key TEXT NOT NULL UNIQUE);
CREATE TABLE file_tags (
 file_id INTEGER NOT NULL REFERENCES file_records(id) ON DELETE CASCADE,
 tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
 source TEXT NOT NULL DEFAULT 'USER' CHECK(source IN ('USER','AI')),
 PRIMARY KEY(file_id,tag_id)
);
CREATE INDEX idx_files_extension ON file_records(extension);
CREATE INDEX idx_files_name ON file_records(name);
CREATE INDEX idx_file_tags_tag ON file_tags(tag_id,file_id);
CREATE INDEX idx_grants_active ON permission_grants(active);
PRAGMA user_version = 1;
