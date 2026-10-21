//! Chronos local workshop: CLI and a small localhost HTTP server (std only).
//!
//! Usage:
//!   chronos-app [serve] [--port 8785] [--root DIR] [--fixture FILE] [--web DIR]
//!   chronos-app generate [--config FILE] [--out FILE] [--root DIR] [--fixture FILE]
//!   chronos-app check WORLD.json [--root DIR] [--fixture FILE]

mod server;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use chronos_core::{generate_world, replay_hash, validate_world, Config, StackFixture, World};

pub const DEFAULT_PORT: u16 = 8785;

pub struct Paths {
    pub fixture: PathBuf,
    /// Path shown to users (relative, never absolute).
    pub fixture_label: String,
    pub web: PathBuf,
}

impl Paths {
    /// Parsed fixture, or the reason it is unavailable (re-read on every call
    /// so a fixture prepared while the server runs is picked up).
    pub fn load_fixture(&self) -> Result<StackFixture, String> {
        let bytes = std::fs::read(&self.fixture).map_err(|_| {
            format!(
                "{} introuvable ; lancez la préparation des données (tools/)",
                self.fixture_label
            )
        })?;
        serde_json::from_slice::<StackFixture>(&bytes)
            .map_err(|e| format!("{} illisible : {e}", self.fixture_label))
    }
}

/// Contract defaults, adapted to the fixture when it is usable so that a
/// default generation uses imported sites, population and radius unchanged.
pub fn default_config(fixture: &Result<StackFixture, String>) -> Config {
    let mut cfg = Config::default();
    if let Ok(fx) = fixture {
        let n = fx.sites.len();
        let total: u64 = fx.sites.iter().map(|s| s.population).sum();
        if (8..=300).contains(&n) {
            cfg.site_count = n;
        }
        if (1_000..=50_000_000).contains(&total) {
            cfg.population = total;
        }
        if fx.map.radius_km.is_finite() && (100.0..=100_000.0).contains(&fx.map.radius_km) {
            cfg.world_radius_km = fx.map.radius_km;
        }
    }
    cfg
}

pub fn timed_generate(
    cfg: &Config,
    fixture: &Result<StackFixture, String>,
) -> Result<World, String> {
    let started = Instant::now();
    let mut world = generate_world(cfg, fixture.as_ref().map_err(|e| e.as_str()))?;
    world.generation_ms = (started.elapsed().as_secs_f64() * 1000.0 * 10.0).round() / 10.0;
    Ok(world)
}

/// Validates an imported world. When the same inputs are available here, the
/// facts must equal their replay: changed facts cannot keep an unchanged canon.
/// Non-blocking diagnostics are added to the warnings.
pub fn check_import(
    mut world: World,
    fixture: &Result<StackFixture, String>,
) -> Result<World, String> {
    let mut notes = validate_world(&world)?;
    let strip = |w: &World| {
        let mut w = w.clone();
        w.warnings.retain(|m| !m.starts_with("Import : "));
        replay_hash(&w)
    };
    match generate_world(&world.config, fixture.as_ref().map_err(|e| e.as_str())) {
        Ok(replay) if replay.canon_id == world.canon_id => {
            if strip(&replay) != strip(&world) {
                return Err(format!(
                    "les faits diffèrent du rejeu du canon {} (export modifié ?) ; régénérez avec la même configuration ou importez l'export d'origine",
                    world.canon_id
                ));
            }
        }
        _ => notes.push(
            "rejeu impossible ici (sources différentes de celles de l'export) : accepté après validation structurelle seulement".into(),
        ),
    }
    for note in notes {
        let line = format!("Import : {note}");
        if !world.warnings.contains(&line) {
            world.warnings.push(line);
        }
    }
    Ok(world)
}

fn find_root(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(root) = explicit {
        return root;
    }
    let looks_like_root = |p: &Path| {
        p.join("web").join("index.html").is_file()
            || p.join("fixtures").join("stack.json").is_file()
            || p.join("crates").join("chronos-core").is_dir()
    };
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if looks_like_root(&cwd) {
        return cwd;
    }
    // Launched from target/{debug,release}: walk up from the executable.
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe.ancestors().skip(1) {
            if looks_like_root(dir) {
                return dir.to_path_buf();
            }
        }
    }
    cwd
}

