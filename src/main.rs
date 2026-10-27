//! UPS Monitor: legge l'UPS via USB HID, registra storico e interruzioni
//! e serve una dashboard protetta da password su http://localhost:8770
//!
//! Senza finestra: è pensato per girare sempre in background (avvio automatico).
//! Se lanciato da un terminale, scrive i messaggi in quel terminale.

#![windows_subsystem = "windows"]

mod auth;
mod hid;
mod monitor;

use std::io::{BufRead, Read};
use std::path::PathBuf;
use std::process::Command;

use auth::{Auth, EsitoLogin, NOME_COOKIE, PAGINA_LOGIN};
use monitor::Monitor;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const DASHBOARD: &str = include_str!("../dashboard.html");
const AIUTO: &str = "\
ups-monitor - dashboard e storico per UPS USB (APC Back-UPS e compatibili)

Uso: ups-monitor.exe [opzioni]
  --porta N           porta della dashboard (predefinita 8770)
  --intervallo S      secondi tra una lettura e l'altra (predefinito 2)
  --dati CARTELLA     dove salvare log, interruzioni e config (predefinita: log\\ accanto all'exe)
  --lan               accetta connessioni dalla rete locale (richiede la password)
  --spegni-dopo MIN   in blackout spegne il PC dopo MIN minuti a batteria (predefinito 5, 0 = mai)
  --spegni-autonomia MIN  spegne comunque se l'autonomia scende sotto MIN minuti (predefinito 3)
  --apri              apre la dashboard nel browser all'avvio
  --imposta-password  chiede la nuova password (legge una riga da stdin) ed esce
  --dump              stampa tutti i valori grezzi letti dall'UPS ed esce
  --aiuto             questo messaggio
";

#[link(name = "kernel32")]
extern "system" {
    fn AttachConsole(pid: u32) -> i32;
}

struct Opzioni {
    porta: u16,
    intervallo: f64,
    dati: Option<PathBuf>,
    lan: bool,
    apri: bool,
    dump: bool,
    imposta_password: bool,
    spegni_dopo_min: f64,
    spegni_autonomia_min: f64,
}

fn opzioni() -> Result<Opzioni, String> {
    let mut o = Opzioni {
        porta: 8770, intervallo: 2.0, dati: None, lan: false, apri: false, dump: false,
        imposta_password: false, spegni_dopo_min: 5.0, spegni_autonomia_min: 3.0,
    };
    let minuti = |v: String, nome: &str| -> Result<f64, String> {
        v.replace(',', ".").parse::<f64>().ok().filter(|m| *m >= 0.0)
            .ok_or(format!("{nome}: valore non valido"))
    };
    let mut a = std::env::args().skip(1);
    while let Some(arg) = a.next() {
        let mut valore = |nome: &str| a.next().ok_or(format!("{nome} richiede un valore"));
        match arg.as_str() {
            "--porta" => o.porta = valore("--porta")?.parse().map_err(|_| "porta non valida")?,
            "--intervallo" => {
                o.intervallo = valore("--intervallo")?.replace(',', ".").parse()
                    .map_err(|_| "intervallo non valido")?;
                o.intervallo = o.intervallo.clamp(0.5, 60.0);
            }
            "--dati" => o.dati = Some(PathBuf::from(valore("--dati")?)),
            "--lan" => o.lan = true,
            "--apri" => o.apri = true,
            "--dump" => o.dump = true,
            "--imposta-password" => o.imposta_password = true,
            "--spegni-dopo" => o.spegni_dopo_min = minuti(valore("--spegni-dopo")?, "--spegni-dopo")?,
            "--spegni-autonomia" => {
                o.spegni_autonomia_min = minuti(valore("--spegni-autonomia")?, "--spegni-autonomia")?
            }
            "--aiuto" | "--help" | "-h" | "/?" => return Err(AIUTO.into()),
            altro => return Err(format!("Opzione sconosciuta: {altro}\n\n{AIUTO}")),
        }
    }
    Ok(o)
}

fn apri_browser(url: &str) {
    let _ = Command::new("explorer.exe").arg(url).spawn();
}

fn dump() {
    match hid::Ups::trova() {
        Ok(ups) => {
            println!("\n{}", ups.prodotto);
            for (k, v) in ups.leggi_tutto() {
                let v = match v {
                    hid::Valore::Num(n) => n.to_string(),
                    hid::Valore::Testo(s) => s,
                };
                println!("  {k:60} {v}");
            }
        }
        Err(e) => println!("\n{e}"),
    }
}

fn header(nome: &str, valore: &str) -> Header {
    Header::from_bytes(nome, valore).unwrap()
}

fn leggi_header<'a>(req: &'a Request, nome: &str) -> Option<&'a str> {
    req.headers().iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(nome))
        .map(|h| h.value.as_str())
}

