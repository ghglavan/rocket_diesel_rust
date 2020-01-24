#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate rocket_contrib;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate md5;
extern crate rand;

#[cfg(test)]
mod tests;

use rocket_contrib::databases::postgres;

use std::collections::{HashMap, HashSet};

use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{self, FlashMessage, Form, FromRequest, Request};
use rocket::response::{Flash, Redirect};
use rocket_contrib::json::{Json, JsonValue};
use rocket_contrib::templates::Template;

use serde::{Deserialize, Serialize};
//use serde_json::Result;

use rocket::State;

use rocket::response::NamedFile;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

extern crate chrono;
use chrono::prelude::DateTime;
use chrono::Utc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(FromForm, Clone, Debug)]
struct Post {
    title: String,
    body: String,
}

#[derive(FromForm, Clone, Debug)]
struct Comment {
    body: String,
}

#[derive(FromForm, Clone, Debug)]
struct Login {
    username: String,
    password: String,
}

#[derive(Debug)]
struct ConnectedUsers {
    connected_users: Arc<Mutex<HashSet<String>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    username: String,
    random: usize,
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = std::convert::Infallible;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<User, Self::Error> {
        request
            .cookies()
            .get_private("user_id")
            .and_then(|cookie| serde_json::from_str(cookie.value()).ok())
            .or_forward(())
    }
}

#[post("/add_post", data = "<post_form>")]
fn add_post(conn: RdrDbConn, post_form: Form<Post>, user: User) -> Redirect {
    let date = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let rows = &conn.execute(
        "INSERT INTO rdr_posts (author, title, date, body) VALUES ($1, $2, $3, $4)",
        &[&user.username, &post_form.title, &date, &post_form.body],
    );

    Redirect::to(uri!(index))
}

#[post("/login", data = "<login_form>")]
fn login(
    conn: RdrDbConn,
    mut cookies: Cookies<'_>,
    login_form: Form<Login>,
    cd_users: State<ConnectedUsers>,
) -> Result<Redirect, Flash<Redirect>> {
    let login = login_form.into_inner();
    let password = format!("{:?}", md5::compute(login.password.clone()));
    drop(login.password);

    let rows = &conn.query(
        "SELECT * FROM rdr_users WHERE username = $1",
        &[&login.username],
    );

    match rows {
        Ok(r) => {
            if (r.len() > 1) {
                return Err(Flash::error(
                    Redirect::to(uri!(login_page)),
                    format!(
                        "Internal error: Unexpected number users with name {}: {}.",
                        login.username,
                        r.len()
                    ),
                ));
            } else if (r.len() == 0) {
                return Err(Flash::error(
                    Redirect::to(uri!(login_page)),
                    "Invalid username/password.",
                ));
            } else {
                let first_row = r.get(0);
                let pw: String = first_row.get("password");

                if (pw == password) {
                    let users = &mut *(cd_users.connected_users).lock().unwrap();
                    users.insert(login.username.clone());

                    let user = User {
                        username: login.username.clone(),
                        random: rand::random::<usize>(),
                    };
                    cookies.add_private(Cookie::new(
                        "user_id",
                        serde_json::to_string(&user).unwrap(),
                    ));
                    return Ok(Redirect::to(uri!(user_index)));
                } else {
                    return Err(Flash::error(
                        Redirect::to(uri!(login_page)),
                        "Invalid username/password.",
                    ));
                }
            }
        }
        Err(e) => {
            return Err(Flash::error(
                Redirect::to(uri!(login_page)),
                format!("Internal error: {}.", e),
            ));
        }
    };
}

#[get("/login")]
fn login_user(_user: User) -> Redirect {
    Redirect::to(uri!(index))
}

#[get("/login", rank = 2)]
fn login_page(flash: Option<FlashMessage<'_, '_>>) -> Template {
    let mut context = HashMap::new();
    if let Some(ref msg) = flash {
        context.insert("flash", msg.msg());
        if msg.name() == "error" {
            context.insert("flash_type", "Error");
        }
    }

    Template::render("login", &context)
}

