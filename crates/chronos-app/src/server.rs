//! Minimal HTTP/1.1 server: JSON API and static files, one thread per connection.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use chronos_core::{Config, World};
use serde_json::json;

use crate::{check_import, default_config, timed_generate, Paths};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 256 * 1024 * 1024;

struct App {
    paths: Paths,
    /// World currently shown (last generated or imported).
    current: Mutex<Option<World>>,
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn json(status: u16, value: &impl serde::Serialize) -> Response {
        let body =
            serde_json::to_vec(value).unwrap_or_else(|_| b"{\"error\":\"serialization\"}".to_vec());
        Response {
            status,
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Response {
        Response::json(status, &json!({ "error": message.into() }))
    }
}

pub fn serve(port: u16, paths: Paths) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| {
        format!("impossible d'écouter sur 127.0.0.1:{port} ({e}) ; essayez --port <autre>")
    })?;
    match &paths.load_fixture() {
        Ok(fx) => println!(
            "Fixture : {} ({} sites, {} personnes)",
            paths.fixture_label,
            fx.sites.len(),
            fx.people.len()
        ),
        Err(e) => println!("Avertissement : {e} — repli sur le monde virtuel synthétique."),
    }
    if !paths.web.join("index.html").is_file() {
        println!("Avertissement : web/index.html absent ; seule l'API répond.");
    }
    println!("Chronos sur http://127.0.0.1:{port}/  (Ctrl+C pour arrêter)");
    let app = Arc::new(App {
        paths,
        current: Mutex::new(None),
    });
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let app = Arc::clone(&app);
        std::thread::spawn(move || {
            if let Err(e) = handle(&app, stream) {
                eprintln!("connexion : {e}");
            }
        });
    }
    Ok(())
}

fn handle(app: &App, stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut head = String::new();
    let mut content_length = 0usize;
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        head.push_str(&line);
        if head.len() > MAX_HEADER_BYTES {
            return write_response(stream, Response::error(431, "en-têtes trop longs"));
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    if content_length > MAX_BODY_BYTES {
        return write_response(
            stream,
            Response::error(413, "corps de requête trop volumineux"),
        );
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let response = route(app, &method, path, &body);
    write_response(stream, response)
}

fn write_response(mut stream: TcpStream, r: Response) -> std::io::Result<()> {
    let reason = match r.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        r.status,
        r.content_type,
        r.body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&r.body)?;
    stream.flush()
}

fn route(app: &App, method: &str, path: &str, body: &[u8]) -> Response {
    match (method, path) {
        ("GET", "/api/health") => Response::json(200, &json!({ "ok": true })),
        ("GET", "/api/defaults") => defaults(app),
        ("GET", "/api/world") => current_world(app),
        ("POST", "/api/generate") => generate(app, body),
        ("POST", "/api/import") => import(app, body),
        (_, p) if p.starts_with("/api/") => {
            if [
                "/api/health",
                "/api/defaults",
                "/api/world",
                "/api/generate",
                "/api/import",
            ]
            .contains(&p)
            {
                Response::error(
                    405,
                    format!("méthode {method} non prise en charge pour {p}"),
                )
            } else {
                Response::error(404, format!("point d'API inconnu : {p}"))
            }
        }
        ("GET", p) | ("HEAD", p) => static_file(&app.paths.web, p),
        _ => Response::error(405, format!("méthode {method} non prise en charge")),
    }
}

fn defaults(app: &App) -> Response {
    let fixture = app.paths.load_fixture();
    let config = default_config(&fixture);
    let world = match timed_generate(&config, &fixture) {
        Ok(w) => w,
        Err(e) => return Response::error(500, format!("génération par défaut impossible : {e}")),
    };
    let mut capabilities = vec![
        "generate",
        "import",
        "export",
        "virtual_map",
        "annual_snapshots",
        "pivot_scenario",
    ];
    if fixture.is_ok()
        && world
            .sources
            .iter()
            .any(|s| s.id == "stack-fixture" && s.status == "imported")
    {
        capabilities.push("stack_map");
    }
    Response::json(
        200,
        &json!({ "config": config, "sources": world.sources, "capabilities": capabilities }),
    )
}

fn current_world(app: &App) -> Response {
    if let Some(w) = app.current.lock().map(|g| g.clone()).unwrap_or(None) {
        return Response::json(200, &w);
    }
    let fixture = app.paths.load_fixture();
    let config = default_config(&fixture);
    match timed_generate(&config, &fixture) {
        Ok(w) => {
            let response = Response::json(200, &w);
            if let Ok(mut guard) = app.current.lock() {
                guard.get_or_insert(w);
            }
            response
        }
        Err(e) => Response::error(500, format!("génération par défaut impossible : {e}")),
    }
}

fn generate(app: &App, body: &[u8]) -> Response {
    let config: Config = if body.iter().all(|b| b.is_ascii_whitespace()) {
        default_config(&app.paths.load_fixture())
    } else {
        match serde_json::from_slice(body) {
            Ok(c) => c,
            Err(e) => return Response::error(400, format!("configuration JSON invalide : {e}")),
        }
    };
    let fixture = app.paths.load_fixture();
    match timed_generate(&config, &fixture) {
        Ok(w) => {
            let response = Response::json(200, &w);
            if let Ok(mut guard) = app.current.lock() {
                *guard = Some(w);
            }
            response
        }
        Err(e) => Response::error(400, e),
    }
}

fn import(app: &App, body: &[u8]) -> Response {
    let world: World = match serde_json::from_slice(body) {
        Ok(w) => w,
        Err(e) => return Response::error(400, format!("export JSON invalide ou incomplet : {e}")),
    };
    match check_import(world, &app.paths.load_fixture()) {
        Ok(w) => {
            let response = Response::json(200, &w);
            if let Ok(mut guard) = app.current.lock() {
                *guard = Some(w);
            }
            response
        }
        Err(e) => Response::error(400, format!("import refusé : {e}")),
    }
}

fn static_file(web: &Path, path: &str) -> Response {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let rel_path = Path::new(rel);
    if rel.contains('\\')
        || rel_path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Response::error(400, "chemin refusé");
    }
    let file = web.join(rel_path);
    match std::fs::read(&file) {
        Ok(body) => {
            let content_type = match file.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8",
                "js" | "mjs" => "text/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "json" => "application/json; charset=utf-8",
                "svg" => "image/svg+xml",
                "png" => "image/png",
                "ico" => "image/x-icon",
                "woff2" => "font/woff2",
                "md" | "txt" => "text/plain; charset=utf-8",
                _ => "application/octet-stream",
            };
            Response { status: 200, content_type, body }
        }
        Err(_) if rel == "index.html" => Response {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: "<!doctype html><meta charset=utf-8><title>Chronos</title><p>Interface web absente (web/index.html). L'API répond : <a href=\"/api/health\">/api/health</a>, <a href=\"/api/defaults\">/api/defaults</a>, <a href=\"/api/world\">/api/world</a>.</p>"
                .as_bytes()
                .to_vec(),
        },
        Err(_) => Response::error(404, format!("fichier introuvable : /{rel}")),
    }
}
