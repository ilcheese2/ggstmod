use crate::ConfigError::NoneError;
use enum_map::EnumMap;
use gglibrary::cxxstd::CxxString;
use gglibrary::memory::{hook_function, hook_function_from_addr, print_memory, signature_scan, signature_scan_from_addr, Hook, ThreadSafePtr};
use gglibrary::output::{budget_log, clear_log};
use gglibrary::red::{AREDGameState_CharaSelect, EBattleCharaSpFlag, ECharaID, EColorID, ECostumeID, Packet_BattleReady, SDecideInfoHistory};
use gglibrary::ue4ss::{CppUserModBase, FMalloc, FString, ModCallback, CONFIG_PATH};
use libc::memcpy;
use minhook::MinHook;
use rand::seq::IndexedRandom;
use serde::Deserialize;
use std::alloc::Layout;
use std::ffi::c_void;
use std::fmt::{format, Debug};
use std::fs::OpenOptions;
use std::io::Write;
use std::marker::PhantomData;
use std::mem::offset_of;
use std::num::ParseIntError;
use std::str::FromStr;
use std::sync::OnceLock;
use strum::{IntoEnumIterator, ParseError};
use toml::{de, Table};
use widestring::U16CString;

#[derive(Debug)]
enum ConfigError {
    ParseConfigError(de::Error),
    ParseRangeError(ParseIntError),
    ParseCharaError(ParseError),
    NoneError,
}

impl From<ParseIntError> for ConfigError {
    fn from(err: ParseIntError) -> ConfigError {
        ConfigError::ParseRangeError(err)
    }
}

impl From<de::Error> for ConfigError {
    fn from(err: de::Error) -> ConfigError {
        ConfigError::ParseConfigError(err)
    }
}

impl From<ParseError> for ConfigError {
    fn from(err: ParseError) -> ConfigError {
        ConfigError::ParseCharaError(err)
    }
}

fn is_color_allowed_for_config(char_id: ECharaID, color_id: EColorID) -> bool { // hook must be disabled
    if color_id == EColorID(72) {
        return false;
    }
    unsafe { (HOOKS.get().unwrap().IsSelectableCharaColorID.target)(char_id , color_id) }
}

type Config = EnumMap<ECharaID, Vec<EColorID>>;

pub fn parse_config(config: &str) -> Result<Config, ConfigError> {
    Table::from_str(config)?.into_iter().map(|(key, value)| {
        let char_id = ECharaID::from_str(key.as_str())?;
        let mut colors = Vec::new();
        let value = value.as_array().ok_or(NoneError)?;
        for color in value {

            if color.is_integer() {
                let color_id = EColorID::deserialize(color.clone())?;
                if is_color_allowed_for_config(char_id, color_id) {
                    colors.push(color_id);
                }
            } else if color.is_str() {
                let range_str = color.as_str().ok_or(NoneError)?;
                let mut parts = range_str.splitn(2, "-");
                let first = u32::from_str(parts.next().unwrap().trim())? - 1;
                let last = u32::from_str(parts.next().unwrap().trim())? - 1;
                for i in first..=last {
                    if is_color_allowed_for_config(char_id, EColorID(i)) {
                        colors.push(EColorID(i));
                    }
                }
            } else {
                return Err(NoneError)
            }
        }
        Ok((char_id, colors))
    }).collect()
}

pub fn create_config() -> Config {
    budget_log("creating config");
    let mut config = EnumMap::default();
    for char_id in ECharaID::iter() {
        let mut colors = Vec::new();
        for i in 0..99 {
            if is_color_allowed_for_config(char_id, EColorID(i)) {
                colors.push(EColorID(i));
            }
        }
        config[char_id] = colors;
    }
    config
}