#[get("/user/<username>")]
fn get_user(username: String, conn: RdrDbConn) -> JsonValue {
    let rows_r = &conn.query("SELECT * FROM rdr_users WHERE username = $1", &[&username]);
    match rows_r {
        Ok(rows) => {
            if rows.len() == 0 {
                return json!({
                    "status": "error",
                    "value": "Not found"
                });
            }
            if rows.len() > 1 {
                return json!({
                    "status": "error",
                    "value": format!("Internal error: too many rows returned for user {}: {}", username, rows.len())
                });
            }
            return json!({
                "status": "ok",
                "value": username.clone()
            });
        }
        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("{}", e)
            });
        }
    }
}

fn get_followed_users(conn: &RdrDbConn, username: &String) -> Result<Vec<String>, String> {
    let mut users: Vec<String> = Vec::new();

    let rows = &conn.query(
        "SELECT follower, followed FROM rdr_follows WHERE follower = $1",
        &[username],
    );

    match rows {
        Ok(n) => {
            for row in n {
                users.push(row.get(1));
            }
        }

        Err(e) => {
            return Err(format!("{}", e));
        }
    }

    Ok(users)
}

#[get("/followed_users")]
fn followed_users(conn: RdrDbConn, user: User) -> JsonValue {
    let users = get_followed_users(&conn, &user.username);
    match users {
        Ok(u) => {
            return json!({
                "status": "ok",
                "value": u
            });
        }

        Err(e) => {
            return json!({
                "status": "error",
                "value": e
            });
        }
    }
}

#[get("/followed_users", rank = 2)]
fn followed_users_not_logged() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/follow/<username>")]
fn follow_user(conn: RdrDbConn, user: User, username: String) -> JsonValue {
    let rows = &conn.execute(
        "INSERT INTO rdr_follows (follower, followed) VALUES ($1, $2)",
        &[&user.username, &username],
    );

    match rows {
        Ok(1) => {
            return json!({"status": "ok"});
        }
        _ => {
            return json!({"status": "error"});
        }
    };
}

#[get("/follow/<username>", rank = 2)]
fn follow_user_not_logged(username: String) -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/unfollow/<username>")]
fn unfollow_user(conn: RdrDbConn, user: User, username: String) -> JsonValue {
    let rows = &conn.execute(
        "DELETE FROM rdr_follows where follower = $1 AND followed = $2",
        &[&user.username, &username],
    );

    match rows {
        Ok(1) => {
            return json!({"status": "ok"});
        }
        _ => {
            return json!({"status": "error"});
        }
    };
}

#[get("/unfollow/<username>", rank = 2)]
fn unfollow_user_not_logged(username: String) -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/posts")]
fn posts(conn: RdrDbConn, user: User) -> JsonValue {
    let mut posts: Vec<JsonValue> = Vec::new();

    let rows = &conn.query(
        "SELECT id, author, title, date FROM rdr_posts WHERE author = $2 or author in (SELECT flw.followed FROM (SELECT followed, follower FROM rdr_follows WHERE follower = $1) AS flw) ORDER BY date DESC",
        &[&user.username, &user.username],
    );

    match rows {
        Ok(n) => {
            for row in n {
                let id = row.get::<usize, i32>(0);

                let mut tags: Vec<String> = Vec::new();

                let tags_rows =
                    &conn.query("SELECT * FROM rdr_tags_in_posts WHERE post_id = $1", &[&id]);

                if let Ok(tags_r) = tags_rows {
                    for tag_row in tags_r {
                        tags.push(tag_row.get(2));
                    }
                }

                posts.push(json!({
                    "id": id,
                    "author": row.get::<usize, String>(1),
                    "title": row.get::<usize, String>(2),
                    "date": row.get::<usize, i64>(3),
                    "tags": tags
                }));
            }

            return json!({
                "status": "ok",
                "value": posts
            });
        }

        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("{}", e)
            });
        }
    }
}