/// Indirizzo del client. Dietro Cloudflare Tunnel la connessione arriva da localhost:
/// solo in quel caso ci fidiamo dell'header con l'IP reale.
fn ip_cliente(req: &Request) -> String {
    let remoto = req.remote_addr().map(|a| a.ip());
    if remoto.is_some_and(|ip| ip.is_loopback()) {
        if let Some(ip) = leggi_header(req, "CF-Connecting-IP")
            .or_else(|| leggi_header(req, "X-Forwarded-For").and_then(|v| v.split(',').next()))
        {
            return ip.trim().to_string();
        }
    }
    remoto.map(|ip| ip.to_string()).unwrap_or_default()
}

fn cookie_sessione(req: &Request) -> Option<String> {
    leggi_header(req, "Cookie")?.split(';').find_map(|c| {
        let (k, v) = c.trim().split_once('=')?;
        (k == NOME_COOKIE).then(|| v.to_string())
    })
}

fn via_https(req: &Request) -> bool {
    leggi_header(req, "X-Forwarded-Proto").is_some_and(|v| v.eq_ignore_ascii_case("https"))
        || leggi_header(req, "CF-Visitor").is_some_and(|v| v.contains("https"))
}

fn pagina_login(errore: &str, codice: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(PAGINA_LOGIN.replace("{{ERRORE}}", errore))
        .with_status_code(codice)
        .with_header(header("Content-Type", "text/html; charset=utf-8"))
}

fn redirect(dove: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string("").with_status_code(303).with_header(header("Location", dove))
}

fn gestisci(mut req: Request, monitor: &Monitor, auth: &Auth) {
    let url = req.url().to_string();
    let (percorso, query) = url.split_once('?').unwrap_or((&url, ""));
    let protetto = auth.attiva();
    let token = cookie_sessione(&req);
    let autenticato = !protetto || token.as_deref().is_some_and(|t| auth.sessione_valida(t));
    let json = || header("Content-Type", "application/json; charset=utf-8");
    let no_cache = || header("Cache-Control", "no-store");

    let risposta = match (req.method(), percorso) {
        (Method::Get, "/login") if !protetto || autenticato => redirect("/"),
        (Method::Get, "/login") => pagina_login("", 200),
        (Method::Post, "/login") => {
            let mut corpo = String::new();
            let _ = req.as_reader().take(4096).read_to_string(&mut corpo);
            let password = corpo.split('&').find_map(|p| p.strip_prefix("password="))
                .map(auth::url_decode).unwrap_or_default();
            match auth.login(&ip_cliente(&req), &password) {
                EsitoLogin::Ok(t) => {
                    let sicuro = if via_https(&req) { "; Secure" } else { "" };
                    redirect("/").with_header(header("Set-Cookie", &format!(
                        "{NOME_COOKIE}={t}; Path=/; Max-Age=2592000; HttpOnly; SameSite=Strict{sicuro}")))
                }
                EsitoLogin::Errata(rimasti) => pagina_login(
                    &format!("Password errata ({rimasti} tentativi rimasti)"), 401),
                EsitoLogin::Bloccato(min) => pagina_login(
                    &format!("Troppi tentativi: riprova tra {min} minuti"), 429),
            }
        }
        (_, "/logout") => {
            if let Some(t) = &token {
                auth.logout(t);
            }
            redirect("/login").with_header(header("Set-Cookie",
                &format!("{NOME_COOKIE}=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict")))
        }
        _ if !autenticato => {
            if percorso.starts_with("/api/") || percorso.ends_with(".csv") {
                Response::from_string(r#"{"errore":"non autenticato"}"#).with_status_code(401).with_header(json())
            } else {
                redirect("/login")
            }
        }
        (Method::Get, "/" | "/index.html") => Response::from_string(DASHBOARD.replace(
            "{{PROTETTO}}", if protetto { "1" } else { "0" }))
            .with_header(header("Content-Type", "text/html; charset=utf-8")).with_header(no_cache()),
        (Method::Get, "/api/stato") => Response::from_string(monitor.stato_json().to_string())
            .with_header(json()).with_header(no_cache()),
        (Method::Get, "/api/storico") => {
            let dal = query.split('&').find_map(|p| p.strip_prefix("dal="))
                .and_then(|v| v.parse().ok()).unwrap_or(0.0);
            Response::from_string(monitor.storico_json(dal).to_string()).with_header(json()).with_header(no_cache())
        }
        (Method::Get, "/api/storico_lungo") => {
            let giorni = query.split('&').find_map(|p| p.strip_prefix("giorni="))
                .and_then(|v| v.parse().ok()).unwrap_or(7);
            Response::from_string(monitor.storico_lungo_json(giorni).to_string())
                .with_header(json()).with_header(no_cache())
        }
        (Method::Get, "/api/interruzioni") => Response::from_string(monitor.interruzioni_json().to_string())
            .with_header(json()).with_header(no_cache()),
        (Method::Get, "/interruzioni.csv") => Response::from_data(monitor.interruzioni_csv())
            .with_header(header("Content-Type", "text/csv; charset=utf-8"))
            .with_header(header("Content-Disposition", "attachment; filename=interruzioni_ups.csv")),
        _ => Response::from_string("Non trovato").with_status_code(StatusCode(404)),
    };
    let risposta = risposta
        .with_header(header("X-Frame-Options", "DENY"))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("Referrer-Policy", "no-referrer"));
    let _ = req.respond(risposta);
}

