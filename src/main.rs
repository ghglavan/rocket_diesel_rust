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

extern crate rand;

#[cfg(test)]
mod tests;

use rocket_contrib::databases::postgres;

use std::collections::{HashMap, HashSet};

use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{self, FlashMessage, Form, FromRequest, Request};
use rocket::response::{Flash, Redirect};
use rocket_contrib::json::Json;
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

#[post("/login", data = "<login>")]
fn login(
    conn: RdrDbConn,
    mut cookies: Cookies<'_>,
    login: Form<Login>,
    cd_users: State<ConnectedUsers>,
) -> Result<Redirect, Flash<Redirect>> {
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

                if (pw == login.password) {
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

#[post("/register", data = "<login>")]
fn register(
    conn: RdrDbConn,
    mut cookies: Cookies<'_>,
    login: Form<Login>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let rows_updated = &conn.execute(
        "INSERT INTO rdr_users (username, password) VALUES ($1, $2)",
        &[&login.username, &login.password],
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
                register_page
            ],
        )
}

fn main() {
    rocket().launch();
}
