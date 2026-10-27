//! Lettura dell'UPS via HID usando direttamente le API di Windows.
//!
//! Il descrittore HID viene analizzato da Windows stesso (funzioni HidP_*):
//! leggiamo TUTTI i valori dei feature report senza tabelle cablate a mano.
//! Le chiavi hanno la forma "UPS.Input.Voltage" (percorso delle collezioni + usage).

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ptr::null_mut;

use serde::Serialize;

pub const VID_APC: u16 = 0x051D;
const PAGINA_POWER_DEVICE: u16 = 0x84;

// --- FFI ----------------------------------------------------------------------
type Handle = *mut c_void;
type Preparsed = isize;
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;
const FILE_SHARE_RW: u32 = 0x1 | 0x2;
const OPEN_EXISTING: u32 = 3;
const DIGCF_PRESENT: u32 = 0x2;
const DIGCF_DEVICEINTERFACE: u32 = 0x10;
const HIDP_FEATURE: i32 = 2;
const HIDP_STATUS_SUCCESS: u32 = 0x0011_0000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Guid {
    d1: u32,
    d2: u16,
    d3: u16,
    d4: [u8; 8],
}

#[repr(C)]
struct SpDeviceInterfaceData {
    cb_size: u32,
    guid: Guid,
    flags: u32,
    reserved: usize,
}

