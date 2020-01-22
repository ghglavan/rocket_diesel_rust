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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

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
        "SELECT id, author, title, date FROM rdr_posts WHERE author in (SELECT flw.followed FROM (SELECT followed, follower FROM rdr_follows WHERE follower = $1) AS flw)",
        &[&user.username],
    );

    match rows {
        Ok(n) => {
            for row in n {
                posts.push(json!({
                    "id": row.get::<usize, i32>(0),
                    "author": row.get::<usize, String>(1),
                    "title": row.get::<usize, String>(2),
                    "date": row.get::<usize, String>(3)
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

#[get("/posts", rank = 2)]
fn posts_not_logged() -> Redirect {
    Redirect::to(uri!(login_page))
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

            if (!found) {
                context.insert("error", "No access".to_string());
                return Template::render("post", &context);
            }

            context.insert("author", author);
            context.insert("title", n.get(0).get(2));
            context.insert("date", n.get(0).get(3));
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
        "INSERT INTO rdr_users (username, password) value ($1, $2)",
        &[&login.username, &password],
    );

    match rows_updated {
        Ok(0) => {
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
                post_not_logged
            ],
        )
}

fn main() {
    rocket().launch();
}
