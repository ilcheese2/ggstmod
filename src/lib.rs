mod cxxstd;
mod output;
mod red;
mod ue4ss;
mod memory;

use crate::cxxstd::{CxxString, CxxStringView};
use crate::memory::signature_scan;
use crate::red::{ECharID, EColorID, Header, Packet_BattleReady};
use crate::ue4ss::{CppUserModBase, FMalloc, FString, Vtable, CONFIG_PATH};
use rand::seq::IndexedRandom;
use crate::ConfigError::NoneError;
use enum_map::{Enum, EnumMap};
use minhook::MinHook;
use serde::{Deserialize, Serialize};
use std::alloc::Layout;
use std::cmp::PartialEq;
use std::ffi::c_void;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::num::ParseIntError;
use std::str::FromStr;
use std::sync::OnceLock;
use libc::memcpy;
use strum::{IntoEnumIterator, ParseError};
use toml::{de, Table};
use widestring::U16CString;
use crate::output::{budget_log, clear_log};

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

fn is_color_allowed_for_config(char_id: ECharID, color_id: EColorID) -> bool { // hook must be disabled
    if color_id == EColorID(72) {
        return false;
    }
    unsafe { (HOOKS.get().unwrap().IsSelectableCharaColorID.target)(char_id , color_id) }
}

type Config = EnumMap<ECharID, Vec<EColorID>>;

