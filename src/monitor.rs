//! Monitoraggio continuo: normalizzazione dei dati, storico, eventi e registro interruzioni.
//!
//! File in `log/` accanto all'eseguibile:
//! - `ups_AAAA-MM-GG.csv`   un campione ogni intervallo (formato Excel italiano)
//! - `interruzioni.csv`     una riga per ogni blackout, permanente, mai troncato
//! - `eventi.jsonl`         eventi (rete persa/tornata, batteria bassa, ...)
//! - `interruzione_in_corso.json`  blackout aperto: sopravvive allo spegnimento del PC

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::hid::{Grezzi, Ups, Valore};

const STORICO_SECONDI: f64 = 24.0 * 3600.0;
const MAX_EVENTI: usize = 200;

const CAMPI_INTERRUZIONE: [&str; 11] = [
    "inizio", "fine", "durata_s", "durata", "causa", "carica_inizio_pct", "carica_min_pct",
    "tensione_batteria_min", "carico_max_w", "tensione_ingresso_prima", "chiusura",
];
const CAMPI_CAMPIONE: [&str; 14] = [
    "ts", "ora", "tensione_ingresso", "tensione_uscita", "frequenza", "carico_pct", "carico_w",
    "carica_pct", "autonomia_s", "tensione_batteria", "temperatura_c", "rete", "in_carica",
    "in_scarica",
];

// Codici proprietari APC (pagina 0xFF86), come documentati nel progetto NUT
fn causa_trasferimento(c: f64) -> String {
    match c as i64 {
        0 => "Nessun trasferimento dall'accensione",
        1 => "Tensione di rete alta",
        2 => "Calo di tensione (brownout)",
        3 => "Mancanza di rete",
        4 => "Piccolo abbassamento temporaneo",
        5 => "Grosso abbassamento temporaneo",
        6 => "Piccolo picco",
        7 => "Grosso picco",
        8 => "Autotest dell'UPS",
        9 => "Fluttuazione eccessiva della tensione",
        n => return format!("Codice {n}"),
    }
    .to_string()
}

fn sensibilita(c: f64) -> String {
    match c as i64 { 0 => "Alta", 1 => "Media", 2 => "Bassa", _ => "?" }.to_string()
}

fn esito_test(c: f64) -> String {
    match c as i64 {
        1 => "Superato", 2 => "Avviso", 3 => "Errore", 4 => "Interrotto",
        5 => "In corso", 6 => "Nessun test eseguito", _ => "?",
    }
    .to_string()
}

fn allarme(c: f64) -> String {
    match c as i64 { 1 => "Disattivato", 2 => "Attivo", 3 => "Silenziato", _ => "?" }.to_string()
}

pub fn adesso() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn data_ora(ts: f64) -> String {
    Local.timestamp_opt(ts as i64, 0).single()
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

pub fn durata_testo(s: f64) -> String {
    let s = s.round() as i64;
    if s < 60 {
        format!("{s} s")
    } else if s < 3600 {
        format!("{} min {} s", s / 60, s % 60)
    } else {
        format!("{} h {} min", s / 3600, (s % 3600) / 60)
    }
}

/// Data HID: (anno-1980)*512 + mese*32 + giorno.
fn data_hid(v: f64) -> Option<NaiveDate> {
    let v = v as i64;
    NaiveDate::from_ymd_opt(1980 + (v >> 9) as i32, ((v >> 5) & 0xF) as u32, (v & 0x1F) as u32)
}

/// Data sostituzione batteria APC: cifre esadecimali MMGGAA (0x040308 = 03/04/2008).
fn data_apc(v: f64) -> Option<NaiveDate> {
    let s = format!("{:06X}", v as i64);
    let n = |a: usize| s.get(a..a + 2)?.parse::<u32>().ok();
    NaiveDate::from_ymd_opt(2000 + n(4)? as i32, n(0)?, n(2)?)
}

// --- Formato CSV per Excel italiano ---------------------------------------
fn num_csv(v: Option<f64>) -> String {
    match v {
        Some(x) if x.fract() == 0.0 => format!("{}", x as i64),
        Some(x) => format!("{}", (x * 100.0).round() / 100.0).replace('.', ","),
        None => String::new(),
    }
}

fn testo_csv(s: &str) -> String {
    s.replace([';', '\n', '\r'], ",")
}

fn leggi_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}