struct Args {
    command: String,
    positional: Vec<String>,
    port: u16,
    root: Option<PathBuf>,
    fixture: Option<PathBuf>,
    web: Option<PathBuf>,
    config: Option<PathBuf>,
    out: Option<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        command: "serve".into(),
        positional: Vec::new(),
        port: DEFAULT_PORT,
        root: None,
        fixture: None,
        web: None,
        config: None,
        out: None,
    };
    let mut it = std::env::args().skip(1);
    let mut first = true;
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} attend une valeur"));
        match a.as_str() {
            "--port" => {
                args.port = value("--port")?
                    .parse()
                    .map_err(|_| "--port attend un entier".to_string())?
            }
            "--root" => args.root = Some(value("--root")?.into()),
            "--fixture" => args.fixture = Some(value("--fixture")?.into()),
            "--web" => args.web = Some(value("--web")?.into()),
            "--config" => args.config = Some(value("--config")?.into()),
            "--out" => args.out = Some(value("--out")?.into()),
            "-h" | "--help" => args.command = "help".into(),
            _ if first && !a.starts_with('-') => args.command = a,
            _ if !a.starts_with('-') => args.positional.push(a),
            _ => return Err(format!("option inconnue : {a}")),
        }
        first = false;
    }
    Ok(args)
}

const HELP: &str = "Chronos — générateur de chronologie déterministe

  chronos-app [serve] [--port 8785] [--root DIR] [--fixture FILE] [--web DIR]
      Sert l'atelier web et l'API sur http://127.0.0.1:PORT (localhost uniquement).
  chronos-app generate [--config config.json] [--out world.json]
      Génère un monde (generation_ms = 0 pour un export identique octet pour octet).
  chronos-app check world.json
      Valide un export et compare au rejeu si les mêmes sources sont disponibles.";

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("erreur : {e}\n\n{HELP}");
            return ExitCode::from(2);
        }
    };
    let root = find_root(args.root.clone());
    let paths = Paths {
        fixture_label: match &args.fixture {
            Some(p) => p
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => "fixtures/stack.json".into(),
        },
        fixture: args
            .fixture
            .clone()
            .unwrap_or_else(|| root.join("fixtures").join("stack.json")),
        web: args.web.clone().unwrap_or_else(|| root.join("web")),
    };
    let result = match args.command.as_str() {
        "help" => {
            println!("{HELP}");
            Ok(())
        }
        "serve" => server::serve(args.port, paths),
        "generate" => cmd_generate(&args, &paths),
        "check" => cmd_check(&args, &paths),
        other => Err(format!("commande inconnue : {other}\n\n{HELP}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_generate(args: &Args, paths: &Paths) -> Result<(), String> {
    let fixture = paths.load_fixture();
    let cfg = match &args.config {
        Some(p) => {
            let text = std::fs::read_to_string(p)
                .map_err(|e| format!("lecture de la configuration : {e}"))?;
            serde_json::from_str::<Config>(&text)
                .map_err(|e| format!("configuration JSON invalide : {e}"))?
        }
        None => default_config(&fixture),
    };
    let mut world = generate_world(&cfg, fixture.as_ref().map_err(|e| e.as_str()))?;
    world.generation_ms = 0.0;
    let json = serde_json::to_string(&world).map_err(|e| e.to_string())?;
    match &args.out {
        Some(p) => std::fs::write(p, json).map_err(|e| format!("écriture : {e}"))?,
        None => println!("{json}"),
    }
    eprintln!(
        "{} : {} sites, {} routes, {} personnes, {} événements, {} avertissement(s), rejeu {}",
        world.canon_id,
        world.sites.len(),
        world.routes.len(),
        world.people.len(),
        world.events.len(),
        world.warnings.len(),
        replay_hash(&world)
    );
    for w in &world.warnings {
        eprintln!("  ! {w}");
    }
    Ok(())
}

fn cmd_check(args: &Args, paths: &Paths) -> Result<(), String> {
    let file = args
        .positional
        .first()
        .ok_or("check attend un fichier world.json")?;
    let text = std::fs::read_to_string(file).map_err(|e| format!("lecture : {e}"))?;
    let world: World = serde_json::from_str(&text).map_err(|e| format!("JSON invalide : {e}"))?;
    let before = world.warnings.len();
    let world = check_import(world, &paths.load_fixture())?;
    println!(
        "valide : {} ({} événements), rejeu {}",
        world.canon_id,
        world.events.len(),
        replay_hash(&world)
    );
    for w in &world.warnings[before..] {
        println!("  {w}");
    }
    Ok(())
}