pub fn parse_config(config: &str) -> Result<Config, ConfigError> {
    Table::from_str(config)?.into_iter().map(|(key, value)| {
        let char_id = ECharID::from_str(key.as_str())?;
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
    for char_id in ECharID::iter() {
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

type fn_IsSelectableCharaColorID = unsafe extern "C" fn(ECharID, EColorID) -> bool;
type fn_SendBattleReady = unsafe extern "C" fn(bool) -> bool;
type fn_SendPacket = unsafe extern "C" fn(u32, *mut Header, *mut c_void) -> bool;
type fn_Seq_GotoBattle = unsafe extern "C" fn(*mut c_void, bool);
type fn_ColorIdToDisplayNumber = unsafe extern "C" fn(*mut FString, EColorID) -> *mut FString;
type fn_IsAllowedCharaColorID = unsafe extern "C" fn(ECharID, EColorID) -> bool;

#[allow(non_snake_case)]
struct Hooks {
    UREDWidgetSimpleCharaSelect_OnInputPressTrigger: Hook<fn_UREDWidgetSimpleCharaSelect_OnInputPressTrigger>,
    IsSelectableCharaColorID: Hook<fn_IsSelectableCharaColorID>,
    SendBattleReady: Hook<fn_SendBattleReady>,
    SendPacket: Hook<fn_SendPacket>,
    Seq_GotoBattle: Hook<fn_Seq_GotoBattle>,
    ColorIdToDisplayNumber: Hook<fn_ColorIdToDisplayNumber>,
    IsAllowedCharaColorID: Hook<fn_IsAllowedCharaColorID>,
}

type Data = PhantomData<Hooks>;
static HOOKS: OnceLock<Hooks> = OnceLock::new();

pub unsafe extern "C"  fn on_update(this: *mut CppUserModBase<Data>) {
    //budget_log("update")
}

pub unsafe extern "C" fn destructor(this: *mut CppUserModBase<Data>) {
    budget_log("destructor");
}

pub unsafe extern "C" fn on_dll_load(this: *mut CppUserModBase<Data>, dll_name: *const CxxStringView) {}

pub unsafe extern "C" fn input_press(this: *mut c_void, flag: u32) {
    budget_log(("press: ".to_string() + flag.to_string().as_str() + "\n").as_str());
    let hooks = HOOKS.get().unwrap();
    (hooks.UREDWidgetSimpleCharaSelect_OnInputPressTrigger.orig)(this, flag);
}

pub unsafe extern "C" fn is_selectable_chara_color_id(char_id: ECharID, color_id: EColorID) -> bool {
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

pub unsafe extern "C" fn is_allowed_chara_color_id(char_id: ECharID, color_id: EColorID) -> bool {
    if color_id == EColorID(72) {
        return true;
    }
    let hooks = HOOKS.get().unwrap();
    (hooks.IsAllowedCharaColorID.orig)(char_id, color_id)
}

pub fn dummy_config() -> Config {
    let mut config = EnumMap::default();
    for char_id in ECharID::iter() {
        config[char_id] = vec![EColorID(0)];
    }
    config
}

const CONFIG_SUFFIX: &str = "\n# use \"1-4\" to add colors 1 through 4 inclusively\n# a higher frequency will correspond to a higher weight \n# KYK = [1,14,\"1-3\"], will have a 40% chance of picking color 1 and and a 20% change of picking colors 14, 2, or 3";

pub fn get_config() -> &'static Config {
    //UREDPlayerData::IsEnableCharaColor
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

fn get_random_color(chara: ECharID) -> EColorID {
    budget_log("get_random_color");
    let color = get_config()[chara].choose(&mut rand::rng()).unwrap().clone();
    budget_log("get_random_color2fg");
    color
}

pub unsafe extern "C" fn send_battle_ready(battle_ready: bool) -> bool {
    let hooks = HOOKS.get().unwrap();
    budget_log("send_battle_ready");
    (hooks.SendBattleReady.orig)(battle_ready)
}

pub unsafe extern "C" fn send_packet(socket_type: u32, header: *mut Header, peer_handle: *mut c_void) -> bool {
    let hooks = HOOKS.get().unwrap();
    if (*header).packet_type == 0x32 {
        let battle_ready: *mut Packet_BattleReady = std::mem::transmute(header);
        let battle_ready = &mut *battle_ready;
        for i in 0..3 {
            if battle_ready.color[i] == 72 {
                let color = get_random_color(ECharID::from_repr(battle_ready.chara[i] as u32).unwrap()).0 as i8;
                budget_log("get_random_color19");
                battle_ready.color[i] = color;
                budget_log("get_random_color4");
            }
        }
    }

    (hooks.SendPacket.orig)(socket_type, header, peer_handle)
}


pub unsafe extern "C" fn seq_gotobattle(this: *mut c_void, b_init: bool) { // different logic for training mode because doesn't save
    let hooks = HOOKS.get().unwrap();
    // technically I need to change chara history but it's static so too much work
    // if b_init {
    //     let mut chara_select: AREDGameState_CharaSelect = std::ptr::read_unaligned(std::mem::transmute(this));
    //     let mut slice = std::slice::from_raw_parts(this as *const u8, 0x1c90);
    //     budget_log("sliced");
    //
    //     budget_log(hex::encode(&slice).as_str());
    //     for  i in 0..2 {
    //         let side_info = &mut chara_select.side_info[i];
    //         budget_log(format!("chara: {:?}, color: {:?}", side_info.decide_info.chara_id, side_info.decide_info.color_id).as_str());
    //         if side_info.decide_info.color_id == 72 {
    //             side_info.decide_info.color_id = 3;
    //         }
    //     }
    // }
    (hooks.Seq_GotoBattle.orig)(this, b_init);
}

pub struct Hook<T> where T: Copy {
    target: T,
    orig: T,
}

impl<T: Copy> Hook<T> {
    pub fn enable(&self) {
        unsafe { MinHook::enable_hook(std::mem::transmute_copy(&ManuallyDrop::new(self.target))) };
    } // crying and sobbing
    pub fn disable(&self) {
        unsafe { MinHook::disable_hook(std::mem::transmute_copy(&ManuallyDrop::new(self.target))) };
    }
}

fn hook_function<T: Copy>(sig: &str, hook: T) -> Option<Hook<T>> {
    let addr = signature_scan(sig);
    if addr.is_none() {
        budget_log(("signature scan failed for: ".to_owned() + std::any::type_name::<T>()).as_str());
        return None;
    }
    let addr = addr.unwrap();

    let res = unsafe { MinHook::create_hook(addr as *mut c_void, std::mem::transmute_copy(&ManuallyDrop::new(hook))) };
    if res.is_err() {
        budget_log(("create hook failed for: ".to_owned() + std::any::type_name::<T>()).as_str());
        budget_log(format!("{:?}", res.unwrap_err()).as_str());
        return None;
    }

    let orig = res.unwrap();
    unsafe {
        Some(Hook {
            target: std::mem::transmute_copy(&ManuallyDrop::new(addr)),
            orig: std::mem::transmute_copy(&ManuallyDrop::new(orig)),
        })
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub unsafe extern "C" fn on_unreal_init(this: *mut CppUserModBase<Data>) {
    budget_log("unreal_init");

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
        Seq_GotoBattle: hook_function::<fn_Seq_GotoBattle>(
            "40 53 48 83 ec ? 48 8b d9 84 d2 0f 84 ? ? ? ? 48 89 bc 24",
            seq_gotobattle).unwrap(),
        ColorIdToDisplayNumber: hook_function::<fn_ColorIdToDisplayNumber>(
            "40 53 48 83 ec ? 48 8b d9 83 fa ? 74 ? 83 fa ? 74",
            color_id_to_display_number).unwrap(),
        IsAllowedCharaColorID: hook_function::<fn_IsAllowedCharaColorID>(
            "83 fa ? 76 ? 83 fa ? 75",
            is_allowed_chara_color_id).unwrap(),
    });

    let res = MinHook::enable_all_hooks();
    if res.is_err() {
        budget_log("enable all hooks failed");
        budget_log(format!("{:?}", res.unwrap_err()).as_str());
        return;
    }
}

pub unsafe extern "C" fn on_ui_init(this: *mut CppUserModBase<Data>) {
    budget_log("ui_init");
}
pub unsafe extern "C" fn on_program_start(this: *mut CppUserModBase<Data>) {
    budget_log("program_start");
}

pub unsafe extern "C" fn lua_function() {}

pub unsafe extern "C" fn render_tab(this: *mut CppUserModBase<Data>) {
    budget_log("render");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn start_mod() -> *mut CppUserModBase<Data> {
    clear_log();
    let vtable = Box::new(Vtable {
        destructor,
        on_update,
        on_unreal_init,
        on_ui_init,
        on_program_start,
        on_lua_start: lua_function,
        on_lua_start2: lua_function,
        on_lua_stop: lua_function,
        on_lua_stop2: lua_function,
        on_dll_load,
        render_tab,
        padding: [0; 0x8], // idk
    });
    unsafe {
        Box::into_raw(Box::new(CppUserModBase {
            vtable: Box::into_raw(vtable),
            padding: [0; 0x18],
            mod_name: CxxString::from_str("RandomCharacterColor"),
            mod_version: CxxString::from_str("1"),
            mod_description: CxxString::from_str(""),
            mod_authors: CxxString::from_str("ilcheese2"),
            mod_intended_sdk_version: CxxString::new(),
            data: Default::default()
        }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uninstall_mod(cpp_mod: *mut CppUserModBase<Data>) {
    std::ptr::drop_in_place(cpp_mod);
    std::alloc::dealloc(cpp_mod.cast(), Layout::new::<CppUserModBase<Data>>());
}