# Sostituzione della batteria – APC Back-UPS CS 650

## Quale batteria comprare

Prima controlla l'etichetta sotto l'UPS: il modello europeo a 230 V è **BK650EI**.

| Scelta | Cosa cercare | Prezzo indicativo |
|---|---|---|
| **Originale APC** | **APC RBC17** (cartuccia di ricambio ufficiale) | 50–80 € |
| **Equivalente** (va benissimo) | batteria al piombo **AGM/VRLA 12 V 9 Ah**, attacchi **faston F2 (6,3 mm)**, misure circa **151 × 65 × 94 mm** | 20–35 € |

Marche affidabili per l'equivalente: Yuasa, CSB, FIAMM, Panasonic, Ultracell.
Una sigla tipica è "12V 9Ah F2" oppure "HR1234W" (la "W" indica la versione per UPS, adatta
alle scariche brevi e intense). **Non** prendere quelle con attacchi F1 (4,8 mm): non entrano
nei faston dell'UPS.

## Dove comprarla

- negozi di batterie e ricambi (anche quelli per auto/moto spesso le hanno: è un formato standard
  usato anche negli antifurto)
- negozi di elettronica e materiale elettrico
- online: Amazon, ePRICE, siti specializzati in batterie per UPS
- per l'originale RBC17: rivenditori APC / Schneider Electric

## Come sostituirla

1. Il Back-UPS CS ha la batteria **sostituibile dall'utente**: si può fare anche con l'UPS acceso,
   ma è più tranquillo **spegnere il PC, spegnere l'UPS e staccarlo dalla presa**
2. Apri lo sportello del vano batteria (sotto o sul retro, a seconda della revisione)
3. Sfila la batteria, scollega i due faston: **rosso = +**, **nero = −**
4. Collega la nuova rispettando i colori, reinseriscila e chiudi lo sportello
5. Ricollega l'UPS alla rete e **lascialo caricare almeno 8 ore** prima di contare sull'autonomia piena

## Dopo la sostituzione

- In dashboard l'indicatore "Sostituire batteria" deve restare spento e l'autonomia tornare piena
- La **data batteria** nel firmware dell'UPS non si aggiorna da sola (ora dice 03/04/2008):
  si può riscrivere via USB come fatto per le soglie di tensione
- La vecchia batteria contiene piombo e acido: **non va nei rifiuti**. Riportala al negozio
  (è obbligato a ritirarla) o all'isola ecologica

## Quando cambiarla

Vita tipica 3–5 anni. Segnali che è ora:
- l'indicatore "Sostituire batteria" si accende o l'autotest non dà "Superato"
- durante un blackout la carica scende molto più in fretta di quanto prometteva l'autonomia
- l'UPS bippa e si spegne subito appena manca la corrente
- la batteria è gonfia o calda
