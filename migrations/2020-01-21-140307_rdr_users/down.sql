DROP TRIGGER users_audit ON rdr_users;
DROP TRIGGER posts_audit ON rdr_posts;
DROP TRIGGER follows_audit ON rdr_follows;
DROP SCHEMA audit CASCADE;
DROP TABLE rdr_users CASCADE;
DROP TABLE rdr_posts CASCADE;
DROP TABLE rdr_follows CASCADE;
DROP TABLE rdr_comments CASCADE;
DROP TABLE rdr_rating CASCADE;
DROP TABLE rdr_post_tags CASCADE;
DROP TABLE rdr_tags_in_posts CASCADE;
DROP TABLE rdr_groups CASCADE;
DROP TABLE rdr_users_in_groups CASCADE;

DROP USER rdr_user;