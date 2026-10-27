# UPS Monitor

Dashboard e storico per l'UPS **APC Back-UPS CS 650** collegato via USB (funziona con
qualunque UPS che si presenta come "HID Power Device").

Un unico eseguibile Rust, senza finestra, che:

- legge **tutti** i valori esposti dall'UPS via HID (96 su questo modello, compresi quelli
  proprietari APC) usando direttamente `hid.dll` di Windows
- registra un campione ogni 2 secondi
- tiene il **registro permanente delle interruzioni di rete**: inizio, fine, durata, causa,
  carica minima, carico massimo
- serve una dashboard scura con grafici su `http://localhost:8770`, protetta da password
- **spegne il PC durante un blackout** come faceva PowerChute: dopo 5 minuti a batteria o
  con meno di 3 minuti di autonomia, con 60 s di preavviso (annullato se la rete torna)

## Installazione

1. `cargo build --release` e copia `target\release\ups-monitor.exe` in una cartella a scelta
2. Imposta la password: `.\ups-monitor.exe --imposta-password` (legge una riga da tastiera)
3. Collegamento con argomento `--apri` sul desktop e collegamento senza argomenti in
   `shell:startup` per l'avvio automatico

## Uso

```powershell
cd <cartella-di-ups-monitor>
.\ups-monitor.exe --apri          # avvia e apre la dashboard
.\ups-monitor.exe --dump          # stampa tutti i valori grezzi dell'UPS
.\ups-monitor.exe --imposta-password   # cambia la password (chiude tutte le sessioni)
.\ups-monitor.exe --aiuto
```

Opzioni: `--porta N` (predefinita 8770), `--intervallo S`, `--dati CARTELLA`,
`--lan` (accetta connessioni dalla rete locale), `--spegni-dopo MIN` (predefinito 5, `0` = solo
a batteria scarica), `--spegni-autonomia MIN` (predefinito 3).

Il collegamento sul desktop lancia `ups-monitor.exe --apri`: se il monitor è già attivo
apre solo la dashboard.

### Spegnimento in blackout

Il monitor lancia `shutdown /s /t 60` (non servono privilegi di amministratore) quando, a
batteria, scatta la prima di queste condizioni: tempo a batteria ≥ `--spegni-dopo`,
autonomia stimata < `--spegni-autonomia`, oppure l'UPS segnala batteria scarica. Se la rete
torna durante il preavviso esegue `shutdown /a`. Nel registro l'interruzione viene chiusa
al riavvio con la nota "PC spento dal monitor".

Come riserva conviene impostare anche Windows (Opzioni risparmio energia → Batteria):
azione "Arresta il sistema" al livello critico (es. 25%), dato che l'ibernazione
spesso non è disponibile con WSL/Hyper-V attivi.

Per fermarlo: `Stop-Process -Name ups-monitor`.

Avvio automatico: collegamento `UPS Monitor.lnk` nella cartella Esecuzione automatica
(`shell:startup`). Parte all'accesso dell'utente, senza finestra.

## File in `log\`

| File | Contenuto |
|---|---|
| `interruzioni.csv` | **una riga per blackout**, mai cancellato. Si apre con Excel (`;`, virgola decimale) |
| `ups_AAAA-MM-GG.csv` | un campione ogni 2 s: tensioni, carico, carica, autonomia, temperatura |
| `eventi.jsonl` | eventi: rete persa e tornata, batteria bassa, sovraccarico, ... |
| `interruzione_in_corso.json` | blackout aperto; se il PC si spegne durante il blackout, al riavvio viene chiuso e segnato come "fine stimata" |
| `config.json` | hash PBKDF2-SHA256 della password (mai in chiaro) |
| `sessioni.json` | hash dei token di sessione attivi |

Dalla dashboard: **Esporta CSV** scarica `interruzioni.csv`.

### Colonne di `interruzioni.csv`

`inizio`, `fine`, `durata_s`, `durata` (leggibile), `causa` (dal codice APC: mancanza di rete,
brownout, picco, autotest...), `carica_inizio_pct`, `carica_min_pct`, `tensione_batteria_min`,
`carico_max_w`, `tensione_ingresso_prima`, `chiusura` (rete tornata / micro-interruzione /
fine stimata).

Le **micro-interruzioni** più brevi di una lettura (< 2 s) vengono comunque registrate:
l'UPS aggiorna il codice "causa dell'ultimo trasferimento" e il monitor se ne accorge.

## Sicurezza

- password unica salvata come PBKDF2-SHA256 (300.000 iterazioni) con salt casuale
- cookie di sessione `HttpOnly`, `SameSite=Strict`, valido 30 giorni (`Secure` dietro HTTPS)
- 5 password errate dallo stesso indirizzo = blocco di 15 minuti
- di default ascolta solo su `127.0.0.1`; per l'accesso remoto usare un **Cloudflare Tunnel**
  (HTTPS, nessuna porta aperta sul router). Dietro il tunnel l'IP reale del client viene
  preso da `CF-Connecting-IP`

## Note sui dati APC

Il significato di **tutti i codici HID** (standard e proprietari APC `0xFF86`) è spiegato in
[docs/CODICI_HID_APC.md](docs/CODICI_HID_APC.md), con la versione Excel filtrabile
[docs/APC_Back-UPS_CS_codici_HID.xlsx](docs/APC_Back-UPS_CS_codici_HID.xlsx).

- La **data batteria** è quella scritta nel firmware dell'UPS (03/04/2008 su questo modello):
  APC non la aggiorna da sola quando si cambia la batteria
- La tensione di uscita (230 V) sul Back-UPS CS è il valore nominale, non una misura
- La potenza in watt è calcolata: carico % × 400 W nominali
- Codici proprietari APC (pagina 0xFF86) decodificati come nel progetto NUT:
  `0x52` causa ultimo trasferimento, `0x61` sensibilità, `0x16` data sostituzione batteria

## Compilazione

```powershell
cargo build --release
Copy-Item target\release\ups-monitor.exe . -Force
```

La dashboard (`dashboard.html`) è incorporata nell'eseguibile (`include_str!`): va ricompilato
dopo ogni modifica. Chart.js viene caricato da cdnjs.
