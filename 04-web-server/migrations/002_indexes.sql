-- 002_indexes.sql
CREATE INDEX IF NOT EXISTS idx_users_email ON users (lower(email));
