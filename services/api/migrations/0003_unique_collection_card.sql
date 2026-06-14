-- Evita cartas duplicadas en la coleccion (control interno de duplicados).
-- Primero elimina los duplicados que pudieran existir, conservando el primero
-- insertado (menor rowid) por (usuario, carta). user_id NULL = usuario unico
-- actual (sin auth); COALESCE lo trata como un solo usuario.
DELETE FROM collection_items
WHERE rowid NOT IN (
    SELECT MIN(rowid)
    FROM collection_items
    GROUP BY COALESCE(user_id, ''), card_id
);

-- Una carta por usuario. Cuando exista autenticacion (user_id no nulo) el
-- indice sigue siendo correcto por usuario.
CREATE UNIQUE INDEX IF NOT EXISTS ux_collection_user_card
    ON collection_items (COALESCE(user_id, ''), card_id);