/// Legge un CSV con separatore ';' scritto da noi: restituisce (intestazione, righe).
fn leggi_csv(path: &Path) -> Vec<Vec<(String, String)>> {
    let Ok(f) = File::open(path) else { return Vec::new() };
    let mut righe = BufReader::new(f).lines().map_while(Result::ok);
    let Some(testa) = righe.next() else { return Vec::new() };
    let campi: Vec<String> = testa.trim_start_matches('\u{feff}').split(';').map(String::from).collect();
    righe
        .filter(|r| !r.trim().is_empty())
        .map(|r| campi.iter().cloned().zip(r.split(';').map(String::from)).collect())
        .collect()
}

fn apri_csv(path: &Path, intestazione: &[&str]) -> std::io::Result<File> {
    let nuovo = !path.exists();
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    if nuovo {
        // BOM UTF-8: Excel riconosce gli accenti
        write!(f, "\u{feff}{}\r\n", intestazione.join(";"))?;
    }
    Ok(f)
}

// --- Strutture --------------------------------------------------------------
#[derive(Serialize, Clone, Default)]
pub struct Misure {
    tensione_ingresso: Option<f64>,
    tensione_uscita: Option<f64>,
    frequenza: Option<f64>,
    carico_pct: Option<f64>,
    carico_w: Option<f64>,
    carica_pct: Option<f64>,
    autonomia_s: Option<f64>,
    tensione_batteria: Option<f64>,
    temperatura_c: Option<f64>,
}

#[derive(Serialize, Clone, Default, PartialEq)]
pub struct Stato {
    rete: bool,
    batteria_presente: bool,
    in_carica: bool,
    in_scarica: bool,
    batteria_bassa: bool,
    autonomia_esaurita: bool,
    sovraccarico: bool,
    sostituire_batteria: bool,
    spegnimento_imminente: bool,
    spegnimento_richiesto: bool,
    comunicazione_persa: bool,
    tensione_non_regolata: bool,
}

