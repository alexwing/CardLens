-- Esquema inicial de la base de datos compartida (SQLite, WAL).
-- Este archivo es el UNICO propietario del esquema: el resto de servicios
-- (ingesta Python, servicio ML) leen/escriben contra estas tablas.

CREATE TABLE IF NOT EXISTS sets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    series TEXT,
    code TEXT,
    release_date TEXT,
    total INTEGER,
    lang TEXT NOT NULL,
    symbol_url TEXT,
    logo_url TEXT
);

CREATE TABLE IF NOT EXISTS cards (
    id TEXT PRIMARY KEY,
    set_id TEXT NOT NULL REFERENCES sets(id),
    name TEXT NOT NULL,
    number TEXT NOT NULL,
    rarity TEXT,
    supertype TEXT,
    subtypes TEXT,
    lang TEXT NOT NULL,
    image_url TEXT,
    image_local TEXT,
    illustrator TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scans (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    image_path TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'done',
    best_card_id TEXT REFERENCES cards(id),
    confidence REAL,
    low_confidence INTEGER NOT NULL DEFAULT 0,
    raw_json TEXT
);

CREATE TABLE IF NOT EXISTS scan_candidates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scan_id TEXT NOT NULL REFERENCES scans(id),
    card_id TEXT NOT NULL REFERENCES cards(id),
    rank INTEGER NOT NULL,
    visual_score REAL NOT NULL,
    ocr_score REAL NOT NULL,
    final_score REAL NOT NULL
);

-- Preparada para autenticacion futura; el MVP no usa auth.
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE,
    display_name TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS collection_items (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id),
    card_id TEXT NOT NULL REFERENCES cards(id),
    scan_id TEXT REFERENCES scans(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    condition TEXT,
    lang TEXT,
    notes TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS prices (
    card_id TEXT NOT NULL REFERENCES cards(id),
    source TEXT NOT NULL,
    currency TEXT NOT NULL,
    market REAL,
    low REAL,
    high REAL,
    trend REAL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (card_id, source)
);

CREATE INDEX IF NOT EXISTS idx_cards_name ON cards(name);
CREATE INDEX IF NOT EXISTS idx_cards_set_id ON cards(set_id);
CREATE INDEX IF NOT EXISTS idx_scan_candidates_scan_id ON scan_candidates(scan_id);
CREATE INDEX IF NOT EXISTS idx_collection_items_card_id ON collection_items(card_id);