fn main() {
    // Siamo un'app senza finestra: se c'è un terminale padre, ci agganciamo per l'output
    unsafe { AttachConsole(u32::MAX) };

    let o = match opzioni() {
        Ok(o) => o,
        Err(msg) => {
            println!("\n{msg}");
            return;
        }
    };
    if o.dump {
        return dump();
    }

    let cartella = o.dati.clone().unwrap_or_else(|| {
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("log")))
            .unwrap_or_else(|| PathBuf::from("log"))
    });
    let auth = Auth::nuovo(&cartella);

    if o.imposta_password {
        println!("\nNuova password (poi Invio):");
        let mut riga = String::new();
        let _ = std::io::stdin().lock().read_line(&mut riga);
        let pw = riga.trim_end_matches(['\r', '\n']);
        if pw.chars().count() < 8 {
            println!("Password troppo corta (minimo 8 caratteri): nulla è cambiato");
            return;
        }
        match auth.imposta_password(pw) {
            Ok(()) => println!("Password impostata in {}. Tutte le sessioni precedenti sono state chiuse.",
                               cartella.join("config.json").display()),
            Err(e) => println!("Errore: {e}"),
        }
        return;
    }
    if o.lan && !auth.attiva() {
        println!("\n--lan richiede una password: impostala prima con --imposta-password");
        return;
    }

    let url = format!("http://localhost:{}", o.porta);
    let host = if o.lan { "0.0.0.0" } else { "127.0.0.1" };
    let server = match Server::http((host, o.porta)) {
        Ok(s) => s,
        Err(e) => {
            println!("\nPorta {} occupata ({e}): ups-monitor è già avviato?", o.porta);
            if o.apri {
                apri_browser(&url);
            }
            return;
        }
    };

    let regola = monitor::RegolaSpegnimento {
        dopo_s: (o.spegni_dopo_min > 0.0).then_some(o.spegni_dopo_min * 60.0),
        autonomia_min_s: o.spegni_autonomia_min * 60.0,
        preavviso_s: 60,
    };
    let monitor = Monitor::nuovo(cartella.clone(), o.intervallo, regola);
    {
        let m = monitor.clone();
        std::thread::spawn(move || m.ciclo());
    }
    println!("\nDashboard UPS su {url}  (password: {})\nDati in {}",
             if auth.attiva() { "attiva" } else { "NON impostata" }, cartella.display());
    if o.apri {
        apri_browser(&url);
    }

    // Pochi thread di servizio: il login (PBKDF2) non blocca le altre richieste
    let server = std::sync::Arc::new(server);
    let mut thread = Vec::new();
    for _ in 0..4 {
        let (s, m, a) = (server.clone(), monitor.clone(), auth.clone());
        thread.push(std::thread::spawn(move || {
            for req in s.incoming_requests() {
                gestisci(req, &m, &a);
            }
        }));
    }
    for t in thread {
        let _ = t.join();
    }
}