#[get("/posts_with_tag/<tag>")]
fn posts_with_tag(conn: RdrDbConn, tag: String, user: User) -> JsonValue {
    let mut posts: Vec<JsonValue> = Vec::new();

    let rows = &conn.query(
        "SELECT id, author, title, date FROM rdr_posts WHERE author = $2 or author in (SELECT flw.followed FROM (SELECT followed, follower FROM rdr_follows WHERE follower = $1) AS flw) ORDER BY date DESC",
        &[&user.username, &user.username],
    );

    match rows {
        Ok(n) => {
            for row in n {
                let id = row.get::<usize, i32>(0);

                let mut tags: Vec<String> = Vec::new();

                let tags_rows =
                    &conn.query("SELECT * FROM rdr_tags_in_posts WHERE post_id = $1", &[&id]);

                let mut found = false;
                if let Ok(tags_r) = tags_rows {
                    for tag_row in tags_r {
                        let t: String = tag_row.get(2);
                        if t == tag {
                            found = true;
                        }
                        tags.push(t);
                    }
                }

                if !found {
                    continue;
                }

                posts.push(json!({
                    "id": id,
                    "author": row.get::<usize, String>(1),
                    "title": row.get::<usize, String>(2),
                    "date": row.get::<usize, i64>(3),
                    "tags": tags
                }));
            }

            return json!({
                "status": "ok",
                "value": posts
            });
        }

        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("{}", e)
            });
        }
    }
}

#[get("/posts_with_group/<group>")]
fn posts_with_group(conn: RdrDbConn, group: String, user: User) -> JsonValue {
    let mut posts: Vec<JsonValue> = Vec::new();

    let rows = &conn.query(
        "SELECT id, author, title, date FROM rdr_posts WHERE author = $2 
        or author in (SELECT flw.followed FROM (SELECT followed, follower FROM rdr_follows WHERE follower = $1
        and followed in (SELECT grp.username from (SELECT * from rdr_users_in_groups WHERE groupname = $3) as grp)
        ) AS flw) 
        ORDER BY date DESC",
        &[&user.username, &user.username, &group],
    );

    match rows {
        Ok(n) => {
            for row in n {
                let id = row.get::<usize, i32>(0);

                let mut tags: Vec<String> = Vec::new();

                let tags_rows =
                    &conn.query("SELECT * FROM rdr_tags_in_posts WHERE post_id = $1", &[&id]);

                if let Ok(tags_r) = tags_rows {
                    for tag_row in tags_r {
                        let t: String = tag_row.get(2);
                        tags.push(t);
                    }
                }

                posts.push(json!({
                    "id": id,
                    "author": row.get::<usize, String>(1),
                    "title": row.get::<usize, String>(2),
                    "date": row.get::<usize, i64>(3),
                    "tags": tags
                }));
            }

            return json!({
                "status": "ok",
                "value": posts
            });
        }

        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("{}", e)
            });
        }
    }
}

#[get("/script.js")]
fn get_script() -> Result<NamedFile, std::io::Error> {
    NamedFile::open(std::path::Path::new("templates/script.js"))
}

#[get("/style.css")]
fn get_css() -> Result<NamedFile, std::io::Error> {
    NamedFile::open(std::path::Path::new("templates/style.css"))
}

#[get("/posts", rank = 2)]
fn posts_not_logged() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[get("/upvote/<post_id>")]
fn upvote(conn: RdrDbConn, user: User, post_id: i32) -> JsonValue {
    &conn.execute(
        "DELETE FROM rdr_rating WHERE author = $1 AND post_id = $2",
        &[&user.username, &post_id],
    );

    &conn.execute(
        "INSERT INTO rdr_rating (post_id, author, upvote, downvote) VALUES ($1, $2, $3, $4)",
        &[&post_id, &user.username, &true, &false],
    );

    json!({"status": "ok", "value": ""})
}

#[get("/downvote/<post_id>")]
fn downvote(conn: RdrDbConn, user: User, post_id: i32) -> JsonValue {
    &conn.execute(
        "DELETE FROM rdr_rating WHERE author = $1 AND post_id = $2",
        &[&user.username, &post_id],
    );

    &conn.execute(
        "INSERT INTO rdr_rating (post_id, author, upvote, downvote) VALUES ($1, $2, $3, $4)",
        &[&post_id, &user.username, &false, &true],
    );

    json!({"status": "ok", "value": ""})
}