type fn_UREDWidgetSimpleCharaSelect_OnInputPressTrigger = unsafe extern "C" fn(*mut c_void, u32);
type fn_IsSelectableCharaColorID = unsafe extern "C" fn(ECharaID, EColorID) -> bool;
type fn_SendBattleReady = unsafe extern "C" fn(bool) -> bool;
type fn_SendPacket = unsafe extern "C" fn(u32, *mut gglibrary::red::Header, *mut c_void) -> bool;
type fn_ColorIdToDisplayNumber = unsafe extern "C" fn(*mut FString, EColorID) -> *mut FString;
type fn_IsAllowedCharaColorID = unsafe extern "C" fn(ECharaID, EColorID) -> bool;
// AREDPawnCharaSelect::UpdateCharaAsset
type fn_UpdateCharaAsset = unsafe extern "C" fn(*mut c_void, ECharaID, EColorID, ECostumeID, EBattleCharaSpFlag, u32, bool);
type fn_GotoBattleSetting = unsafe extern "C" fn(*mut c_void);

#[allow(non_snake_case)]
struct Hooks {
    UREDWidgetSimpleCharaSelect_OnInputPressTrigger: Hook<fn_UREDWidgetSimpleCharaSelect_OnInputPressTrigger>,
    IsSelectableCharaColorID: Hook<fn_IsSelectableCharaColorID>,
    SendBattleReady: Hook<fn_SendBattleReady>,
    SendPacket: Hook<fn_SendPacket>,
    ColorIdToDisplayNumber: Hook<fn_ColorIdToDisplayNumber>,
    IsAllowedCharaColorID: Hook<fn_IsAllowedCharaColorID>,
    UpdateCharaAsset: Hook<fn_UpdateCharaAsset>,
    GotoBattleSetting: Hook<fn_GotoBattleSetting>,
    CharaHistory: Option<ThreadSafePtr<c_void>>,
}

type Data = PhantomData<Hooks>;
static HOOKS: OnceLock<Hooks> = OnceLock::new();

pub unsafe extern "C" fn input_press(this: *mut c_void, flag: u32) {
    budget_log(("press: ".to_string() + flag.to_string().as_str() + "\n").as_str());
    let hooks = HOOKS.get().unwrap();
    (hooks.UREDWidgetSimpleCharaSelect_OnInputPressTrigger.orig)(this, flag);
}

pub unsafe extern "C" fn is_selectable_chara_color_id(char_id: ECharaID, color_id: EColorID) -> bool {
    if color_id == EColorID(72) {
        return true;
    }
    let hooks = HOOKS.get().unwrap();
    (hooks.IsSelectableCharaColorID.orig)(char_id, color_id)
}

pub unsafe extern "C" fn color_id_to_display_number(result: *mut FString, color_id: EColorID) -> *mut FString {
    if color_id == EColorID(72) {
        let ptr = (*FMalloc)(2 * 4, 0);
        memcpy(ptr, U16CString::from_str("RND").unwrap().as_mut_ptr().cast(), 2 * 3 + 1);
        let fstring = &mut *result;
        fstring.data = ptr.cast();
        fstring.size = 4;
        fstring.capacity = 4;
        result
    }
    else {
        let hooks = HOOKS.get().unwrap();
        (hooks.ColorIdToDisplayNumber.orig)(result, color_id)
    }
}

pub unsafe extern "C" fn is_allowed_chara_color_id(char_id: ECharaID, color_id: EColorID) -> bool {
    if color_id == EColorID(72) {
        return true;
    }
    let hooks = HOOKS.get().unwrap();
    (hooks.IsAllowedCharaColorID.orig)(char_id, color_id)
}


pub unsafe extern "C" fn update_chara_asset(
    this: *mut c_void,
    char_id: ECharaID,
    color_id: EColorID,
    costume_id: ECostumeID,
    sp_flag: EBattleCharaSpFlag,
    side: u32,
    is_silhouette: bool,
) {
    let hooks = HOOKS.get().unwrap();
    if color_id == EColorID(72) {
        return (hooks.UpdateCharaAsset.orig)(this, char_id, EColorID(0), costume_id, sp_flag, side, true);
    }
    (hooks.UpdateCharaAsset.orig)(this, char_id, color_id, costume_id, sp_flag, side, is_silhouette);
}

