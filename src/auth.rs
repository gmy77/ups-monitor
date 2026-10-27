//! Accesso con password unica.
//!
//! - La password non è mai salvata in chiaro: `config.json` contiene salt + hash PBKDF2-SHA256
//! - Dopo il login il browser riceve un cookie di sessione (30 giorni); su disco
//!   (`sessioni.json`) finisce solo l'hash SHA-256 del token
//! - Anti forza bruta: 5 tentativi errati per indirizzo => blocco di 15 minuti

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::monitor::adesso;

const ITERAZIONI: u32 = 300_000;
const DURATA_SESSIONE: f64 = 30.0 * 24.0 * 3600.0;
const MAX_TENTATIVI: u32 = 5;
const BLOCCO_SECONDI: f64 = 15.0 * 60.0;
pub const NOME_COOKIE: &str = "ups_sessione";

#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(alg: *mut std::ffi::c_void, buf: *mut u8, len: u32, flags: u32) -> i32;
}

fn casuale(n: usize) -> Vec<u8> {
    let mut v = vec![0u8; n];
    // BCRYPT_USE_SYSTEM_PREFERRED_RNG = 2
    let st = unsafe { BCryptGenRandom(std::ptr::null_mut(), v.as_mut_ptr(), n as u32, 2) };
    assert!(st == 0, "BCryptGenRandom fallita");
    v
}

fn esa(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn da_esa(s: &str) -> Vec<u8> {
    (0..s.len() / 2).filter_map(|i| u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()).collect()
}

/// Confronto a tempo costante: non rivela quanti byte coincidono.
fn uguali(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn deriva(password: &str, salt: &[u8], iterazioni: u32) -> Vec<u8> {
    let mut out = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterazioni, &mut out);
    out.to_vec()
}

#[derive(Serialize, Deserialize)]
struct Config {
    salt: String,
    hash: String,
    iterazioni: u32,
}

struct Interno {
    config: Option<Config>,
    sessioni: HashMap<String, f64>, // hash del token -> scadenza
    tentativi: HashMap<String, (u32, f64)>, // ip -> (errori, bloccato fino a)
}

#[derive(Clone)]
pub struct Auth {
    dentro: Arc<Mutex<Interno>>,
    file_config: PathBuf,
    file_sessioni: PathBuf,
}

pub enum EsitoLogin {
    Ok(String),
    Errata(u32),
    Bloccato(u64),
}

impl Auth {
    pub fn nuovo(cartella: &std::path::Path) -> Auth {
        let file_config = cartella.join("config.json");
        let file_sessioni = cartella.join("sessioni.json");
        let config = fs::read_to_string(&file_config).ok().and_then(|s| serde_json::from_str(&s).ok());
        let mut sessioni: HashMap<String, f64> = fs::read_to_string(&file_sessioni).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let ora = adesso();
        sessioni.retain(|_, scad| *scad > ora);
        Auth {
            dentro: Arc::new(Mutex::new(Interno { config, sessioni, tentativi: HashMap::new() })),
            file_config,
            file_sessioni,
        }
    }

    pub fn attiva(&self) -> bool {
        self.dentro.lock().unwrap().config.is_some()
    }

    /// Imposta (o cambia) la password e invalida tutte le sessioni esistenti.
    pub fn imposta_password(&self, password: &str) -> std::io::Result<()> {
        let salt = casuale(16);
        let cfg = Config { salt: esa(&salt), hash: esa(&deriva(password, &salt, ITERAZIONI)), iterazioni: ITERAZIONI };
        if let Some(dir) = self.file_config.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&self.file_config, serde_json::to_string_pretty(&cfg).unwrap())?;
        let mut d = self.dentro.lock().unwrap();
        d.config = Some(cfg);
        d.sessioni.clear();
        let _ = fs::remove_file(&self.file_sessioni);
        Ok(())
    }

    fn salva_sessioni(&self, d: &Interno) {
        let _ = fs::write(&self.file_sessioni, serde_json::to_string(&d.sessioni).unwrap_or_default());
    }

    pub fn login(&self, ip: &str, password: &str) -> EsitoLogin {
        let ora = adesso();
        let mut d = self.dentro.lock().unwrap();
        if let Some(&(n, fino)) = d.tentativi.get(ip) {
            if n >= MAX_TENTATIVI && fino > ora {
                return EsitoLogin::Bloccato(((fino - ora) / 60.0).ceil() as u64);
            }
        }
        let corretta = match &d.config {
            Some(c) => uguali(&deriva(password, &da_esa(&c.salt), c.iterazioni), &da_esa(&c.hash)),
            None => false,
        };
        if !corretta {
            let voce = d.tentativi.entry(ip.to_string()).or_insert((0, 0.0));
            if voce.1 <= ora && voce.0 >= MAX_TENTATIVI {
                *voce = (0, 0.0); // blocco scaduto: si riparte da zero
            }
            voce.0 += 1;
            voce.1 = ora + BLOCCO_SECONDI;
            let errori = voce.0;
            eprintln!("Login fallito da {ip} ({errori}/{MAX_TENTATIVI})");
            return if errori >= MAX_TENTATIVI {
                EsitoLogin::Bloccato((BLOCCO_SECONDI / 60.0) as u64)
            } else {
                EsitoLogin::Errata(MAX_TENTATIVI - errori)
            };
        }
        d.tentativi.remove(ip);
        let token = esa(&casuale(32));
        d.sessioni.retain(|_, scad| *scad > ora);
        d.sessioni.insert(esa(&Sha256::digest(token.as_bytes())), ora + DURATA_SESSIONE);
        self.salva_sessioni(&d);
        EsitoLogin::Ok(token)
    }

    pub fn sessione_valida(&self, token: &str) -> bool {
        let chiave = esa(&Sha256::digest(token.as_bytes()));
        let d = self.dentro.lock().unwrap();
        d.sessioni.get(&chiave).is_some_and(|scad| *scad > adesso())
    }

    pub fn logout(&self, token: &str) {
        let chiave = esa(&Sha256::digest(token.as_bytes()));
        let mut d = self.dentro.lock().unwrap();
        if d.sessioni.remove(&chiave).is_some() {
            self.salva_sessioni(&d);
        }
    }
}

