DROP TRIGGER users_audit ON rdr_users;
DROP TRIGGER posts_audit ON rdr_posts;
DROP TRIGGER follows_audit ON rdr_follows;
DROP SCHEMA audit CASCADE;
DROP TABLE rdr_users CASCADE;
DROP TABLE rdr_posts CASCADE;
DROP TABLE rdr_follows CASCADE;