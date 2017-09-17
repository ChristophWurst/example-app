use std::error::Error;

use mime;
use hyper::{StatusCode, Body};
use hyper::header::Location;
use hyper::server::{Request, Response};
use futures::{Future, Stream};
use serde_urlencoded;

use gotham::state::{State, FromState};
use gotham::middleware::session::SessionData;
use gotham::http::response::create_response;

use session::Session;

#[derive(Deserialize)]
struct FormData {
    item: String,
}

pub fn index(state: State, _req: Request) -> (State, Response) {
    let response = {
        let session = SessionData::<Session>::borrow_from(&state);

        // Gotham helper for creating responses and setting ia range of important headers
        // to meet specifications and enhance security.
        create_response(
            &state,
            StatusCode::Ok,
            Some((index_body(session.todo_list.clone()), mime::TEXT_HTML)),
        )

    };

    (state, response)
}

// TODO: This is full of CSRF holes. Don't be full of CSRF holes.
pub fn add(mut state: State, req: Request) -> (State, Response) {
    let response = {
        let session = SessionData::<Session>::borrow_mut_from(&mut state);

        let data_result = ugly_form_body_parser(req.body());

        match data_result {
            Ok(data) => session.todo_list.push(data.item),
            Err(e) => warn!("failed to parse form body: {}", e),
        }

        let mut response = Response::new().with_status(StatusCode::SeeOther);
        response.headers_mut().set(Location::new("/todo"));

        response
    };

    (state, response)
}

pub fn reset(mut state: State, _req: Request) -> (State, Response) {
    let session = SessionData::<Session>::take_from(&mut state);
    session.discard(&mut state).unwrap();

    let mut response = Response::new().with_status(StatusCode::SeeOther);
    response.headers_mut().set(Location::new("/todo"));

    (state, response)
}

// Someday Gotham will have compiled templates with blazing speeds and type safety.
//
// Today we have raw strings that we extend, impressive huh?
//
// You think you're better than me?
//   - Izzy Mandelbaum
fn index_body(items: Vec<String>) -> Vec<u8> {
    let mut out = String::new();

    let part = r#"
        <!doctype html>
        <html>
            <head>
                <meta charset="UTF-8">
                <title>Todo (Session-backed)</title>
            </head>
            <body>
                <h1>Todo list</h1>
    "#;
    out.extend(part.chars());

    // TODO: This allows HTML injection by the user, huge potential for XSS if copied.
    // Tidy this up sooner rather than later. Currently only thwarted by us not decoding the
    // URL-encoded body.
    if items.len() > 0 {
        out.extend("<ul>".chars());
        for item in items {
            let part = format!("<li>{}</li>", item);
            out.extend(part.chars());
        }
        out.extend("</ul>".chars());
    }

    let part = r#"
                <form method="post">
                    <input type="text" name="item"/>
                    <button type="submit">Add</button>
                </form>
                <script type="text/javascript">
                    document.forms[0].getElementsByTagName('input')[0].focus()
                </script>

                <form method="post" action="/todo/reset">
                    <button type="submit">Reset</button>
                </form>
                <br><br>
                <a href="/">Go Home</a>
            </body>
        </html>
        "#;
    out.extend(part.chars());
    out.into_bytes()
}

fn ugly_form_body_parser(body: Body) -> Result<FormData, Box<Error>> {
    let mut req_body = Vec::new();
    for part in try!(body.collect().wait()) {
        req_body.extend(part);
    }

    let data = try!(serde_urlencoded::from_bytes::<FormData>(&req_body));
    Ok(data)
}
