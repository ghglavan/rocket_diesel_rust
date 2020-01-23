CREATE schema audit;
REVOKE CREATE ON schema audit FROM public;
 
CREATE TABLE audit.logged_actions (
    schema_name text NOT NULL,
    TABLE_NAME text NOT NULL,
    user_name text,
    action_tstamp TIMESTAMP WITH TIME zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action TEXT NOT NULL CHECK (action IN ('I','D','U')),
    original_data text,
    new_data text,
    query text
) WITH (fillfactor=100);
 
REVOKE ALL ON audit.logged_actions FROM public;
 
GRANT SELECT ON audit.logged_actions TO public;
 
CREATE INDEX logged_actions_schema_table_idx 
ON audit.logged_actions(((schema_name||'.'||TABLE_NAME)::TEXT));
 
CREATE INDEX logged_actions_action_tstamp_idx 
ON audit.logged_actions(action_tstamp);
 
CREATE INDEX logged_actions_action_idx 
ON audit.logged_actions(action);
 

CREATE OR REPLACE FUNCTION audit.if_modified_func() RETURNS TRIGGER AS $body$
DECLARE
    v_old_data TEXT;
    v_new_data TEXT;
BEGIN
 
    IF (TG_OP = 'UPDATE') THEN
        v_old_data := ROW(OLD.*);
        v_new_data := ROW(NEW.*);
        INSERT INTO audit.logged_actions (schema_name,table_name,user_name,action,original_data,new_data,query) 
        VALUES (TG_TABLE_SCHEMA::TEXT,TG_TABLE_NAME::TEXT,session_user::TEXT,substring(TG_OP,1,1),v_old_data,v_new_data, current_query());
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        v_old_data := ROW(OLD.*);
        INSERT INTO audit.logged_actions (schema_name,table_name,user_name,action,original_data,query)
        VALUES (TG_TABLE_SCHEMA::TEXT,TG_TABLE_NAME::TEXT,session_user::TEXT,substring(TG_OP,1,1),v_old_data, current_query());
        RETURN OLD;
    ELSIF (TG_OP = 'INSERT') THEN
        v_new_data := ROW(NEW.*);
        INSERT INTO audit.logged_actions (schema_name,table_name,user_name,action,new_data,query)
        VALUES (TG_TABLE_SCHEMA::TEXT,TG_TABLE_NAME::TEXT,session_user::TEXT,substring(TG_OP,1,1),v_new_data, current_query());
        RETURN NEW;
    ELSE
        RAISE WARNING '[AUDIT.IF_MODIFIED_FUNC] - Other action occurred: %, at %',TG_OP,now();
        RETURN NULL;
    END IF;
 
EXCEPTION
    WHEN data_exception THEN
        RAISE WARNING '[AUDIT.IF_MODIFIED_FUNC] - UDF ERROR [DATA EXCEPTION] - SQLSTATE: %, SQLERRM: %',SQLSTATE,SQLERRM;
        RETURN NULL;
    WHEN unique_violation THEN
        RAISE WARNING '[AUDIT.IF_MODIFIED_FUNC] - UDF ERROR [UNIQUE] - SQLSTATE: %, SQLERRM: %',SQLSTATE,SQLERRM;
        RETURN NULL;
    WHEN OTHERS THEN
        RAISE WARNING '[AUDIT.IF_MODIFIED_FUNC] - UDF ERROR [OTHER] - SQLSTATE: %, SQLERRM: %',SQLSTATE,SQLERRM;
        RETURN NULL;
END;
$body$
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, audit;
 
CREATE TABLE rdr_users (
  username VARCHAR PRIMARY KEY  NOT NULL,
  password VARCHAR NOT NULL
);

INSERT INTO rdr_users (username, password) VALUES ('user1', '24c9e15e52afc47c225b757e7bee1f9d');
INSERT INTO rdr_users (username, password) VALUES ('user2', '7e58d63b60197ceb55a1c487989a3720');
INSERT INTO rdr_users (username, password) VALUES ('user3', '92877af70a45fd6a2ed7fe81e1236b78');
INSERT INTO rdr_users (username, password) VALUES ('user4', '3f02ebe3d7929b091e3d8ccfde2f3bc6');