pub unsafe extern "C" fn goto_battle_setting(this: *mut c_void) {
    let hooks = HOOKS.get().unwrap();
    budget_log("goto_battle_setting");
    let mut chara_select = (this as *mut AREDGameState_CharaSelect).as_mut().unwrap();

    let mut is_rand: [bool; 2] = [false, false];
    for  i in 0..2 {
        let side_info = &mut chara_select.side_info[i];
        budget_log(format!("chara: {:?}, color: {:?}", side_info.decide_info.chara_id, side_info.decide_info.color_id).as_str());
        if side_info.decide_info.color_id == EColorID(72) {
            side_info.decide_info.color_id = get_random_color(side_info.decide_info.chara_id);
            is_rand[i] = true;
        }
    }
    (hooks.GotoBattleSetting.orig)(this);
    if let Some(chara_history) = &hooks.CharaHistory {
        budget_log(print_memory(chara_history.cast(), size_of::<SDecideInfoHistory>()).as_str());
        let decide_history: *mut SDecideInfoHistory = std::mem::transmute(chara_history.0);

        if !(*decide_history).is_valid { // should be true
            return;
        }
        for i in 0..2 {
            let decide_info = &mut (*decide_history).chara_history[i];
            if is_rand[i] {
                decide_info.color_id = EColorID(72);
            }
        }
    }
}

pub fn dummy_config() -> Config {
    let mut config = EnumMap::default();
    for char_id in ECharaID::iter() {
        config[char_id] = vec![EColorID(0)];
    }
    config
}

const CONFIG_SUFFIX: &str = "\n# use \"1-4\" to add colors 1 through 4 inclusively\n# a higher frequency will correspond to a higher weight \n# KYK = [1,14,\"1-3\"], will have a 40% chance of picking color 1 and and a 20% change of picking colors 14, 2, or 3";

pub fn get_config() -> &'static Config {
    HOOKS.get().unwrap().IsSelectableCharaColorID.disable();
    let conf = CONFIG.get_or_init(|| {
        let config_path = (*CONFIG_PATH).as_str();
        budget_log(format!("config: {}", config_path).as_str());
        if !std::fs::exists(config_path).unwrap() {
            let config = create_config();
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(config_path);
            if file.is_err() {
                budget_log("failed to open config file");
                budget_log(format!("{:?}", file.err().unwrap()).as_str());
                dummy_config()
            } else {
                let table = toml::to_string(&config);
                if table.is_err() {
                    budget_log("failed to serialize config");
                    budget_log(format!("{:?}", table.err().unwrap()).as_str());
                    dummy_config()
                } else {
                    file.unwrap().write((table.unwrap() + CONFIG_SUFFIX).as_bytes()).unwrap();
                    config
                }
            }
        } else {
            let file = std::fs::read_to_string(config_path);
            if file.is_err() {
                budget_log("failed to read config file");
                budget_log(format!("{:?}", file.err().unwrap()).as_str());
                dummy_config()
            } else {
                budget_log("hello");
                parse_config(file.unwrap().as_str()).unwrap_or_else(|err| {
                    budget_log("failed to parse config");
                    budget_log(format!("{:?}", err).as_str());
                    dummy_config()
                })
            }
        }
    });
    budget_log(format!("{:?}", conf).as_str());
    HOOKS.get().unwrap().IsSelectableCharaColorID.enable();
    conf
}

fn get_random_color(chara: ECharaID) -> EColorID {
    let color = get_config()[chara].choose(&mut rand::rng()).unwrap().clone();
    color
}

pub unsafe extern "C" fn send_battle_ready(battle_ready: bool) -> bool {
    let hooks = HOOKS.get().unwrap();
    budget_log("send_battle_ready");
    (hooks.SendBattleReady.orig)(battle_ready)
}

