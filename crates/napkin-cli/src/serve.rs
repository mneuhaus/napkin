//! `napkin serve`: Numi's Alfred endpoint, `GET http://127.0.0.1:15055/?q=<expression>` answered
//! with the formatted result as plain text, so Numi's Alfred workflow works unchanged.

use std::borrow::Cow;
use std::io::Write;
use std::net::Ipv4Addr;
use std::str::Utf8Error;

use anyhow::{Context as _, anyhow};
use napkin_core::Engine;
use percent_encoding::percent_decode_str;
use tiny_http::{Header, Method, Response, Server};

/// Numi's port for the Alfred integration.
pub(crate) const DEFAULT_PORT: u16 = 15055;

/// An HTTP answer: status code and plain-text body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) body: String,
}

impl Reply {
    fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }
}

/// Serves requests one after the other until the process is stopped. Binds to 127.0.0.1 only;
/// the actual address (useful with `--port 0`) is announced on `log`.
pub(crate) fn run(engine: &Engine, port: u16, log: &mut impl Write) -> anyhow::Result<()> {
    let server = Server::http((Ipv4Addr::LOCALHOST, port))
        .map_err(|error| anyhow!("could not listen on 127.0.0.1:{port}: {error}"))?;
    let address = server
        .server_addr()
        .to_ip()
        .context("the server is not listening on an IP address")?;
    writeln!(log, "napkin: serving http://{address}/?q=<expression>")?;
    log.flush()?;
    let content_type = Header::from_bytes("Content-Type", "text/plain; charset=utf-8")
        .map_err(|()| anyhow!("invalid content-type header"))?;
    for request in server.incoming_requests() {
        let reply = answer(engine, request.method(), request.url());
        let response = Response::from_string(reply.body)
            .with_status_code(reply.status)
            .with_header(content_type.clone());
        if let Err(error) = request.respond(response) {
            writeln!(log, "napkin: could not answer a request: {error}")?;
        }
    }
    Ok(())
}

/// The reply to one request: `200` with the result, `422` with the evaluation error, `400` without
/// a `q` parameter, `405` for anything but `GET`/`HEAD`. Any path is accepted (`/?q=` and `?q=`).
pub(crate) fn answer(engine: &Engine, method: &Method, url: &str) -> Reply {
    if !matches!(method, Method::Get | Method::Head) {
        return Reply::new(405, "only GET is supported");
    }
    let expression = match query_parameter(url, "q") {
        Ok(Some(expression)) => expression,
        Ok(None) => return Reply::new(400, "missing ?q=<expression>"),
        Err(_) => return Reply::new(400, "the query is not valid UTF-8"),
    };
    match engine.evaluate_expression(&expression) {
        Ok(answer) => Reply::new(200, answer.text),
        Err(error) => Reply::new(422, error.message()),
    }
}

/// The percent-decoded value of the first `name=` parameter in `url`'s query. `+` stays a plus
/// sign (it is an operator here, and Numi's workflow encodes spaces as `%20`).
fn query_parameter(url: &str, name: &str) -> Result<Option<String>, Utf8Error> {
    let Some((_, query)) = url.split_once('?') else {
        return Ok(None);
    };
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if decode(key)? == name {
            return decode(value).map(|value| Some(value.into_owned()));
        }
    }
    Ok(None)
}

fn decode(text: &str) -> Result<Cow<'_, str>, Utf8Error> {
    percent_decode_str(text).decode_utf8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_q_parameter() {
        assert_eq!(
            query_parameter("/?q=20%20inches%20in%20cm", "q"),
            Ok(Some("20 inches in cm".to_owned()))
        );
        assert_eq!(
            query_parameter("?q=1+2", "q"),
            Ok(Some("1+2".to_owned())),
            "plus stays a plus"
        );
        assert_eq!(
            query_parameter("/?a=1&q=%E2%82%AC5&q=other", "q"),
            Ok(Some("€5".to_owned()))
        );
        assert_eq!(query_parameter("/?q", "q"), Ok(Some(String::new())));
        assert_eq!(query_parameter("/?query=5", "q"), Ok(None));
        assert_eq!(query_parameter("/favicon.ico", "q"), Ok(None));
        assert!(query_parameter("/?q=%FF", "q").is_err());
    }

    #[test]
    fn answers_like_numi() {
        let engine = Engine::new();
        assert_eq!(answer(&engine, &Method::Get, "/?q=42"), Reply::new(200, "42"));
        assert_eq!(
            answer(&engine, &Method::Get, "?q=%2042%20"),
            Reply::new(200, "42")
        );
        assert_eq!(answer(&engine, &Method::Get, "/").status, 400);
        assert_eq!(answer(&engine, &Method::Get, "/?q=%FF").status, 400);
        assert_eq!(answer(&engine, &Method::Post, "/?q=42").status, 405);
        let failed = answer(&engine, &Method::Get, "/?q=");
        assert_eq!(failed.status, 422);
        assert!(!failed.body.is_empty(), "the error message is the body");
    }
}