#[derive(Serialize, Clone)]
pub struct Lettura {
    ts: f64,
    misure: Misure,
    stato: Stato,
    info: Value,
    grezzi: Grezzi,
    #[serde(skip)]
    causa: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Punto {
    t: f64,
    vin: Option<f64>,
    load: Option<f64>,
    w: Option<f64>,
    chg: Option<f64>,
    run: Option<f64>,
    vbat: Option<f64>,
    temp: Option<f64>,
    ac: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Evento {
    ts: f64,
    livello: String,
    testo: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InterruzioneAperta {
    inizio: f64,
    ultimo: f64,
    causa: Option<String>,
    carica_inizio_pct: Option<f64>,
    carica_min_pct: Option<f64>,
    tensione_batteria_min: Option<f64>,
    carico_max_w: Option<f64>,
    tensione_ingresso_prima: Option<f64>,
    #[serde(default)]
    recuperata: bool,
    /// Istante in cui il monitor ha comandato lo spegnimento del PC
    #[serde(default)]
    spegnimento_pc: Option<f64>,
}

/// Regola di spegnimento del PC durante un blackout (come PowerChute).
#[derive(Clone, Copy, Serialize)]
pub struct RegolaSpegnimento {
    /// Spegni dopo questi secondi a batteria (None = mai per tempo)
    pub dopo_s: Option<f64>,
    /// Spegni quando l'autonomia stimata scende sotto questi secondi
    pub autonomia_min_s: f64,
    /// Preavviso dato da Windows prima dello spegnimento
    pub preavviso_s: u32,
}

fn esegui_shutdown(argomenti: &[&str]) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    match std::process::Command::new("shutdown.exe").args(argomenti).creation_flags(CREATE_NO_WINDOW).status() {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("shutdown {argomenti:?} terminato con {s}"),
        Err(e) => eprintln!("Impossibile eseguire shutdown: {e}"),
    }
}

pub fn normalizza(r: Grezzi) -> Lettura {
    let n = |k: &str| r.get(k).and_then(Valore::num);
    let t = |k: &str| match r.get(k) {
        Some(Valore::Testo(s)) => Some(s.clone()),
        _ => None,
    };
    let b = |k: &str| n(k).unwrap_or(0.0) != 0.0;

    let potenza_nom = n("UPS.Output.ConfigActivePower");
    let carico = n("UPS.Output.PercentLoad");
    let data_batt = n("UPS.Battery.0xFF86:0x0016").and_then(data_apc)
        .or_else(|| n("UPS.Battery.ManufactureDate").and_then(data_hid));
    let oggi = Local::now().date_naive();
    let eta = data_batt.map(|d| ((oggi - d).num_days() as f64 / 365.25 * 10.0).round() / 10.0);
    let causa = n("UPS.Input.0xFF86:0x0052").map(causa_trasferimento);

    let misure = Misure {
        tensione_ingresso: n("UPS.Input.Voltage"),
        tensione_uscita: n("UPS.Output.Voltage"),
        frequenza: n("UPS.Output.Frequency"),
        carico_pct: carico,
        carico_w: carico.zip(potenza_nom).map(|(c, p)| (c * p / 10.0).round() / 10.0),
        carica_pct: n("UPS.PowerSummary.RemainingCapacity"),
        autonomia_s: n("UPS.PowerSummary.RunTimeToEmpty"),
        tensione_batteria: n("UPS.PowerSummary.Voltage"),
        temperatura_c: n("UPS.Battery.Temperature").map(|k| ((k - 273.15) * 10.0).round() / 10.0),
    };
    let stato = Stato {
        rete: b("UPS.PresentStatus.ACPresent"),
        batteria_presente: b("UPS.PresentStatus.BatteryPresent"),
        in_carica: b("UPS.PresentStatus.Charging"),
        in_scarica: b("UPS.PresentStatus.Discharging"),
        batteria_bassa: b("UPS.PresentStatus.BelowRemainingCapacityLimit"),
        autonomia_esaurita: b("UPS.PresentStatus.RemainingTimeLimitExpired"),
        sovraccarico: b("UPS.PresentStatus.Overload"),
        sostituire_batteria: b("UPS.PresentStatus.NeedReplacement"),
        spegnimento_imminente: b("UPS.PresentStatus.ShutdownImminent"),
        spegnimento_richiesto: b("UPS.PresentStatus.ShutdownRequested"),
        comunicazione_persa: b("UPS.PresentStatus.CommunicationLost"),
        tensione_non_regolata: b("UPS.PresentStatus.VoltageNotRegulated"),
    };
    let info = json!({
        "produttore": t("UPS.iManufacturer"),
        "modello": t("UPS.iProduct"),
        "prodotto_completo": t("UPS.PowerSummary.iProduct"),
        "seriale": t("UPS.iSerialNumber"),
        "chimica": t("UPS.PowerSummary.iDeviceChemistry"),
        "data_produzione_ups": n("UPS.ManufactureDate").and_then(data_hid).map(|d| d.to_string()),
        "data_batteria": data_batt.map(|d| d.to_string()),
        "eta_batteria_anni": eta,
        "potenza_nominale_w": potenza_nom,
        "tensione_nominale": n("UPS.Input.ConfigVoltage"),
        "tensione_batteria_nominale": n("UPS.PowerSummary.ConfigVoltage"),
        "soglia_bassa_v": n("UPS.Output.LowVoltageTransfer"),
        "soglia_alta_v": n("UPS.Output.HighVoltageTransfer"),
        "sensibilita": n("UPS.Input.0xFF86:0x0061").map(sensibilita),
        "ultimo_trasferimento": causa,
        "esito_autotest": n("UPS.Battery.Test").map(esito_test),
        "allarme_acustico": n("UPS.AudibleAlarmControl").map(allarme),
        "soglia_batteria_bassa_pct": n("UPS.PowerSummary.RemainingCapacityLimit"),
        "soglia_avviso_pct": n("UPS.PowerSummary.WarningCapacityLimit"),
        "soglia_autonomia_s": n("UPS.PowerSummary.RemainingTimeLimit"),
        "ritardo_spegnimento_s": n("UPS.Output.DelayBeforeShutdown"),
        "ritardo_riavvio_s": n("UPS.Output.DelayBeforeReboot"),
    });
    Lettura { ts: adesso(), misure, stato, info, grezzi: r, causa }
}

/// Eventi rilevati confrontando lo stato con quello precedente.
/// (Rete persa/tornata sono gestiti dal registro interruzioni.)
fn regole_eventi(s: &Stato) -> [(bool, bool, &'static str, &'static str); 7] {
    [
        (s.rete, false, "critico", "Mancanza di rete: UPS a batteria"),
        (s.batteria_bassa, true, "critico", "Batteria sotto la soglia minima"),
        (s.sovraccarico, true, "critico", "Sovraccarico in uscita"),
        (s.sostituire_batteria, true, "avviso", "L'UPS chiede di sostituire la batteria"),
        (s.spegnimento_imminente, true, "critico", "Spegnimento imminente"),
        (s.in_carica, true, "info", "Batteria in carica"),
        (s.in_carica, false, "info", "Carica completata"),
    ]
}

struct Interno {
    ultimo: Option<Lettura>,
    errore: Option<String>,
    storico: VecDeque<Punto>,
    eventi: VecDeque<Evento>,
    interruzione: Option<InterruzioneAperta>,
    campioni: u64,
}

#[derive(Clone)]
pub struct Monitor {
    dentro: Arc<Mutex<Interno>>,
    cartella: PathBuf,
    pub intervallo: f64,
    avvio: f64,
    spegnimento: RegolaSpegnimento,
}

impl Monitor {
    pub fn nuovo(cartella: PathBuf, intervallo: f64, spegnimento: RegolaSpegnimento) -> Monitor {
        fs::create_dir_all(&cartella).ok();
        let m = Monitor {
            dentro: Arc::new(Mutex::new(Interno {
                ultimo: None, errore: None, storico: VecDeque::new(), eventi: VecDeque::new(),
                interruzione: None, campioni: 0,
            })),
            cartella,
            intervallo,
            avvio: adesso(),
            spegnimento,
        };
        m.carica_da_disco();
        m
    }

    fn file(&self, nome: &str) -> PathBuf {
        self.cartella.join(nome)
    }

    fn carica_da_disco(&self) {
        let mut d = self.dentro.lock().unwrap();
        // Eventi
        if let Ok(f) = File::open(self.file("eventi.jsonl")) {
            for r in BufReader::new(f).lines().map_while(Result::ok) {
                if let Ok(e) = serde_json::from_str(&r) {
                    d.eventi.push_back(e);
                }
            }
            while d.eventi.len() > MAX_EVENTI {
                d.eventi.pop_front();
            }
        }
        // Storico delle ultime 24 h dai CSV giornalieri: un riavvio non azzera i grafici
        let limite = adesso() - STORICO_SECONDI;
        let mut csv: Vec<PathBuf> = fs::read_dir(&self.cartella).into_iter().flatten().flatten()
            .map(|e| e.path())
            .filter(|p| p.file_name().and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("ups_") && n.ends_with(".csv")))
            .collect();
        csv.sort();
        for p in csv.iter().rev().take(2).rev() {
            for r in leggi_csv(p) {
                let g = |k: &str| r.iter().find(|(c, _)| c == k).and_then(|(_, v)| leggi_num(v));
                let Some(t) = g("ts") else { continue };
                if t < limite {
                    continue;
                }
                d.storico.push_back(Punto {
                    t, vin: g("tensione_ingresso"), load: g("carico_pct"), w: g("carico_w"),
                    chg: g("carica_pct"), run: g("autonomia_s"), vbat: g("tensione_batteria"),
                    temp: g("temperatura_c"), ac: g("rete").unwrap_or(1.0) as u8,
                });
            }
        }
        // Blackout rimasto aperto: il monitor si è fermato durante l'interruzione
        // (tipicamente il PC si è spento a batteria scarica). Si chiude al primo campione.
        if let Ok(s) = fs::read_to_string(self.file("interruzione_in_corso.json")) {
            if let Ok(mut it) = serde_json::from_str::<InterruzioneAperta>(&s) {
                it.recuperata = true;
                d.interruzione = Some(it);
            }
        }
    }

    // --- Eventi e registro --------------------------------------------------
    fn evento(&self, d: &mut Interno, livello: &str, testo: String) {
        let e = Evento { ts: adesso(), livello: livello.into(), testo };
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(self.file("eventi.jsonl")) {
            let _ = writeln!(f, "{}", serde_json::to_string(&e).unwrap_or_default());
        }
        d.eventi.push_back(e);
        while d.eventi.len() > MAX_EVENTI {
            d.eventi.pop_front();
        }
    }

    fn salva_in_corso(&self, it: &InterruzioneAperta) {
        if let Ok(s) = serde_json::to_string(it) {
            let _ = fs::write(self.file("interruzione_in_corso.json"), s);
        }
    }

    fn chiudi_interruzione(&self, d: &mut Interno, fine: f64, chiusura: &str) -> f64 {
        let Some(it) = d.interruzione.take() else { return 0.0 };
        let durata = (fine - it.inizio).max(0.0);
        let riga = [
            data_ora(it.inizio), data_ora(fine), num_csv(Some((durata * 10.0).round() / 10.0)),
            durata_testo(durata), testo_csv(it.causa.as_deref().unwrap_or("")),
            num_csv(it.carica_inizio_pct), num_csv(it.carica_min_pct),
            num_csv(it.tensione_batteria_min), num_csv(it.carico_max_w),
            num_csv(it.tensione_ingresso_prima), testo_csv(chiusura),
        ];
        match apri_csv(&self.file("interruzioni.csv"), &CAMPI_INTERRUZIONE) {
            Ok(mut f) => {
                let _ = write!(f, "{}\r\n", riga.join(";"));
            }
            Err(e) => eprintln!("Impossibile scrivere interruzioni.csv: {e}"),
        }
        let _ = fs::remove_file(self.file("interruzione_in_corso.json"));
        durata
    }

    fn gestisci_interruzioni(&self, d: &mut Interno, l: &Lettura, prec: Option<&Lettura>) {
        let rete = l.stato.rete;
        let m = &l.misure;

        if d.interruzione.as_ref().is_some_and(|i| i.recuperata) {
            if rete {
                let it = d.interruzione.as_ref().unwrap();
                let (fine, spento) = (it.ultimo, it.spegnimento_pc);
                let chiusura = match spento {
                    Some(t) => format!("PC spento dal monitor alle {} (fine blackout non osservata)",
                                       &data_ora(t)[11..16]),
                    None => "monitor fermo durante il blackout (fine stimata)".into(),
                };
                let durata = self.chiudi_interruzione(d, fine, &chiusura);
                self.evento(d, "avviso", format!(
                    "Recuperata un'interruzione rimasta aperta (almeno {})", durata_testo(durata)));
                return;
            }
            // Il PC è stato riacceso a blackout ancora in corso: la regola vale di nuovo
            let it = d.interruzione.as_mut().unwrap();
            it.recuperata = false;
            it.spegnimento_pc = None;
        }

        // Micro-interruzione più breve dell'intervallo di lettura: la rete risulta sempre
        // presente, ma l'UPS ha aggiornato la causa dell'ultimo trasferimento
        if let Some(p) = prec {
            if rete && p.stato.rete && d.interruzione.is_none() && l.causa != p.causa {
                d.interruzione = Some(InterruzioneAperta {
                    inizio: l.ts, ultimo: l.ts, causa: l.causa.clone(),
                    carica_inizio_pct: m.carica_pct,
                    tensione_ingresso_prima: p.misure.tensione_ingresso,
                    ..Default::default()
                });
                let chiusura = format!("micro-interruzione (< {} s)", self.intervallo);
                self.chiudi_interruzione(d, l.ts, &chiusura);
                self.evento(d, "avviso", format!("Micro-interruzione: {}", l.causa.clone().unwrap_or_default()));
                return;
            }
        }

        if !rete {
            match d.interruzione.as_mut() {
                None => {
                    let it = InterruzioneAperta {
                        inizio: l.ts, ultimo: l.ts, causa: l.causa.clone(),
                        carica_inizio_pct: m.carica_pct, carica_min_pct: m.carica_pct,
                        tensione_batteria_min: m.tensione_batteria, carico_max_w: m.carico_w,
                        tensione_ingresso_prima: prec.and_then(|p| p.misure.tensione_ingresso),
                        recuperata: false,
                        spegnimento_pc: None,
                    };
                    self.salva_in_corso(&it);
                    d.interruzione = Some(it);
                }
                Some(it) => {
                    it.ultimo = l.ts;
                    if l.causa.is_some() {
                        it.causa = l.causa.clone();
                    }
                    let min = |a: Option<f64>, b: Option<f64>| match (a, b) {
                        (Some(x), Some(y)) => Some(x.min(y)), (x, y) => x.or(y),
                    };
                    let max = |a: Option<f64>, b: Option<f64>| match (a, b) {
                        (Some(x), Some(y)) => Some(x.max(y)), (x, y) => x.or(y),
                    };
                    it.carica_min_pct = min(it.carica_min_pct, m.carica_pct);
                    it.tensione_batteria_min = min(it.tensione_batteria_min, m.tensione_batteria);
                    it.carico_max_w = max(it.carico_max_w, m.carico_w);
                }
            }
            self.controlla_spegnimento(d, l);
            if let Some(it) = d.interruzione.clone() {
                self.salva_in_corso(&it);
            }
        } else if d.interruzione.is_some() {
            if d.interruzione.as_ref().unwrap().spegnimento_pc.is_some() {
                esegui_shutdown(&["/a"]);
                self.evento(d, "ok", "Rete tornata in tempo: spegnimento del PC annullato".into());
            }
            let durata = self.chiudi_interruzione(d, l.ts, "rete tornata");
            self.evento(d, "ok", format!("Rete tornata dopo {}", durata_testo(durata)));
        }
    }

    /// Regola "alla PowerChute": a batteria da troppo tempo o autonomia agli sgoccioli
    /// => spegnimento del PC con preavviso (annullato se la rete torna prima).
    fn controlla_spegnimento(&self, d: &mut Interno, l: &Lettura) {
        let r = self.spegnimento;
        let Some(it) = d.interruzione.as_ref() else { return };
        if it.spegnimento_pc.is_some() {
            return;
        }
        let a_batteria = l.ts - it.inizio;
        let motivo = if r.dopo_s.is_some_and(|s| a_batteria >= s) {
            format!("a batteria da {}", durata_testo(a_batteria))
        } else if l.misure.autonomia_s.is_some_and(|s| s < r.autonomia_min_s) {
            format!("autonomia residua {}", durata_testo(l.misure.autonomia_s.unwrap()))
        } else if l.stato.batteria_bassa || l.stato.spegnimento_imminente {
            "l'UPS segnala batteria scarica".into()
        } else {
            return;
        };
        let messaggio = format!(
            "Blackout: {motivo}. Il PC si spegnerà tra {} secondi. Salva il lavoro!", r.preavviso_s);
        esegui_shutdown(&["/s", "/t", &r.preavviso_s.to_string(), "/c", &messaggio, "/d", "u:6:12"]);
        d.interruzione.as_mut().unwrap().spegnimento_pc = Some(l.ts);
        self.evento(d, "critico", format!("Spegnimento del PC avviato ({motivo}), preavviso {} s", r.preavviso_s));
    }

    fn registra_csv(&self, l: &Lettura) {
        let oggi = Local::now();
        let nome = format!("ups_{:04}-{:02}-{:02}.csv", oggi.year(), oggi.month(), oggi.day());
        let m = &l.misure;
        let riga = [
            format!("{}", l.ts as i64), oggi.format("%H:%M:%S").to_string(),
            num_csv(m.tensione_ingresso), num_csv(m.tensione_uscita), num_csv(m.frequenza),
            num_csv(m.carico_pct), num_csv(m.carico_w), num_csv(m.carica_pct), num_csv(m.autonomia_s),
            num_csv(m.tensione_batteria), num_csv(m.temperatura_c),
            (l.stato.rete as u8).to_string(), (l.stato.in_carica as u8).to_string(),
            (l.stato.in_scarica as u8).to_string(),
        ];
        if let Ok(mut f) = apri_csv(&self.file(&nome), &CAMPI_CAMPIONE) {
            let _ = write!(f, "{}\r\n", riga.join(";"));
        }
    }

    fn elabora(&self, l: Lettura) {
        let mut d = self.dentro.lock().unwrap();
        let prec = d.ultimo.take();
        match &prec {
            Some(p) => {
                for ((prima, _, _, _), (ora, valore, livello, testo)) in
                    regole_eventi(&p.stato).into_iter().zip(regole_eventi(&l.stato))
                {
                    if prima != ora && ora == valore {
                        self.evento(&mut d, livello, testo.into());
                    }
                }
            }
            None if d.errore.is_none() => self.evento(&mut d, "info", "Monitoraggio avviato".into()),
            None => {}
        }
        if d.errore.take().is_some() {
            self.evento(&mut d, "ok", "Comunicazione con l'UPS ristabilita".into());
        }
        self.gestisci_interruzioni(&mut d, &l, prec.as_ref());

        let m = &l.misure;
        d.storico.push_back(Punto {
            t: l.ts, vin: m.tensione_ingresso, load: m.carico_pct, w: m.carico_w,
            chg: m.carica_pct, run: m.autonomia_s, vbat: m.tensione_batteria,
            temp: m.temperatura_c, ac: l.stato.rete as u8,
        });
        let limite = l.ts - STORICO_SECONDI;
        while d.storico.front().is_some_and(|p| p.t < limite) {
            d.storico.pop_front();
        }
        d.campioni += 1;
        drop(d);
        self.registra_csv(&l);
        self.dentro.lock().unwrap().ultimo = Some(l);
    }

    fn errore(&self, e: String) {
        let mut d = self.dentro.lock().unwrap();
        if d.errore.is_none() {
            self.evento(&mut d, "critico", format!("Comunicazione con l'UPS persa: {e}"));
        }
        d.errore = Some(e);
        d.ultimo = None;
    }

    /// Ciclo di lettura: è l'unico thread che tocca l'HID.
    pub fn ciclo(&self) {
        let mut ups: Option<Ups> = None;
        loop {
            if ups.is_none() {
                match Ups::trova() {
                    Ok(u) => ups = Some(u),
                    Err(e) => self.errore(e),
                }
            }
            if let Some(u) = &ups {
                let grezzi = u.leggi_tutto();
                if grezzi.is_empty() {
                    self.errore("L'UPS non risponde".into());
                    ups = None;
                } else {
                    self.elabora(normalizza(grezzi));
                }
            }
            std::thread::sleep(Duration::from_secs_f64(self.intervallo));
        }
    }

    // --- API ------------------------------------------------------------------
    pub fn stato_json(&self) -> Value {
        let d = self.dentro.lock().unwrap();
        let in_corso = d.interruzione.as_ref().filter(|i| !i.recuperata);
        json!({
            "dati": d.ultimo,
            "errore": d.errore,
            "eventi": d.eventi.iter().rev().take(50).collect::<Vec<_>>(),
            "interruzione_in_corso": in_corso,
            "server": {"avvio": self.avvio, "campioni": d.campioni, "intervallo": self.intervallo},
            "spegnimento": self.spegnimento,
        })
    }

    pub fn storico_json(&self, dal: f64) -> Value {
        let d = self.dentro.lock().unwrap();
        json!(d.storico.iter().filter(|p| p.t > dal).collect::<Vec<_>>())
    }

    pub fn interruzioni_json(&self) -> Value {
        let righe = leggi_csv(&self.file("interruzioni.csv"));
        let lista: Vec<Value> = righe.into_iter().map(|r| {
            let mut o = serde_json::Map::new();
            for (k, v) in r {
                let val = match k.as_str() {
                    "inizio" | "fine" | "durata" | "causa" | "chiusura" => json!(v),
                    _ => json!(leggi_num(&v)),
                };
                o.insert(k, val);
            }
            Value::Object(o)
        }).collect();
        json!(lista)
    }

    pub fn interruzioni_csv(&self) -> Vec<u8> {
        fs::read(self.file("interruzioni.csv")).unwrap_or_else(|_| {
            format!("\u{feff}{}\r\n", CAMPI_INTERRUZIONE.join(";")).into_bytes()
        })
    }
}