#[get("/rating/<post_id>")]
fn rating(conn: RdrDbConn, user: User, post_id: i32) -> JsonValue {
    let mut n_upvotes = 0_u32;
    let mut n_downvotes = 0_u32;
    let mut user_upvoted = false;
    let mut user_downvoted = false;

    let rows = &conn.query(
        "SELECT * FROM rdr_rating WHERE post_id = $1 and upvote = 't'",
        &[&post_id],
    );

    match rows {
        Ok(n) => {
            n_upvotes = n.len() as u32;
        }
        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("could not get the nr of upvotes: {}", e)
            });
        }
    };

    let rows = &conn.query(
        "SELECT * FROM rdr_rating WHERE post_id = $1 and downvote = 't'",
        &[&post_id],
    );

    match rows {
        Ok(n) => {
            n_downvotes = n.len() as u32;
        }
        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("could not get the nr of downvotes: {}", e)
            });
        }
    };

    let rows = &conn.query(
        "SELECT * FROM rdr_rating WHERE author = $1 and post_id = $2",
        &[&user.username, &post_id],
    );
    match rows {
        Ok(n) => {
            if n.len() > 1 {
                return json!({
                    "status": "error",
                    "value": format!("internal error: {} has too many ratings {}", user.username, n.len())
                });
            }

            if n.len() == 1 {
                user_upvoted = n.get(0).get(3);
                user_downvoted = n.get(0).get(4);
            }

            if user_upvoted && user_downvoted {
                return json!({
                    "status": "error",
                    "value": format!("internal error: {} has both rating types", user.username)
                });
            }
        }
        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("could not get the nr of downvotes: {}", e)
            });
        }
    };

    json!({
        "status": "ok",
        "value": json!({
        "n_upvotes": n_upvotes,
        "n_downvotes": n_downvotes,
        "user_upvoted": user_upvoted,
        "user_downvoted": user_downvoted
    })
    })
}

#[post("/add_comment/<post_id>", data = "<comment_form>")]
fn add_comment(
    conn: RdrDbConn,
    user: User,
    post_id: i32,
    comment_form: Form<Comment>,
) -> JsonValue {
    let author = user.username.clone();
    let date = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let rows = &conn.query(
        "INSERT INTO rdr_comments (post_id, author, date, body) VALUES ($1, $2, $3, $4)",
        &[&post_id, &author, &date, &comment_form.body],
    );

    match rows {
        Ok(_) => {
            return json!({
                "status": "ok",
                "value": json!({
                    "author" :author,
                    "date": date
                })
            });
        }
        Err(r) => {
            return json!({
                "status": "error",
                "value": format!("internal error: {}", r)
            });
        }
    };
}

#[get("/groups")]
fn get_groups(conn: RdrDbConn, user: User) -> JsonValue {
    let rows = &conn.query(
        "SELECT * FROM rdr_users_in_groups WHERE username = $1",
        &[&user.username],
    );

    let mut groups: Vec<String> = Vec::new();

    match rows {
        Ok(n) => {
            for row in n {
                groups.push(row.get(2));
            }

            return json!({
                "status": "ok",
                "value": groups
            });
        }
        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("Internal error: {}", e)
            });
        }
    }
}

#[get("/comments/<post_id>")]
fn comments(conn: RdrDbConn, user: User, post_id: i32) -> JsonValue {
    let mut comms: Vec<JsonValue> = Vec::new();
    let rows = &conn.query("SELECT * FROM rdr_comments WHERE post_id = $1", &[&post_id]);

    match rows {
        Ok(n) => {
            for row in n {
                comms.push(json!({
                    "author": row.get::<usize, String>(2),
                    "date": row.get::<usize, i64>(3),
                    "comment": row.get::<usize, String>(4)
                }));
            }

            return json!({
                "status": "ok",
                "value": comms
            });
        }

        Err(e) => {
            return json!({
                "status": "error",
                "value": format!("{}", e)
            });
        }
    }
}

