table! {
    rdr_comments (id) {
        id -> Int4,
        post_id -> Int4,
        author -> Varchar,
        date -> Int8,
        body -> Varchar,
    }
}

table! {
    rdr_follows (id) {
        id -> Int4,
        follower -> Varchar,
        followed -> Varchar,
    }
}

table! {
    rdr_post_tags (tag_name) {
        tag_name -> Varchar,
    }
}

table! {
    rdr_posts (id) {
        id -> Int4,
        author -> Varchar,
        title -> Varchar,
        date -> Int8,
        body -> Varchar,
    }
}

table! {
    rdr_rating (id) {
        id -> Int4,
        post_id -> Int4,
        author -> Varchar,
        upvote -> Bool,
        downvote -> Bool,
    }
}

table! {
    rdr_tags_in_posts (id) {
        id -> Int4,
        post_id -> Int4,
        tag_name -> Varchar,
    }
}

table! {
    rdr_users (username) {
        username -> Varchar,
        password -> Varchar,
    }
}

joinable!(rdr_comments -> rdr_posts (post_id));
joinable!(rdr_comments -> rdr_users (author));
joinable!(rdr_posts -> rdr_users (author));
joinable!(rdr_rating -> rdr_posts (post_id));
joinable!(rdr_rating -> rdr_users (author));
joinable!(rdr_tags_in_posts -> rdr_post_tags (tag_name));
joinable!(rdr_tags_in_posts -> rdr_posts (post_id));

allow_tables_to_appear_in_same_query!(
    rdr_comments,
    rdr_follows,
    rdr_post_tags,
    rdr_posts,
    rdr_rating,
    rdr_tags_in_posts,
    rdr_users,
);
