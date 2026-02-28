-- Connections table: one row per TCP connection
CREATE TABLE IF NOT EXISTS connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    source_ip TEXT NOT NULL,
    source_port INTEGER NOT NULL,

    destination_ip TEXT NOT NULL,
    destination_port INTEGER NOT NULL,
    client_version TEXT,
    connection_success INTEGER NOT NULL
);

-- Auth attempts table: multiple rows per connection
CREATE TABLE IF NOT EXISTS auth_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    attempt_number INTEGER NOT NULL,
    auth_method TEXT NOT NULL,
    username TEXT NOT NULL,
    password TEXT,
    success INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (connection_id) REFERENCES connections(id)
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_connections_timestamp ON connections(timestamp);
CREATE INDEX IF NOT EXISTS idx_connections_source_ip ON connections(source_ip);
CREATE INDEX IF NOT EXISTS idx_auth_connection ON auth_attempts(connection_id);
CREATE INDEX IF NOT EXISTS idx_auth_username ON auth_attempts(username);
CREATE INDEX IF NOT EXISTS idx_auth_timestamp ON auth_attempts(timestamp);
