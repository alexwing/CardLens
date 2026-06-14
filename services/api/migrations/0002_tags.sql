-- Etiquetas (tags) para los items de la coleccion.
-- Cada item puede tener varias tags; cada tag puede aplicarse a varios items
-- (relacion N:M via collection_item_tags). Los ids de tag son UUID v4 en texto.

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Unicidad case-insensitive del nombre: "Raras" y "raras" son la misma tag.
CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_name_nocase ON tags(name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS collection_item_tags (
    item_id TEXT NOT NULL REFERENCES collection_items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_collection_item_tags_tag_id ON collection_item_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_collection_item_tags_item_id ON collection_item_tags(item_id);