#[repr(C)]
#[derive(Default)]
struct HiddAttributes {
    size: u32,
    vendor_id: u16,
    product_id: u16,
    version: u16,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct HidpCaps {
    usage: u16,
    usage_page: u16,
    input_len: u16,
    output_len: u16,
    feature_len: u16,
    reserved: [u16; 17],
    n_link_nodes: u16,
    n_input_button_caps: u16,
    n_input_value_caps: u16,
    n_input_data_indices: u16,
    n_output_button_caps: u16,
    n_output_value_caps: u16,
    n_output_data_indices: u16,
    n_feature_button_caps: u16,
    n_feature_value_caps: u16,
    n_feature_data_indices: u16,
}

/// Parte variabile di HIDP_VALUE_CAPS / HIDP_BUTTON_CAPS (Range o NotRange).
/// Con IsRange: [UsageMin, UsageMax, ...]; altrimenti [Usage, Reserved1, ...].
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct RangeUnion {
    a: u16,
    b: u16,
    resto: [u16; 6],
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct HidpValueCaps {
    usage_page: u16,
    report_id: u8,
    is_alias: u8,
    bit_field: u16,
    link_collection: u16,
    link_usage: u16,
    link_usage_page: u16,
    is_range: u8,
    is_string_range: u8,
    is_designator_range: u8,
    is_absolute: u8,
    has_null: u8,
    reserved: u8,
    bit_size: u16,
    report_count: u16,
    reserved2: [u16; 5],
    units_exp: u32,
    units: u32,
    logical_min: i32,
    logical_max: i32,
    physical_min: i32,
    physical_max: i32,
    u: RangeUnion,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct HidpButtonCaps {
    usage_page: u16,
    report_id: u8,
    is_alias: u8,
    bit_field: u16,
    link_collection: u16,
    link_usage: u16,
    link_usage_page: u16,
    is_range: u8,
    is_string_range: u8,
    is_designator_range: u8,
    is_absolute: u8,
    report_count: u16,
    reserved2: u16,
    reserved: [u32; 9],
    u: RangeUnion,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HidpLinkCollectionNode {
    link_usage: u16,
    link_usage_page: u16,
    parent: u16,
    n_children: u16,
    next_sibling: u16,
    first_child: u16,
    bits: u32,
    user_context: *mut c_void,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(name: *const u16, access: u32, share: u32, sec: *mut c_void,
                   disp: u32, flags: u32, template: Handle) -> Handle;
    fn CloseHandle(h: Handle) -> i32;
}

#[link(name = "setupapi")]
extern "system" {
    fn SetupDiGetClassDevsW(guid: *const Guid, enumerator: *const u16, parent: *mut c_void,
                            flags: u32) -> Handle;
    fn SetupDiEnumDeviceInterfaces(set: Handle, info: *mut c_void, guid: *const Guid,
                                   index: u32, data: *mut SpDeviceInterfaceData) -> i32;
    fn SetupDiGetDeviceInterfaceDetailW(set: Handle, data: *mut SpDeviceInterfaceData,
                                        detail: *mut c_void, size: u32, required: *mut u32,
                                        info: *mut c_void) -> i32;
    fn SetupDiDestroyDeviceInfoList(set: Handle) -> i32;
}

#[link(name = "hid")]
extern "system" {
    fn HidD_GetHidGuid(guid: *mut Guid);
    fn HidD_GetAttributes(h: Handle, attr: *mut HiddAttributes) -> u8;
    fn HidD_GetPreparsedData(h: Handle, pp: *mut Preparsed) -> u8;
    fn HidD_FreePreparsedData(pp: Preparsed) -> u8;
    fn HidD_GetFeature(h: Handle, buf: *mut c_void, len: u32) -> u8;
    fn HidD_GetIndexedString(h: Handle, idx: u32, buf: *mut c_void, len: u32) -> u8;
    fn HidD_GetProductString(h: Handle, buf: *mut c_void, len: u32) -> u8;
    fn HidP_GetCaps(pp: Preparsed, caps: *mut HidpCaps) -> u32;
    fn HidP_GetValueCaps(tipo: i32, caps: *mut HidpValueCaps, len: *mut u16, pp: Preparsed) -> u32;
    fn HidP_GetButtonCaps(tipo: i32, caps: *mut HidpButtonCaps, len: *mut u16, pp: Preparsed) -> u32;
    fn HidP_GetLinkCollectionNodes(nodes: *mut HidpLinkCollectionNode, len: *mut u32,
                                   pp: Preparsed) -> u32;
    fn HidP_GetUsageValue(tipo: i32, page: u16, coll: u16, usage: u16, val: *mut u32,
                          pp: Preparsed, report: *mut u8, len: u32) -> u32;
    fn HidP_GetUsages(tipo: i32, page: u16, coll: u16, list: *mut u16, len: *mut u32,
                      pp: Preparsed, report: *mut u8, rlen: u32) -> u32;
}

// --- Nomi degli usage (HID Power Device: pagine 0x84 e 0x85) -----------------
fn nome_noto(page: u16, usage: u16) -> Option<&'static str> {
    Some(match (page, usage) {
        (0x84, 0x01) => "iName", (0x84, 0x02) => "PresentStatus", (0x84, 0x03) => "ChangedStatus",
        (0x84, 0x04) => "UPS", (0x84, 0x05) => "PowerSupply", (0x84, 0x10) => "BatterySystem",
        (0x84, 0x11) => "BatterySystemID", (0x84, 0x12) => "Battery", (0x84, 0x13) => "BatteryID",
        (0x84, 0x14) => "Charger", (0x84, 0x15) => "ChargerID", (0x84, 0x16) => "PowerConverter",
        (0x84, 0x17) => "PowerConverterID", (0x84, 0x18) => "OutletSystem",
        (0x84, 0x19) => "OutletSystemID", (0x84, 0x1A) => "Input", (0x84, 0x1B) => "InputID",
        (0x84, 0x1C) => "Output", (0x84, 0x1D) => "OutputID", (0x84, 0x1E) => "Flow",
        (0x84, 0x1F) => "FlowID", (0x84, 0x20) => "Outlet", (0x84, 0x21) => "OutletID",
        (0x84, 0x24) => "PowerSummary", (0x84, 0x25) => "PowerSummaryID",
        (0x84, 0x30) => "Voltage", (0x84, 0x31) => "Current", (0x84, 0x32) => "Frequency",
        (0x84, 0x33) => "ApparentPower", (0x84, 0x34) => "ActivePower",
        (0x84, 0x35) => "PercentLoad", (0x84, 0x36) => "Temperature", (0x84, 0x37) => "Humidity",
        (0x84, 0x38) => "BadCount", (0x84, 0x40) => "ConfigVoltage", (0x84, 0x41) => "ConfigCurrent",
        (0x84, 0x42) => "ConfigFrequency", (0x84, 0x43) => "ConfigApparentPower",
        (0x84, 0x44) => "ConfigActivePower", (0x84, 0x45) => "ConfigPercentLoad",
        (0x84, 0x46) => "ConfigTemperature", (0x84, 0x47) => "ConfigHumidity",
        (0x84, 0x50) => "SwitchOnControl", (0x84, 0x51) => "SwitchOffControl",
        (0x84, 0x52) => "ToggleControl", (0x84, 0x53) => "LowVoltageTransfer",
        (0x84, 0x54) => "HighVoltageTransfer", (0x84, 0x55) => "DelayBeforeReboot",
        (0x84, 0x56) => "DelayBeforeStartup", (0x84, 0x57) => "DelayBeforeShutdown",
        (0x84, 0x58) => "Test", (0x84, 0x59) => "ModuleReset", (0x84, 0x5A) => "AudibleAlarmControl",
        (0x84, 0x60) => "Present", (0x84, 0x61) => "Good", (0x84, 0x62) => "InternalFailure",
        (0x84, 0x63) => "VoltageOutOfRange", (0x84, 0x64) => "FrequencyOutOfRange",
        (0x84, 0x65) => "Overload", (0x84, 0x66) => "OverCharged", (0x84, 0x67) => "OverTemperature",
        (0x84, 0x68) => "ShutdownRequested", (0x84, 0x69) => "ShutdownImminent",
        (0x84, 0x6B) => "SwitchOn/Off", (0x84, 0x6C) => "Switchable", (0x84, 0x6D) => "Used",
        (0x84, 0x6E) => "Boost", (0x84, 0x6F) => "Buck", (0x84, 0x70) => "Initialized",
        (0x84, 0x71) => "Tested", (0x84, 0x72) => "AwaitingPower", (0x84, 0x73) => "CommunicationLost",
        (0x84, 0xFD) => "iManufacturer", (0x84, 0xFE) => "iProduct", (0x84, 0xFF) => "iSerialNumber",
        (0x85, 0x01) => "SMBBatteryMode", (0x85, 0x02) => "SMBBatteryStatus",
        (0x85, 0x03) => "SMBAlarmWarning", (0x85, 0x29) => "RemainingCapacityLimit",
        (0x85, 0x2A) => "RemainingTimeLimit", (0x85, 0x2B) => "AtRate", (0x85, 0x2C) => "CapacityMode",
        (0x85, 0x2D) => "BroadcastToCharger", (0x85, 0x2E) => "PrimaryBattery",
        (0x85, 0x2F) => "ChargeController", (0x85, 0x40) => "TerminateCharge",
        (0x85, 0x41) => "TerminateDischarge", (0x85, 0x42) => "BelowRemainingCapacityLimit",
        (0x85, 0x43) => "RemainingTimeLimitExpired", (0x85, 0x44) => "Charging",
        (0x85, 0x45) => "Discharging", (0x85, 0x46) => "FullyCharged", (0x85, 0x47) => "FullyDischarged",
        (0x85, 0x48) => "ConditioningFlag", (0x85, 0x49) => "AtRateOK", (0x85, 0x4A) => "SMBErrorCode",
        (0x85, 0x4B) => "NeedReplacement", (0x85, 0x60) => "AtRateTimeToFull",
        (0x85, 0x61) => "AtRateTimeToEmpty", (0x85, 0x62) => "AverageCurrent",
        (0x85, 0x63) => "MaxError", (0x85, 0x64) => "RelativeStateOfCharge",
        (0x85, 0x65) => "AbsoluteStateOfCharge", (0x85, 0x66) => "RemainingCapacity",
        (0x85, 0x67) => "FullChargeCapacity", (0x85, 0x68) => "RunTimeToEmpty",
        (0x85, 0x69) => "AverageTimeToEmpty", (0x85, 0x6A) => "AverageTimeToFull",
        (0x85, 0x6B) => "CycleCount", (0x85, 0x80) => "BattPackModelLevel",
        (0x85, 0x81) => "InternalChargeController", (0x85, 0x82) => "PrimaryBatterySupport",
        (0x85, 0x83) => "DesignCapacity", (0x85, 0x84) => "SpecificationInfo",
        (0x85, 0x85) => "ManufactureDate", (0x85, 0x86) => "SerialNumber",
        (0x85, 0x87) => "iManufacturerName", (0x85, 0x88) => "iDeviceName",
        (0x85, 0x89) => "iDeviceChemistry", (0x85, 0x8A) => "ManufacturerData",
        (0x85, 0x8B) => "Rechargeable", (0x85, 0x8C) => "WarningCapacityLimit",
        (0x85, 0x8D) => "CapacityGranularity1", (0x85, 0x8E) => "CapacityGranularity2",
        (0x85, 0x8F) => "iOEMInformation", (0x85, 0xC0) => "InhibitCharge",
        (0x85, 0xC1) => "EnablePolling", (0x85, 0xC2) => "ResetToZero", (0x85, 0xD0) => "ACPresent",
        (0x85, 0xD1) => "BatteryPresent", (0x85, 0xD2) => "PowerFail", (0x85, 0xD3) => "AlarmInhibited",
        (0x85, 0xD4) => "ThermistorUnderRange", (0x85, 0xD5) => "ThermistorHot",
        (0x85, 0xD6) => "ThermistorCold", (0x85, 0xD7) => "ThermistorOverRange",
        (0x85, 0xD8) => "VoltageOutOfRange", (0x85, 0xD9) => "CurrentOutOfRange",
        (0x85, 0xDA) => "CurrentNotRegulated", (0x85, 0xDB) => "VoltageNotRegulated",
        (0x85, 0xDC) => "MasterMode",
        _ => return None,
    })
}

pub fn nome_usage(page: u16, usage: u16) -> String {
    match nome_noto(page, usage) {
        Some(n) => n.to_string(),
        None => format!("0x{page:04X}:0x{usage:04X}"),
    }
}

/// Un valore letto: numero o stringa (per gli usage "iXxx" che puntano a stringhe USB).
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Valore {
    Num(f64),
    Testo(String),
}

impl Valore {
    pub fn num(&self) -> Option<f64> {
        match self {
            Valore::Num(n) => Some(*n),
            Valore::Testo(_) => None,
        }
    }
}

pub type Grezzi = BTreeMap<String, Valore>;

fn wide(s: &[u16]) -> String {
    let fine = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    String::from_utf16_lossy(&s[..fine]).trim().to_string()
}

fn apri(path: &[u16]) -> Option<Handle> {
    for accesso in [GENERIC_READ | GENERIC_WRITE, 0] {
        // Con accesso 0 ("query") i feature report funzionano lo stesso
        let h = unsafe {
            CreateFileW(path.as_ptr(), accesso, FILE_SHARE_RW, null_mut(), OPEN_EXISTING, 0, null_mut())
        };
        if h != INVALID_HANDLE_VALUE && !h.is_null() {
            return Some(h);
        }
    }
    None
}

struct ValoreCap {
    page: u16,
    usage: u16,
    coll: u16,
    report_id: u8,
    cap: HidpValueCaps,
}

struct BottoneCap {
    page: u16,
    umin: u16,
    umax: u16,
    coll: u16,
    report_id: u8,
}

pub struct Ups {
    handle: Handle,
    pp: Preparsed,
    caps: HidpCaps,
    nodi: Vec<HidpLinkCollectionNode>,
    valori: Vec<ValoreCap>,
    bottoni: Vec<BottoneCap>,
    pub prodotto: String,
}

impl Ups {
    /// Cerca un UPS: prima per VID APC, poi qualunque Power Device HID (pagina 0x84).
    pub fn trova() -> Result<Ups, String> {
        let mut guid = Guid::default();
        unsafe { HidD_GetHidGuid(&mut guid) };
        let set = unsafe {
            SetupDiGetClassDevsW(&guid, std::ptr::null(), null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE)
        };
        if set == INVALID_HANDLE_VALUE {
            return Err("SetupDiGetClassDevs fallita".into());
        }
        let mut trovato = None;
        let mut i = 0;
        loop {
            let mut data = SpDeviceInterfaceData {
                cb_size: std::mem::size_of::<SpDeviceInterfaceData>() as u32,
                guid: Guid::default(), flags: 0, reserved: 0,
            };
            if unsafe { SetupDiEnumDeviceInterfaces(set, null_mut(), &guid, i, &mut data) } == 0 {
                break;
            }
            i += 1;
            let mut richiesto = 0u32;
            unsafe { SetupDiGetDeviceInterfaceDetailW(set, &mut data, null_mut(), 0, &mut richiesto, null_mut()) };
            if richiesto == 0 {
                continue;
            }
            // SP_DEVICE_INTERFACE_DETAIL_DATA_W: cbSize (8 su x64) + percorso UTF-16
            let mut buf = vec![0u32; (richiesto as usize).div_ceil(4) + 1];
            buf[0] = if cfg!(target_pointer_width = "64") { 8 } else { 6 };
            if unsafe {
                SetupDiGetDeviceInterfaceDetailW(set, &mut data, buf.as_mut_ptr() as *mut c_void,
                                                 richiesto, null_mut(), null_mut())
            } == 0 {
                continue;
            }
            let path: Vec<u16> = unsafe {
                let p = (buf.as_ptr() as *const u8).add(4) as *const u16;
                let n = (richiesto as usize - 4) / 2;
                std::slice::from_raw_parts(p, n).to_vec()
            };
            let Some(h) = apri(&path) else { continue };
            let mut attr = HiddAttributes { size: std::mem::size_of::<HiddAttributes>() as u32, ..Default::default() };
            unsafe { HidD_GetAttributes(h, &mut attr) };
            match Ups::da_handle(h) {
                Ok(ups) if attr.vendor_id == VID_APC || ups.caps.usage_page == PAGINA_POWER_DEVICE => {
                    trovato = Some(ups);
                    break;
                }
                Ok(ups) => drop(ups),
                Err(_) => unsafe {
                    CloseHandle(h);
                },
            }
        }
        unsafe { SetupDiDestroyDeviceInfoList(set) };
        trovato.ok_or_else(|| "UPS non trovato: controlla il cavo USB".to_string())
    }

    fn da_handle(handle: Handle) -> Result<Ups, String> {
        let mut pp: Preparsed = 0;
        if unsafe { HidD_GetPreparsedData(handle, &mut pp) } == 0 {
            return Err("HidD_GetPreparsedData fallita".into());
        }
        let mut caps = HidpCaps::default();
        unsafe { HidP_GetCaps(pp, &mut caps) };
        let mut prodotto = [0u16; 128];
        unsafe { HidD_GetProductString(handle, prodotto.as_mut_ptr() as *mut c_void, 256) };
        let mut ups = Ups {
            handle, pp, caps, nodi: Vec::new(), valori: Vec::new(), bottoni: Vec::new(),
            prodotto: wide(&prodotto),
        };
        if caps.usage_page == PAGINA_POWER_DEVICE {
            ups.carica_struttura();
        }
        Ok(ups)
    }

    fn carica_struttura(&mut self) {
        let mut n = self.caps.n_link_nodes as u32;
        let vuoto = HidpLinkCollectionNode {
            link_usage: 0, link_usage_page: 0, parent: 0, n_children: 0, next_sibling: 0,
            first_child: 0, bits: 0, user_context: null_mut(),
        };
        self.nodi = vec![vuoto; n as usize];
        unsafe { HidP_GetLinkCollectionNodes(self.nodi.as_mut_ptr(), &mut n, self.pp) };
        self.nodi.truncate(n as usize);

        // Solo i feature report: contengono tutto lo stato dell'UPS
        let mut nv = self.caps.n_feature_value_caps;
        let mut vc = vec![HidpValueCaps::default(); nv as usize];
        if nv > 0 {
            unsafe { HidP_GetValueCaps(HIDP_FEATURE, vc.as_mut_ptr(), &mut nv, self.pp) };
        }
        for c in vc.into_iter().take(nv as usize) {
            let (a, b) = if c.is_range != 0 { (c.u.a, c.u.b) } else { (c.u.a, c.u.a) };
            for usage in a..=b {
                self.valori.push(ValoreCap { page: c.usage_page, usage, coll: c.link_collection,
                                             report_id: c.report_id, cap: c });
            }
        }
        let mut nb = self.caps.n_feature_button_caps;
        let mut bc = vec![HidpButtonCaps::default(); nb as usize];
        if nb > 0 {
            unsafe { HidP_GetButtonCaps(HIDP_FEATURE, bc.as_mut_ptr(), &mut nb, self.pp) };
        }
        for c in bc.into_iter().take(nb as usize) {
            let (a, b) = if c.is_range != 0 { (c.u.a, c.u.b) } else { (c.u.a, c.u.a) };
            self.bottoni.push(BottoneCap { page: c.usage_page, umin: a, umax: b,
                                           coll: c.link_collection, report_id: c.report_id });
        }
    }

    /// Percorso leggibile della collezione, es. "UPS.PowerSummary".
    fn percorso(&self, mut idx: u16) -> String {
        let mut parti = Vec::new();
        while (idx as usize) < self.nodi.len() {
            let n = &self.nodi[idx as usize];
            parti.push(nome_usage(n.link_usage_page, n.link_usage));
            if idx == 0 {
                break;
            }
            idx = n.parent;
        }
        parti.reverse();
        parti.join(".")
    }

    fn leggi_report(&self, id: u8) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; self.caps.feature_len.max(2) as usize];
        buf[0] = id;
        let ok = unsafe { HidD_GetFeature(self.handle, buf.as_mut_ptr() as *mut c_void, buf.len() as u32) };
        (ok != 0).then_some(buf)
    }

    fn stringa(&self, indice: u32) -> Option<String> {
        if indice == 0 {
            return None;
        }
        let mut buf = [0u16; 128];
        let ok = unsafe { HidD_GetIndexedString(self.handle, indice, buf.as_mut_ptr() as *mut c_void, 256) };
        let s = wide(&buf);
        (ok != 0 && !s.is_empty()).then_some(s)
    }

    fn scala(c: &HidpValueCaps, raw: u32) -> f64 {
        let mut v = raw as i64;
        // Valori con segno: estende il bit di segno se LogicalMin < 0
        if c.logical_min < 0 && c.bit_size > 0 && c.bit_size < 32 && v & (1 << (c.bit_size - 1)) != 0 {
            v -= 1 << c.bit_size;
        } else if c.logical_min < 0 && c.bit_size == 32 {
            v = raw as i32 as i64;
        }
        let mut esp = (c.units_exp & 0xF) as i32;
        if esp > 7 {
            esp -= 16;
        }
        // Volt e watt sono espressi in unità CGS (cm, g): 1 V = 10^7 unità,
        // quindi l'esponente dichiarato va compensato di -7
        if c.units == 0x00F0_D121 || c.units == 0xD121 {
            esp -= 7;
        }
        let r = v as f64 * 10f64.powi(esp);
        (r * 10_000.0).round() / 10_000.0
    }

    /// Legge ogni feature report e restituisce {percorso.usage: valore}.
    pub fn leggi_tutto(&self) -> Grezzi {
        let mut cache: BTreeMap<u8, Option<Vec<u8>>> = BTreeMap::new();
        let mut dati = Grezzi::new();
        for v in &self.valori {
            let buf = cache.entry(v.report_id).or_insert_with(|| self.leggi_report(v.report_id));
            let Some(buf) = buf.as_mut() else { continue };
            let mut raw = 0u32;
            let st = unsafe {
                HidP_GetUsageValue(HIDP_FEATURE, v.page, v.coll, v.usage, &mut raw, self.pp,
                                   buf.as_mut_ptr(), buf.len() as u32)
            };
            if st != HIDP_STATUS_SUCCESS {
                continue;
            }
            let nome = nome_usage(v.page, v.usage);
            let chiave = format!("{}.{}", self.percorso(v.coll), nome);
            let mut c = nome.chars();
            let indice_stringa = c.next() == Some('i') && c.next().is_some_and(|ch| ch.is_ascii_uppercase());
            let valore = match indice_stringa.then(|| self.stringa(raw)).flatten() {
                Some(s) => Valore::Testo(s),
                None => Valore::Num(Self::scala(&v.cap, raw)),
            };
            dati.insert(chiave, valore);
        }
        for b in &self.bottoni {
            let buf = cache.entry(b.report_id).or_insert_with(|| self.leggi_report(b.report_id));
            let Some(buf) = buf.as_mut() else { continue };
            let mut lista = [0u16; 64];
            let mut n = lista.len() as u32;
            let st = unsafe {
                HidP_GetUsages(HIDP_FEATURE, b.page, b.coll, lista.as_mut_ptr(), &mut n, self.pp,
                               buf.as_mut_ptr(), buf.len() as u32)
            };
            if st != HIDP_STATUS_SUCCESS {
                continue;
            }
            let attivi = &lista[..n as usize];
            let base = self.percorso(b.coll);
            for u in b.umin..=b.umax {
                let acceso = if attivi.contains(&u) { 1.0 } else { 0.0 };
                dati.insert(format!("{base}.{}", nome_usage(b.page, u)), Valore::Num(acceso));
            }
        }
        dati
    }
}

impl Drop for Ups {
    fn drop(&mut self) {
        unsafe {
            if self.pp != 0 {
                HidD_FreePreparsedData(self.pp);
            }
            CloseHandle(self.handle);
        }
    }
}