CREATE TABLE rdr_posts (
    id SERIAL PRIMARY KEY,
    author VARCHAR REFERENCES rdr_users(username) NOT NULL,
    title VARCHAR NOT NULL,
    date BIGINT NOT NULL,
    body VARCHAR NOT NULL
);

INSERT INTO rdr_posts (author, title, date, body) VALUES ('user4', 'user4_post1', 1579773449, 'body of user4 post 1');
INSERT INTO rdr_posts (author, title, date, body) VALUES ('user4', 'user4_post2', 1579774449, 'body of user4 post 2');
INSERT INTO rdr_posts (author, title, date, body) VALUES ('user2', 'user2_post1', 1579775449, 'body of user2 post 1');
INSERT INTO rdr_posts (author, title, date, body) VALUES ('user2', 'user2_post2', 1579776449, 'body of user2 post 2');

CREATE TABLE rdr_follows (
    id SERIAL PRIMARY KEY,
    follower VARCHAR REFERENCES rdr_users(username) NOT NULL,
    followed VARCHAR REFERENCES rdr_users(username) NOT NULL
);

INSERT INTO rdr_follows (follower, followed) VALUES ('user1', 'user2');
INSERT INTO rdr_follows (follower, followed) VALUES ('user1', 'user3');
INSERT INTO rdr_follows (follower, followed) VALUES ('user1', 'user4');

CREATE TABLE rdr_comments (
    id SERIAL PRIMARY KEY,
    post_id INTEGER REFERENCES rdr_posts(id) NOT NULL,
    author VARCHAR REFERENCES rdr_users(username) NOT NULL,
    date BIGINT NOT NULL,
    body VARCHAR NOT NULL
);

INSERT INTO rdr_comments (post_id, author, date, body) VALUES (1, 'user1', 1579776449, 'me liky');
INSERT INTO rdr_comments (post_id, author, date, body) VALUES (1, 'user2', 1579777449, 'me liky too');
INSERT INTO rdr_comments (post_id, author, date, body) VALUES (1, 'user3', 1579778449, 'meh');

CREATE TABLE rdr_rating (
    id SERIAL PRIMARY KEY,
    post_id INTEGER REFERENCES rdr_posts(id) NOT NULL,
    author VARCHAR REFERENCES rdr_users(username) NOT NULL,
    upvote BOOLEAN NOT NULL,
    downvote BOOLEAN NOT NULL
);

INSERT INTO rdr_rating (post_id, author, upvote, downvote) VALUES (1, 'user1', 't', 'f');
INSERT INTO rdr_rating (post_id, author, upvote, downvote) VALUES (1, 'user2', 't', 'f');
INSERT INTO rdr_rating (post_id, author, upvote, downvote) VALUES (1, 'user3', 'f', 't');

CREATE TABLE rdr_post_tags (
    tag_name VARCHAR PRIMARY KEY 
);

INSERT INTO rdr_post_tags (tag_name) VALUES ('tag1');
INSERT INTO rdr_post_tags (tag_name) VALUES ('tag2');
INSERT INTO rdr_post_tags (tag_name) VALUES ('tag3');
INSERT INTO rdr_post_tags (tag_name) VALUES ('tag4');

CREATE TABLE rdr_tags_in_posts (
    id SERIAL PRIMARY KEY,
    post_id INTEGER REFERENCES rdr_posts(id) NOT NULL,
    tag_name VARCHAR REFERENCES rdr_post_tags(tag_name) NOT NULL
);

INSERT INTO rdr_tags_in_posts (post_id, tag_name) VALUES (1, 'tag1');
INSERT INTO rdr_tags_in_posts (post_id, tag_name) VALUES (1, 'tag2');
INSERT INTO rdr_tags_in_posts (post_id, tag_name) VALUES (1, 'tag3');
INSERT INTO rdr_tags_in_posts (post_id, tag_name) VALUES (1, 'tag4');


CREATE TRIGGER users_audit
AFTER INSERT OR UPDATE OR DELETE ON rdr_users
FOR EACH ROW EXECUTE PROCEDURE audit.if_modified_func();

CREATE TRIGGER posts_audit
AFTER INSERT OR UPDATE OR DELETE ON rdr_posts
FOR EACH ROW EXECUTE PROCEDURE audit.if_modified_func();

CREATE TRIGGER follows_audit
AFTER INSERT OR UPDATE OR DELETE ON rdr_follows
FOR EACH ROW EXECUTE PROCEDURE audit.if_modified_func();