#[get("/post/<post_id>")]
fn post(conn: RdrDbConn, user: User, post_id: i32) -> Template {
    let rows = &conn.query("SELECT * FROM rdr_posts WHERE id = $1", &[&post_id]);

    let mut context = HashMap::new();
    match rows {
        Ok(n) => {
            if n.len() > 1 {
                context.insert(
                    "error",
                    format!("Internal error: Too many posts with this id: {}", n.len()),
                );
                return Template::render("post", &context);
            }

            if n.len() == 0 {
                context.insert("error", "No post found".to_string());
                return Template::render("post", &context);
            }

            let id: i32 = n.get(0).get(0);
            let author: String = n.get(0).get(1);

            let mut found = false;
            let users = get_followed_users(&conn, &user.username);
            if let Ok(u) = users {
                for user in u {
                    if user == author {
                        found = true;
                        break;
                    }
                }
            }

            if (!found && author != user.username) {
                context.insert("error", "No access".to_string());
                return Template::render("post", &context);
            }

            context.insert("id", id.to_string());
            context.insert("author", author);
            context.insert("title", n.get(0).get(2));
            let date = n.get(0).get::<usize, i64>(3) as u64;
            let d = UNIX_EPOCH + Duration::from_secs(date);
            let datetime = DateTime::<Utc>::from(d);
            let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

            context.insert("date", timestamp_str);
            context.insert("body", n.get(0).get(4));
            return Template::render("post", &context);
        }

        Err(e) => {
            context.insert("error", format!("Internal error: {}", e));
            return Template::render("post", &context);
        }
    }
}

#[get("/post/<post_id>", rank = 2)]
fn post_not_logged(post_id: i32) -> Template {
    let mut context = HashMap::new();
    context.insert("error", "You are not logged in".to_string());
    return Template::render("post", &context);
}
#[post("/register", data = "<login_form>")]
fn register(
    conn: RdrDbConn,
    mut cookies: Cookies<'_>,
    login_form: Form<Login>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let login = login_form.into_inner();
    let password = format!("{:?}", md5::compute(login.password.clone()));
    drop(login.password);

    let rows_updated = &conn.execute(
        "INSERT INTO rdr_users (username, password) values ($1, $2)",
        &[&login.username, &password],
    );

    match rows_updated {
        Ok(1) => {
            return Ok(Flash::success(
                Redirect::to(uri!(register_page)),
                "Registration was successfull",
            ));
        }
        Ok(c) => {
            return Err(Flash::error(
                Redirect::to(uri!(register_page)),
                format!("Internal error: unexpencted number of rows returned: {}", c),
            ));
        }
        Err(e) => {
            return Err(Flash::error(
                Redirect::to(uri!(register_page)),
                format!("Internal error: {}", e),
            ));
        }
    }
}

#[get("/register")]
fn register_page(flash: Option<FlashMessage<'_, '_>>) -> Template {
    let mut context = HashMap::new();
    if let Some(ref msg) = flash {
        context.insert("flash", msg.msg());
        if msg.name() == "error" {
            context.insert("flash_type", "Error");
        }
    }

    Template::render("register", &context)
}

#[post("/logout")]
fn logout(
    user: User,
    mut cookies: Cookies<'_>,
    cd_users: State<ConnectedUsers>,
) -> Flash<Redirect> {
    cookies.remove_private(Cookie::named("user_id"));
    let users = &mut *(cd_users.connected_users).lock().unwrap();
    users.remove(&user.username);
    Flash::success(Redirect::to(uri!(login_page)), "Successfully logged out.")
}

#[get("/")]
fn user_index(user: User) -> Template {
    let mut context = HashMap::new();
    context.insert("user_id", user.username);
    Template::render("index", &context)
}

#[get("/", rank = 2)]
fn index() -> Redirect {
    Redirect::to(uri!(login_page))
}

#[database("rdr")]
struct RdrDbConn(postgres::Connection);

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .attach(Template::fairing())
        .attach(RdrDbConn::fairing())
        .manage(ConnectedUsers {
            connected_users: Arc::new(Mutex::new(HashSet::new())),
        })
        .mount(
            "/",
            routes![
                index,
                user_index,
                login,
                logout,
                login_user,
                login_page,
                register,
                register_page,
                followed_users,
                followed_users_not_logged,
                get_user,
                posts,
                posts_not_logged,
                unfollow_user,
                unfollow_user_not_logged,
                follow_user,
                follow_user_not_logged,
                post,
                post_not_logged,
                add_post,
                comments,
                rating,
                upvote,
                downvote,
                add_comment,
                get_groups,
                posts_with_tag,
                posts_with_group,
                get_script,
                get_css
            ],
        )
}

fn main() {
    rocket().launch();
}
