table! {
    rdr_follows (id) {
        id -> Int4,
        follower -> Varchar,
        followed -> Varchar,
    }
}

table! {
    rdr_posts (id) {
        id -> Int4,
        author -> Varchar,
        title -> Varchar,
        date -> Varchar,
        body -> Varchar,
    }
}

table! {
    rdr_users (username) {
        username -> Varchar,
        password -> Varchar,
    }
}

joinable!(rdr_posts -> rdr_users (author));

allow_tables_to_appear_in_same_query!(
    rdr_follows,
    rdr_posts,
    rdr_users,
);
