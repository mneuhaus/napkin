//! Numi's Alfred integration: `http://127.0.0.1:15055/?q=<expression>` answers with the
//! formatted result as plain text (empty when the expression computes nothing).

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::thread;

use anyhow::anyhow;
use tiny_http::{Header, Request, Response, Server};

use crate::sheet::SharedEngine;

/// The port Numi's Alfred workflow calls.
pub(crate) const PORT: u16 = 15055;

/// The running endpoint; dropping it stops the server thread.
pub(crate) struct AlfredServer {
    server: Arc<Server>,
}

impl std::fmt::Debug for AlfredServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlfredServer")
            .field("address", &self.server.server_addr().to_ip())
            .finish()
    }
}

impl AlfredServer {
    /// Binds `127.0.0.1:port` (only loopback) and answers on a background thread.
    pub(crate) fn start(engine: SharedEngine, port: u16) -> anyhow::Result<Self> {
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
        let server = Arc::new(Server::http(address).map_err(|error| anyhow!("binding {address}: {error}"))?);
        let worker = Arc::clone(&server);
        thread::Builder::new()
            .name("napkin-alfred".into())
            .spawn(move || {
                for request in worker.incoming_requests() {
                    answer(request, &engine);
                }
            })?;
        Ok(Self { server })
    }
}

impl Drop for AlfredServer {
    fn drop(&mut self) {
        // Ends `incoming_requests`, so the thread finishes on its own.
        self.server.unblock();
    }
}

fn answer(request: Request, engine: &SharedEngine) {
    let body = query_parameter(request.url(), "q")
        .map(|expression| result_text(&expression, engine))
        .unwrap_or_default();
    let response = Response::from_string(body).with_header(plain_text());
    if let Err(error) = request.respond(response) {
        eprintln!("could not answer an Alfred request: {error}");
    }
}

fn plain_text() -> Header {
    Header::from_bytes("Content-Type", "text/plain; charset=utf-8").expect("a static, valid header")
}

/// The formatted result of `expression`, empty when it computes nothing.
fn result_text(expression: &str, engine: &SharedEngine) -> String {
    engine
        .read()
        .expect("engine lock poisoned")
        .evaluate_expression(expression)
        .map(|answer| answer.text)
        .unwrap_or_default()
}

/// The decoded value of `name` in the query string of `url`.
fn query_parameter(url: &str, name: &str) -> Option<String> {
    let (_, query) = url.split_once('?')?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (percent_decode(key) == name).then(|| percent_decode(value))
    })
}

/// Decodes a query value: `+` is a space, `%XX` escapes as in [`percent_decode_path`].
fn percent_decode(text: &str) -> String {
    percent_decode_path(&text.replace('+', " "))
}

/// Decodes `%XX` escapes (UTF-8); invalid escapes stay as they are.
pub(crate) fn percent_decode_path(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
                match hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                    Some(byte) => {
                        decoded.push(byte);
                        index += 2;
                    }
                    None => decoded.push(b'%'),
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use std::io::{Read as _, Write as _};
    use std::net::TcpStream;
    use std::sync::RwLock;

    use napkin_core::Engine;

    use super::*;

    #[test]
    fn query_values_are_percent_decoded() {
        assert_eq!(
            query_parameter("/?q=20%20inches%20in%20cm", "q").as_deref(),
            Some("20 inches in cm")
        );
        assert_eq!(query_parameter("/?x=1&q=1+%2B+2", "q").as_deref(), Some("1 + 2"));
        assert_eq!(query_parameter("/?q=%E2%82%AC5", "q").as_deref(), Some("€5"));
        assert_eq!(query_parameter("/?q=100%", "q").as_deref(), Some("100%"));
        assert_eq!(query_parameter("/?q=%zz", "q").as_deref(), Some("%zz"));
        assert_eq!(query_parameter("/", "q"), None);
    }

    #[test]
    fn the_endpoint_answers_with_plain_text() {
        let engine = Arc::new(RwLock::new(Engine::new()));
        let server = AlfredServer::start(engine, 0).expect("bound");
        let port = server.server.server_addr().to_ip().expect("an IP address").port();
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connected");
        write!(stream, "GET /?q=42 HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("sent");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read");
        assert!(response.starts_with("HTTP/1.0 200"), "{response}");
        assert!(response.contains("text/plain"), "{response}");
        assert!(response.ends_with("\r\n\r\n42"), "{response}");
    }
}