/// Decodifica un valore application/x-www-form-urlencoded.
pub fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let cifra = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => match (cifra(b[i + 1]), cifra(b[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub const PAGINA_LOGIN: &str = r##"<!doctype html>
<html lang="it"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>UPS Monitor – Accesso</title>
<style>
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;font:15px system-ui,Segoe UI,sans-serif;color:#e6edf7;
background:radial-gradient(1200px 600px at 20% -10%,#1d3557 0,transparent 60%),radial-gradient(900px 500px at 110% 110%,#3a1d57 0,transparent 55%),#0a0f1a}
form{width:min(360px,calc(100vw - 32px));padding:32px;border-radius:18px;background:linear-gradient(160deg,#141c2e,#0f1524);border:1px solid #23304a;box-shadow:0 20px 60px #0008}
h1{margin:0 0 4px;font-size:22px}p{margin:0 0 22px;color:#8a9bb8}
input{width:100%;padding:13px 14px;border-radius:10px;border:1px solid #2b3a58;background:#0b111d;color:#fff;font-size:16px;outline:none}
input:focus{border-color:#4fa3ff;box-shadow:0 0 0 3px #4fa3ff33}
button{margin-top:16px;width:100%;padding:13px;border:0;border-radius:10px;font-weight:600;font-size:15px;color:#fff;cursor:pointer;background:linear-gradient(90deg,#2f80ed,#7b5cff)}
.err{margin-top:14px;color:#ff7a7a;min-height:1.2em}
</style></head><body>
<form method="post" action="/login"><h1>⚡ UPS Monitor</h1><p>Accesso riservato</p>
<input type="password" name="password" placeholder="Password" autofocus autocomplete="current-password" required>
<button type="submit">Entra</button><div class="err">{{ERRORE}}</div></form>
</body></html>"##;
