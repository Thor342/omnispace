use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn init_db(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS spaces (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            icon        TEXT NOT NULL DEFAULT '📁',
            color       TEXT NOT NULL DEFAULT '#6366f1',
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notes (
            id          TEXT PRIMARY KEY,
            space_id    TEXT NOT NULL,
            title       TEXT NOT NULL,
            content     TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY (space_id) REFERENCES spaces(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS files (
            id              TEXT PRIMARY KEY,
            space_id        TEXT NOT NULL,
            name            TEXT NOT NULL,
            original_path   TEXT NOT NULL,
            stored_path     TEXT NOT NULL,
            file_type       TEXT NOT NULL DEFAULT 'other',
            size            INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT NOT NULL,
            FOREIGN KEY (space_id) REFERENCES spaces(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS links (
            id          TEXT PRIMARY KEY,
            space_id    TEXT NOT NULL,
            title       TEXT NOT NULL DEFAULT '',
            url         TEXT NOT NULL,
            link_type   TEXT NOT NULL DEFAULT 'general',
            created_at  TEXT NOT NULL,
            FOREIGN KEY (space_id) REFERENCES spaces(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id          TEXT PRIMARY KEY,
            space_id    TEXT NOT NULL,
            title       TEXT NOT NULL,
            completed   INTEGER NOT NULL DEFAULT 0,
            order_index INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY (space_id) REFERENCES spaces(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pages (
            id          TEXT PRIMARY KEY,
            space_id    TEXT NOT NULL,
            title       TEXT NOT NULL DEFAULT 'Nueva página',
            order_index INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY (space_id) REFERENCES spaces(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS blocks (
            id          TEXT PRIMARY KEY,
            page_id     TEXT NOT NULL,
            block_type  TEXT NOT NULL,
            x           REAL NOT NULL DEFAULT 100,
            y           REAL NOT NULL DEFAULT 100,
            width       REAL NOT NULL DEFAULT 420,
            height      REAL NOT NULL DEFAULT 300,
            content     TEXT NOT NULL DEFAULT '{}',
            z_index     INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS page_strokes (
            page_id     TEXT PRIMARY KEY,
            data        TEXT NOT NULL DEFAULT '[]',
            updated_at  TEXT NOT NULL,
            FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS categories (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            color       TEXT NOT NULL DEFAULT '#6366f1',
            order_index INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
    ")?;

    // Migrations
    let _ = conn.execute("ALTER TABLE spaces ADD COLUMN category_id TEXT", []);
    let _ = conn.execute("ALTER TABLE categories ADD COLUMN icon TEXT NOT NULL DEFAULT '📂'", []);

    Ok(conn)
}
