# Codici HID dell'APC Back-UPS CS 650 (BK650EI) spiegati

Cosa significano tutti i valori che un **APC Back-UPS CS 650** espone via USB come *HID Power Device*, compresi i codici **proprietari APC** (pagina `0xFF86`) che APC non ha mai documentato.

Valori di esempio letti con [`ups-monitor --dump`](../README.md) da un'unità con firmware `817.v2.I USB FW:v2`. Gli stessi codici compaiono anche con NUT (`usbhid-ups`), PowerChute e altri strumenti HID. È disponibile anche la versione Excel filtrabile: [APC_Back-UPS_CS_codici_HID.xlsx](APC_Back-UPS_CS_codici_HID.xlsx).

## Come leggere i codici

- **Struttura**: `UPS.Collezione.Usage`. Ad esempio `UPS.Battery.Voltage` è la tensione (`Voltage`) nella sezione batteria (`Battery`)
- **Standard HID**: pagine `0x84` e `0x85` della specifica USB *HID Power Device*, con significato documentato
- **`0xFF86:0x....`**: codici proprietari APC. I significati vengono dal reverse engineering del progetto [NUT](https://github.com/networkupstools/nut/blob/master/drivers/apc-hid.c) e dal confronto con i valori letti
- **Copie**: molti valori compaiono sia in `UPS.Battery` / `UPS.PresentStatus` sia in `UPS.PowerSummary`. `PowerSummary` è il riepilogo che Windows legge per l'icona della batteria
- **Unità**: le tensioni sono già in volt. La temperatura è in **kelvin**. Le date usano due formati diversi:
  - formato HID: `(anno-1980)×512 + mese×32 + giorno` (12906 = 10/03/2005)
  - formato APC: cifre esadecimali `MMGGAA` (262920 = `0x040308` = 03/04/2008)

Certezza: ✅ documentato o confermato dai valori · 🟡 da NUT, coerente ma non ufficiale · ⚪ non documentato da nessuno

## In breve: i codici proprietari APC

| Codice | Significato | Certezza |
|---|---|---|
| `UPS.0xFF86:0x0079` | Revisione firmware/USB interna APC | 🟡 probabile |
| `UPS.Input.0xFF86:0x0061` | Sensibilità ai disturbi di rete | 🟡 probabile |
| `UPS.Input.0xFF86:0x0052` | Causa dell'ultimo passaggio a batteria | 🟡 probabile |
| `UPS.Battery.0xFF86:0x0016` | Data di sostituzione batteria (formato APC) | ✅ certo |
| `UPS.Battery.0xFF86:0x0018` | Dato interno APC della batteria | ⚪ sconosciuto |
| `UPS.Battery.0xFF86:0x0019` | Dato interno APC della batteria | ⚪ sconosciuto |
| `UPS.Battery.0xFF86:0x001A` | Dato interno APC della batteria | ⚪ sconosciuto |
| `UPS.Battery.0xFF86:0x001B` | Dato interno APC della batteria | ⚪ sconosciuto |
| `UPS.Battery.0xFF86:0x0024` | Costante interna APC | ⚪ sconosciuto |
| `UPS.Output.0xFF86:0x007D` | Ritardo di spegnimento in versione APC | 🟡 probabile |
| `UPS.Output.0xFF86:0x007E` | Ritardo di riavvio in versione APC | 🟡 probabile |
| `UPS.Output.0xFF86:0x007C` | Comando APC di riavvio/spegnimento forzato | 🟡 probabile |
| `UPS.Output.0xFF86:0x0076` | Comando APC 'spegni dopo il ritardo' | 🟡 probabile |
| `UPS.0xFF86:0x0072` | Test del pannello (LED e cicalino) | 🟡 probabile |
| `UPS.PresentStatus.0xFF86:0x0080` | Indicatore di stato aggiuntivo APC | ⚪ sconosciuto |
| `UPS.PowerSummary.PresentStatus.0xFF86:0x0080` | Indicatore di stato aggiuntivo APC (copia) | ⚪ sconosciuto |
| `UPS.0xFF86:0x0001.0xFF86:0x0023` | Collezione generale APC: campo interno | ⚪ sconosciuto |
| `UPS.0xFF86:0x0001.0xFF86:0x0025` | Collezione generale APC: campo interno | ⚪ sconosciuto |
| `UPS.0xFF86:0x0001.0xFF86:0x0026` | Collezione generale APC: campo interno | ⚪ sconosciuto |
| `UPS.0xFF86:0x0027` | Campo interno APC | ⚪ sconosciuto |
| `UPS.0xFF86:0x0028` | Campo interno APC | ⚪ sconosciuto |
| `UPS.0xFF86:0x0042` | Campo interno APC | ⚪ sconosciuto |
| `UPS.Input.0xFF86:0x0024` | Costante interna APC | ⚪ sconosciuto |
| `UPS.PowerConverter.0xFF86:0x0024` | Costante interna APC | ⚪ sconosciuto |

## Identità

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.iManufacturer` | American Power Conversion |  | Produttore | ✅ certo |  |
| `UPS.iName` | APC |  | Nome breve del produttore | ✅ certo |  |
| `UPS.iProduct` | Back-UPS CS 650 |  | Modello | ✅ certo |  |
| `UPS.iSerialNumber` | (numero di serie) |  | Numero di serie | ✅ certo | Nei seriali APC di questa epoca le cifre dopo le due lettere indicano anno e settimana di produzione (es. ..0511.. = 2005, settimana 11) |
| `UPS.ManufactureDate` | 12906 | 10/03/2005 | Data di produzione dell'UPS | ✅ certo | Formato HID: (anno-1980)×512 + mese×32 + giorno |
| `UPS.PowerSummary.iProduct` | Back-UPS CS 650 FW:817.v2.I USB FW:v2 |  | Modello completo con versioni firmware | ✅ certo | Firmware UPS 817.v2.I, firmware USB v2 (non aggiornabili) |
| `UPS.PowerSummary.iManufacturer` | American Power Conversion |  | Produttore (copia nel riepilogo) | ✅ certo |  |
| `UPS.PowerSummary.iOEMInformation` | American Power Conversion |  | Informazioni OEM | ✅ certo |  |
| `UPS.PowerSummary.iSerialNumber` | (numero di serie) |  | Numero di serie (copia nel riepilogo) | ✅ certo |  |
| `UPS.PowerSummary.ManufactureDate` | 12906 | 10/03/2005 | Data di produzione (copia nel riepilogo) | ✅ certo |  |
| `UPS.0xFF86:0x0079` | 8 |  | Revisione firmware/USB interna APC | 🟡 probabile | NUT la chiama APC_USB_FirmwareRevision; il valore 8 non corrisponde in modo evidente a 'v2' |

## Rete elettrica e uscita

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.Input.Voltage` | 244 | 244 V | Tensione di rete misurata in ingresso | ✅ certo | Rete italiana: 230 V ±10% (207–253 V) |
| `UPS.Input.ConfigVoltage` | 230 | 230 V | Tensione di rete nominale | ✅ certo |  |
| `UPS.Output.Voltage` | 230 | 230 V | Tensione di uscita | ✅ certo | Sul Back-UPS CS è il valore nominale, non una misura |
| `UPS.Output.ConfigVoltage` | 230 | 230 V | Tensione di uscita nominale | ✅ certo |  |
| `UPS.Output.Frequency` | 50 | 50 Hz | Frequenza di uscita | ✅ certo |  |
| `UPS.Output.PercentLoad` | 42 | 42 % | Carico attuale in percentuale della potenza massima | ✅ certo |  |
| `UPS.Output.ConfigActivePower` | 400 | 400 W | Potenza massima erogabile | ✅ certo | 650 VA = 400 W reali |
| `(calcolato)` | 168 | 168 W | Potenza assorbita stimata = carico % × 400 W | ✅ certo | Non è un codice dell'UPS: è ricavato dalle due righe sopra |
| `UPS.Output.LowVoltageTransfer` | 180 | 180 V | Sotto questa tensione l'UPS passa a batteria | ✅ certo | Scrivibile: l'UPS accetta ogni valore intero 160–196 V (verificato scrivendo e rileggendo). Di fabbrica 196 |
| `UPS.Output.HighVoltageTransfer` | 256 | 256 V | Sopra questa tensione l'UPS passa a batteria | ✅ certo | Scrivibile: l'UPS accetta ogni valore intero 256–278 V (verificato scrivendo e rileggendo). Di fabbrica 256. Con hardware datato conviene restare a 256: a 264 V i condensatori degli alimentatori arrivano a circa 373 V di picco |
| `UPS.Input.0xFF86:0x0061` | 1 | Media | Sensibilità ai disturbi di rete | 🟡 probabile | Range dichiarato 0–2. Alta = interviene anche per piccole distorsioni |
| `UPS.Input.0xFF86:0x0052` | 8 | Autotest dell'UPS | Causa dell'ultimo passaggio a batteria | 🟡 probabile | Decodifica dal progetto NUT. Il monitor la usa per le micro-interruzioni |

## Batteria

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.Battery.Voltage` | 13,59 | 13,59 V | Tensione attuale della batteria | ✅ certo | 13,5–13,8 V = mantenimento a piena carica. A batteria scende verso 11–12 V |
| `UPS.Battery.ConfigVoltage` | 12 | 12 V | Tensione nominale della batteria | ✅ certo |  |
| `UPS.Battery.RemainingCapacity` | 100 | 100 % | Carica residua | ✅ certo |  |
| `UPS.Battery.RunTimeToEmpty` | 727 | 12 min 7 s | Autonomia stimata al carico attuale | ✅ certo | Stima dell'UPS, non una misura. Cambia con il carico |
| `UPS.Battery.RemainingTimeLimit` | 120 | 2 min | Soglia di autonomia minima | ✅ certo | Sotto questo valore si accende RemainingTimeLimitExpired |
| `UPS.Battery.Temperature` | 302,3 | 29,2 °C | Temperatura interna (batteria) | ✅ certo | Espressa in kelvin: °C = K − 273,15. Ideale sotto i 30 °C |
| `UPS.Battery.ManufactureDate` | 14467 | 03/04/2008 | Data della batteria (formato HID) | ✅ certo | Non si aggiorna da sola quando si cambia la batteria |
| `UPS.Battery.0xFF86:0x0016` | 262920 | 03/04/2008 | Data di sostituzione batteria (formato APC) | ✅ certo | Esadecimale 04 03 08 = mese 04, giorno 03, anno 08. Coincide con la riga sopra |
| `UPS.Battery.Test` | 6 | Nessun test eseguito | Esito dell'ultimo autotest | ✅ certo | Scrivendo 1 si avvia un autotest rapido |
| `UPS.Battery.0xFF86:0x0018` | 2114885126 | esadecimale 7E0E9606 | Dato interno APC della batteria | ⚪ sconosciuto | Non documentato nemmeno da NUT |
| `UPS.Battery.0xFF86:0x0019` | 0 |  | Dato interno APC della batteria | ⚪ sconosciuto |  |
| `UPS.Battery.0xFF86:0x001A` | 0 |  | Dato interno APC della batteria | ⚪ sconosciuto |  |
| `UPS.Battery.0xFF86:0x001B` | 0 |  | Dato interno APC della batteria | ⚪ sconosciuto |  |
| `UPS.Battery.0xFF86:0x0024` | 243 |  | Costante interna APC | ⚪ sconosciuto | Lo stesso usage compare in Input (126) e PowerConverter (150): forse costanti di taratura |

## Riepilogo (letto da Windows per l'icona batteria)

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.PowerSummary.Voltage` | 13,59 | 13,59 V | Tensione batteria (copia) | ✅ certo |  |
| `UPS.PowerSummary.ConfigVoltage` | 12 | 12 V | Tensione nominale batteria (copia) | ✅ certo |  |
| `UPS.PowerSummary.RemainingCapacity` | 100 | 100 % | Carica residua (copia) | ✅ certo | È il valore che Windows mostra nell'icona batteria |
| `UPS.PowerSummary.RunTimeToEmpty` | 727 | 12 min 7 s | Autonomia stimata (copia) | ✅ certo |  |
| `UPS.PowerSummary.RemainingTimeLimit` | 120 | 2 min | Soglia di autonomia minima (copia) | ✅ certo |  |
| `UPS.PowerSummary.RemainingCapacityLimit` | 10 | 10 % | Soglia 'batteria bassa' | ✅ certo | Sotto il 10% si accende BelowRemainingCapacityLimit |
| `UPS.PowerSummary.WarningCapacityLimit` | 50 | 50 % | Soglia di avviso | ✅ certo |  |
| `UPS.PowerSummary.DesignCapacity` | 100 | 100 % | Capacità di progetto | ✅ certo |  |
| `UPS.PowerSummary.FullChargeCapacity` | 100 | 100 % | Capacità a piena carica | ✅ certo | Su una batteria usurata potrebbe scendere sotto 100; APC però la lascia quasi sempre a 100 |
| `UPS.PowerSummary.CapacityMode` | 2 | in percentuale | Unità delle capacità | ✅ certo | 2 = valori in % |
| `UPS.PowerSummary.CapacityGranularity1` | 1 | 1 % | Risoluzione della misura di carica | ✅ certo |  |
| `UPS.PowerSummary.CapacityGranularity2` | 1 | 1 % | Risoluzione della misura di carica | ✅ certo |  |
| `UPS.PowerSummary.Rechargeable` | 1 | SÌ (attivo) | Batteria ricaricabile | ✅ certo |  |
| `UPS.PowerSummary.iDeviceChemistry` | PbAc | piombo-acido | Chimica della batteria | ✅ certo | Batteria al piombo sigillata (AGM/VRLA) |

## Allarme, comandi e ritardi

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.AudibleAlarmControl` | 1 | Disattivato | Cicalino di allarme | ✅ certo | Codifica standard HID: 1 disattivato, 2 attivo, 3 silenziato. Verificare alla prova di blackout se bippa |
| `UPS.PowerSummary.AudibleAlarmControl` | 1 | Disattivato | Cicalino di allarme (copia) | ✅ certo |  |
| `UPS.Output.DelayBeforeShutdown` | -1 | nessuno programmato | Secondi prima che l'UPS tolga corrente alle uscite | ✅ certo | −1 = nessuno spegnimento programmato. Scrivendo un valore l'UPS si spegne dopo quel tempo |
| `UPS.Output.DelayBeforeStartup` | 0 | 0 s | Ritardo di riaccensione delle uscite | ✅ certo |  |
| `UPS.Output.DelayBeforeReboot` | 0 | 0 s | Ritardo per lo spegni-e-riaccendi | ✅ certo |  |
| `UPS.PowerSummary.DelayBeforeShutdown` | -1 | nessuno programmato | Ritardo di spegnimento (copia) | ✅ certo |  |
| `UPS.PowerSummary.DelayBeforeReboot` | 0 | 0 s | Ritardo spegni-e-riaccendi (copia) | ✅ certo |  |
| `UPS.Output.0xFF86:0x007D` | -1 | nessuno programmato | Ritardo di spegnimento in versione APC | 🟡 probabile | NUT: APCDelayBeforeShutdown. −1 come la versione standard |
| `UPS.Output.0xFF86:0x007E` | 0 | 0 s | Ritardo di riavvio in versione APC | 🟡 probabile | NUT: APCDelayBeforeStartup |
| `UPS.Output.0xFF86:0x007C` | 0 |  | Comando APC di riavvio/spegnimento forzato | 🟡 probabile | NUT lo associa ai comandi di spegnimento forzato |
| `UPS.Output.0xFF86:0x0076` | 0 |  | Comando APC 'spegni dopo il ritardo' | 🟡 probabile | NUT: APCShutdownAfterDelay |
| `UPS.0xFF86:0x0072` | 0 |  | Test del pannello (LED e cicalino) | 🟡 probabile | NUT: APCPanelTest. Scrivendo 1 l'UPS fa il test di spie e cicalino |

## Indicatori di stato (1 = attivo)

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.PresentStatus.ACPresent` | 1 | SÌ (attivo) | Rete elettrica presente | ✅ certo | 1 = alimentato dalla rete, 0 = a batteria (blackout) |
| `UPS.PresentStatus.BatteryPresent` | 1 | SÌ (attivo) | Batteria presente | ✅ certo | 0 = batteria scollegata o guasta |
| `UPS.PresentStatus.BelowRemainingCapacityLimit` | 0 | no | Batteria bassa | ✅ certo | Carica scesa sotto RemainingCapacityLimit (10%) |
| `UPS.PresentStatus.Charging` | 0 | no | Batteria in carica | ✅ certo | Resta a 0 quando la batteria è già piena |
| `UPS.PresentStatus.CommunicationLost` | 0 | no | Comunicazione persa | ✅ certo | Problema di comunicazione interna dell'UPS |
| `UPS.PresentStatus.Discharging` | 0 | no | Batteria in scarica | ✅ certo | 1 = l'UPS sta alimentando i carichi a batteria |
| `UPS.PresentStatus.NeedReplacement` | 0 | no | Sostituire la batteria | ✅ certo | Lo imposta l'UPS dopo un autotest fallito |
| `UPS.PresentStatus.Overload` | 0 | no | Sovraccarico | ✅ certo | Carico oltre i 400 W nominali: staccare qualcosa |
| `UPS.PresentStatus.RemainingTimeLimitExpired` | 0 | no | Autonomia esaurita | ✅ certo | Autonomia stimata sotto RemainingTimeLimit (120 s) |
| `UPS.PresentStatus.ShutdownImminent` | 0 | no | Spegnimento imminente | ✅ certo | L'UPS sta per togliere corrente alle uscite |
| `UPS.PresentStatus.ShutdownRequested` | 0 | no | Spegnimento richiesto | ✅ certo | È stato inviato un comando di spegnimento all'UPS |
| `UPS.PresentStatus.VoltageNotRegulated` | 0 | no | Tensione non regolata | ✅ certo | Uscita fuori tolleranza |
| `UPS.PresentStatus.0xFF86:0x0080` | 0 | no | Indicatore di stato aggiuntivo APC | ⚪ sconosciuto | Bit di stato non documentato |
| `UPS.PowerSummary.PresentStatus.ACPresent` | 1 | SÌ (attivo) | Rete elettrica presente (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.BatteryPresent` | 1 | SÌ (attivo) | Batteria presente (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.BelowRemainingCapacityLimit` | 0 | no | Batteria bassa (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.Charging` | 0 | no | Batteria in carica (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.CommunicationLost` | 0 | no | Comunicazione persa (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.Discharging` | 0 | no | Batteria in scarica (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.NeedReplacement` | 0 | no | Sostituire la batteria (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.Overload` | 0 | no | Sovraccarico (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.RemainingTimeLimitExpired` | 0 | no | Autonomia esaurita (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.ShutdownImminent` | 0 | no | Spegnimento imminente (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.ShutdownRequested` | 0 | no | Spegnimento richiesto (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.VoltageNotRegulated` | 0 | no | Tensione non regolata (copia) | ✅ certo | Copia nel riepilogo usato da Windows |
| `UPS.PowerSummary.PresentStatus.0xFF86:0x0080` | 0 | no | Indicatore di stato aggiuntivo APC (copia) | ⚪ sconosciuto |  |

## Interni APC non documentati

| Codice | Valore | Leggibile | Significato | Certezza | Note |
|---|---|---|---|---|---|
| `UPS.0xFF86:0x0001.0xFF86:0x0023` | 0 |  | Collezione generale APC: campo interno | ⚪ sconosciuto |  |
| `UPS.0xFF86:0x0001.0xFF86:0x0025` | 50462976 | esadecimale 3020100 | Collezione generale APC: campo interno | ⚪ sconosciuto | 03020100 = byte 00 01 02 03: sembra un valore di riempimento o di servizio |
| `UPS.0xFF86:0x0001.0xFF86:0x0026` | 14 |  | Collezione generale APC: campo interno | ⚪ sconosciuto |  |
| `UPS.0xFF86:0x0027` | 0 |  | Campo interno APC | ⚪ sconosciuto |  |
| `UPS.0xFF86:0x0028` | 0 |  | Campo interno APC | ⚪ sconosciuto |  |
| `UPS.0xFF86:0x0042` | 7 |  | Campo interno APC | ⚪ sconosciuto | In NUT compare tra gli usage visti ma non decodificati |
| `UPS.Input.0xFF86:0x0024` | 126 |  | Costante interna APC | ⚪ sconosciuto | Vedi Battery.0xFF86:0x0024 |
| `UPS.PowerConverter.0xFF86:0x0024` | 150 |  | Costante interna APC | ⚪ sconosciuto | Vedi Battery.0xFF86:0x0024 |

## Scrittura

Molti valori sono *feature report* scrivibili. Verificati su questa unità scrivendo e rileggendo:

- `UPS.Output.LowVoltageTransfer`: ogni intero tra **160 e 196 V** (report ID 36)
- `UPS.Output.HighVoltageTransfer`: ogni intero tra **256 e 278 V** (report ID 35)

Non provati qui ma previsti dallo standard: `UPS.Battery.Test = 1` avvia un autotest, `UPS.Output.DelayBeforeShutdown = N` spegne le uscite dopo N secondi.

Correzioni e altri valori decodificati sono benvenuti: apri una issue.
