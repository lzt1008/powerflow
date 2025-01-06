CREATE TABLE IF NOT EXISTS charging_histories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_level INTEGER NOT NULL,
    end_level INTEGER NOT NULL,
    charging_time INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    detail BLOB NOT NULL
);