pub unsafe extern "C" fn send_packet(socket_type: u32, header: *mut gglibrary::red::Header, peer_handle: *mut c_void) -> bool {
    let hooks = HOOKS.get().unwrap();
    if (*header).packet_type == 0x32 {
        let battle_ready: *mut Packet_BattleReady = std::mem::transmute(header);
        let battle_ready = &mut *battle_ready;
        for i in 0..3 {
            if battle_ready.color[i] == 72 {
                let color = get_random_color(ECharaID::from_repr(battle_ready.chara[i] as u32).unwrap()).0 as i8;
                battle_ready.color[i] = color;
            }
        }
    }

    (hooks.SendPacket.orig)(socket_type, header, peer_handle)
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub unsafe extern "C" fn on_unreal_init(this: *mut CppUserModBase<Data>) {
    budget_log("unreal_init");

    let addr = signature_scan("48 89 5c 24 ? 48 89 74 24 ? 48 89 7c 24 ? 55 41 54 41 55 41 56 41 57 48 8d 6c 24 ? 48 81 ec ? ? ? ? 48 8b 05 ? ? ? ? 48 33 c4 48 89 45 ? c6 05").unwrap();
    // mov rip+disp32 01
    // C6 05 DC CC 9E 03 01;
    let mut chara_history_inst = signature_scan_from_addr("C6 05 ? ? ? ? 01", addr);

    if let Some(inst) = chara_history_inst {
        let offset = (inst.offset(2) as *mut u32).read_unaligned();
        if offset < 1000000 || offset > 100000000 { // stupid sanity check
            chara_history_inst = None;
        } else {
            chara_history_inst = Some(inst.offset(offset as isize + 7 - 0x40));
        }
    }

    HOOKS.set(Hooks {
        UREDWidgetSimpleCharaSelect_OnInputPressTrigger: hook_function::<fn_UREDWidgetSimpleCharaSelect_OnInputPressTrigger>(
            "40 53 48 83 ec ?? 8b 81 ?? ?? ?? ?? 48 8b d9 85 c0 0f 85",
            input_press).unwrap(),
        IsSelectableCharaColorID: hook_function::<fn_IsSelectableCharaColorID>(
            "48 89 5c 24 ?? 48 89 74 24 ?? 48 89 7c 24 ?? 55 41 54 41 55 41 56 41 57 48 8b ec 48 83 ec ?? 45 33 ed 8b fa",
            is_selectable_chara_color_id).unwrap(),
        SendBattleReady: hook_function::<fn_SendBattleReady>(
            "4c 8b dc 55 49 8d ab ? ? ? ? 48 81 ec ? ? ? ? 48 8b 05 ? ? ? ? 48 33 c4 48 89 85 ? ? ? ? 49 89 5b ? 0f b6 d9",
            send_battle_ready).unwrap(),
        SendPacket: hook_function::<fn_SendPacket>(
            "4d 8b c8 4c 8b c2 8b d1 48 8b 0d ? ? ? ? e9",
            send_packet).unwrap(),
        ColorIdToDisplayNumber: hook_function::<fn_ColorIdToDisplayNumber>(
            "40 53 48 83 ec ? 48 8b d9 83 fa ? 74 ? 83 fa ? 74",
            color_id_to_display_number).unwrap(),
        IsAllowedCharaColorID: hook_function::<fn_IsAllowedCharaColorID>(
            "83 fa ? 76 ? 83 fa ? 75",
            is_allowed_chara_color_id).unwrap(),
        UpdateCharaAsset: hook_function::<fn_UpdateCharaAsset>(
            "4c 8b dc 45 89 4b ? 55 53",
            update_chara_asset).unwrap(),
        GotoBattleSetting: hook_function_from_addr::<fn_GotoBattleSetting>(
            addr as *mut c_void,
            goto_battle_setting).unwrap(),
        CharaHistory: chara_history_inst.map(|inst| {ThreadSafePtr(inst as *mut c_void)}),
    });

    let res = MinHook::enable_all_hooks();
    if res.is_err() {
        budget_log("enable all hooks failed");
        budget_log(format!("{:?}", res.unwrap_err()).as_str());
        return;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn start_mod() -> *mut CppUserModBase<Data> {
    clear_log();
    let vtable = Box::new(gglibrary::ue4ss::Vtable {
        on_unreal_init: ModCallback(on_unreal_init),
        ..Default::default()
    });

    unsafe {
        Box::into_raw(Box::new(CppUserModBase {
            vtable: Box::into_raw(vtable),
            padding: [0; 0x18],
            mod_name: CxxString::from_str("Random Chara Color"),
            mod_version: CxxString::from_str("1"),
            mod_description: CxxString::from_str(""),
            mod_authors: CxxString::from_str("ilcheese2"),
            mod_intended_sdk_version: CxxString::from_str(""),
            data: Default::default()
        }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uninstall_mod(cpp_mod: *mut CppUserModBase<Data>) {
    std::ptr::drop_in_place(cpp_mod);
    std::alloc::dealloc(cpp_mod.cast(), Layout::new::<CppUserModBase<Data>>());
}