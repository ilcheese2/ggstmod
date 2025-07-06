use std::ffi::c_void;
use enum_map::Enum;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use std::ops::Deref;
use strum::{Display, EnumIter, EnumString, FromRepr};

#[derive(
    Debug,
    PartialEq,
    Display,
    Enum,
    EnumString,
    EnumIter,
    FromRepr,
    Clone,
    Copy,
    Serialize,
    Deserialize,
)]
#[repr(u32)]
pub enum ECharaID {
    SOL,
    KYK,
    MAY,
    AXL,
    CHP,
    POT,
    FAU,
    MLL,
    ZAT,
    RAM,
    LEO,
    NAG,
    GIO,
    ANJ,
    INO,
    GLD,
    JKO,
    COS,
    BKN,
    TST,
    BGT,
    SIN,
    BED,
    ASK,
    JHN,
    ELP,
    ABA,
    SLY,
    DZY,
    VEN,
    UNI,
}

#[repr(u16)]
#[derive(Display, FromRepr, PartialEq)]
pub enum SessionPacketID {
    FrameData,
    BattleTerminate,
    FrameDataRequest, // UserMessageStart = 0x20  FrameData = UserMessageStart + 0x1
    FrameDataResponse,
    ReMatch,
    PlayerMatchResult,
    RCodeRequest,
    RCodeResponse,
    BattleSpectatorRequest,
    BattleSpectatorResponse,

    Chat,
    BattleReady,
    CommitMemberRequest,
    CommitPartRequest,
    RoomState,
    RoomState2,
    GotoBattle,
    NicoChat,
    SendCustomData,
    SendCustomDataHost,
    SendCustomDataHost2,
    SendCustomDataHost3,
    BattleReadyCoop,
    GotoBattleCoop,
    WatchingRequest,
    KickBall,
    BallInfo,
    PartyMessage,
    MatchResultMessage,
    WatchingRequest2,
    RoomStateExInfo,
    RoomStateExInfoHost,
    SendCoopInfoData,
    RPGSignal,
    CoopSignal,
    SendCustomDataSlot,
    GotoPacketParty,
    DisbandTeam,
    GotoPacketPartyPrev,
    TeamMatchCreated,
    FullSyncChange,
    FullSyncEnd,
    EasyAntiCheat,
}

pub type ECostumeID = i32;
pub type EBattleCharaSpFlag = bool;

#[repr(C)]
#[derive(PartialEq, Clone, Copy, Debug)]
pub struct EColorID(pub u32);

impl Deref for EColorID {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for EColorID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0 + 1)
    }
}

impl<'de> Deserialize<'de> for EColorID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let res = u32::deserialize(deserializer);
        if res.is_err() {
            res.map(EColorID)
        } else {
            Ok(EColorID(res.unwrap() - 1))
        }
    }
}

#[repr(C)]
pub struct SDecideInfo {
    pub chara_id: ECharaID,
    pub color_id: EColorID,
    costume_id: u32,
    script_id: u32,
    stage_id: u32,
    bgm_id: u32,
    sp_flag: u32,
    skill_set: u32,
}

#[repr(C)]
pub struct SSideInfo {
    side_id: u32,
    pad_id: u32,
    pub decide_info: SDecideInfo, // de(s)ide
    cpu: u32,
    page: i32,
}

#[repr(C)]
pub struct AREDGameState_CharaSelect {
    padding: [u8; 0xe68],
    pub side_info: [SSideInfo; 2],
}

#[repr(C)]
#[derive(Debug)]
pub struct Header {
    size: u16,
    pub packet_type: u16, // SessionPacketID
}

#[repr(C)]
pub struct Packet_BattleReady {
    header: Header,
    ready: i8,
    pub chara: [i8; 3], // why is this signed???? what
    pub color: [i8; 3],
    stage: i8,
    bgm: i16,
    dan: [i8; 3],
    costume: [i8; 3],
}

pub type EBGMID = i32;

// AREDGameState_CharaSelect::SDecideInfoHistory
#[repr(C)]
pub struct SDecideInfoHistory {
    pub chara_history: [SDecideInfo; 2],
    pub is_valid: bool,
    main_side: u32,
}

type RECFLG = u16;

#[repr(C)]
pub struct CMemorySlot {
    pub chara_id: u8,
    pub memory_direction: i32,
    pub memory_max_time: i32,
    pub memory_start_time: i32,
    pub memory_flags: [RECFLG; 60 * (5 + 40)], // counter-attack is 8/normal is 40
}

pub struct UUserWidget {
    pub vtable: *const UREDWidgetBase_vtbl,
}

pub struct UREDWidgetBase {
    pub widget: UUserWidget,
}

pub struct UREDUMGCommonWidget {
    pub widget_base: UREDWidgetBase,
}

pub struct UREDUMGCommonWindowBase {
    pub common_widget: UREDUMGCommonWidget,
}

pub struct UREDCommonSelectorWindowBase {
    pub common_window: UREDUMGCommonWindowBase,
}


#[repr(C)]
pub struct UUserWidget_vtbl { // this was a waste of time
    pub destructor: fn(this: *mut c_void),
    pub register_dependencies: fn(this: *mut c_void),
    pub deferred_register: fn(this: *mut UUserWidget, class: *mut c_void, package: *const c_void, name: *const c_void),
    pub can_be_cluster_root: fn(this: *mut UUserWidget) -> bool,
    pub can_be_in_cluster: fn(this: *mut UUserWidget) -> bool,
    pub create_cluster: fn(this: *mut UUserWidget),
    pub on_cluster_marked_as_pending_kill: fn(this: *mut UUserWidget),
    pub get_detailed_info_internal: fn(this: *mut UUserWidget, result: *mut c_void) -> *mut c_void,
    pub post_init_properties: fn(this: *mut UUserWidget),
    pub post_cdo_construct: fn(this: *mut UUserWidget),
    pub pre_save_root: fn(this: *mut UUserWidget, platform: *const c_void) -> bool,
    pub post_save_root: fn(this: *mut UUserWidget, b_cleanup_is_required: bool),
    pub pre_save: fn(this: *mut UUserWidget, platform: *const c_void),
    pub is_ready_for_async_post_load: fn(this: *mut UUserWidget) -> bool,
    pub post_load: fn(this: *mut UUserWidget),
    pub post_load_subobjects: fn(this: *mut UUserWidget, instancing_graph: *mut c_void),
    pub begin_destroy: fn(this: *mut UUserWidget),
    pub is_ready_for_finish_destroy: fn(this: *mut UUserWidget) -> bool,
    pub finish_destroy: fn(this: *mut UUserWidget),
    pub serialize: fn(this: *mut UUserWidget, record: c_void),
    pub serialize_2: fn(this: *mut UUserWidget, archive: *mut c_void),
    pub shutdown_after_error: fn(this: *mut UUserWidget),
    pub post_interp_change: fn(this: *mut UUserWidget, property: *mut c_void),
    pub post_rename: fn(this: *mut UUserWidget, old_object: *mut c_void, new_name: c_void),
    pub post_duplicate: fn(this: *mut UUserWidget, mode: u32),
    pub post_duplicate_2: fn(this: *mut UUserWidget, b_duplicate_for_editor: bool),
    pub needs_load_for_client: fn(this: *mut UUserWidget) -> bool,
    pub needs_load_for_server: fn(this: *mut UUserWidget) -> bool,
    pub needs_load_for_target_platform: fn(this: *mut UUserWidget, platform: *const c_void) -> bool,
    pub needs_load_for_editor_game: fn(this: *mut UUserWidget) -> bool,
    pub is_editor_only: fn(this: *mut UUserWidget) -> bool,
    pub is_post_load_thread_safe: fn(this: *mut UUserWidget) -> bool,
    pub is_destruction_thread_safe: fn(this: *mut UUserWidget) -> bool,
    pub get_preload_dependencies: fn(this: *mut UUserWidget, out_deps: *mut c_void),
    pub get_prestream_packages: fn(this: *mut UUserWidget, out_packages: *mut c_void),
    pub export_custom_properties: fn(this: *mut UUserWidget, output_device: *mut c_void, flags: u32),
    pub import_custom_properties: fn(this: *mut UUserWidget, filename: *const c_void, feedback_context: *mut c_void),
    pub post_edit_import: fn(this: *mut UUserWidget),
    pub post_reload_config: fn(this: *mut UUserWidget, property: *mut c_void),
    pub rename: fn(this: *mut UUserWidget, new_name: *const c_void, new_outer: *mut c_void, flags: u32) -> bool,
    pub get_desc: fn(this: *mut UUserWidget, result: *mut c_void) -> *mut c_void,
    pub get_sparse_class_data_struct: fn(this: *mut UUserWidget) -> *mut c_void,
    pub get_world: fn(this: *mut UUserWidget) -> *mut c_void,
    pub get_native_property_values: fn(this: *mut UUserWidget, out_values: *mut c_void, flags: u32) -> bool,
    pub get_resource_size_ex: fn(this: *mut UUserWidget, resource_size: *mut c_void),
    pub get_exporter_name: fn(this: *mut UUserWidget, result: *mut c_void) -> *mut c_void,
    pub get_restore_for_uobject_overwrite: fn(this: *mut UUserWidget) -> *mut c_void,
    pub are_native_properties_identical_to: fn(this: *mut UUserWidget, other: *mut c_void) -> bool,
    pub get_asset_registry_tags: fn(this: *mut UUserWidget, out_tags: *mut c_void),
    pub is_asset: fn(this: *mut UUserWidget) -> bool,
    pub get_primary_asset_id: fn(this: *mut UUserWidget, result: *mut c_void) -> *mut c_void,
    pub is_localized_resource: fn(this: *mut UUserWidget) -> bool,
    pub is_safe_for_root_set: fn(this: *mut UUserWidget) -> bool,
    pub tag_subobjects: fn(this: *mut UUserWidget, flags: u32),
    pub get_lifetime_replicated_props: fn(this: *mut UUserWidget, out_props: *mut c_void),
    pub is_name_stable_for_networking: fn(this: *mut UUserWidget) -> bool,
    pub is_full_name_stable_for_networking: fn(this: *mut UUserWidget) -> bool,
    pub is_supported_for_networking: fn(this: *mut UUserWidget) -> bool,
    pub get_subobjects_with_stable_names_for_networking: fn(this: *mut UUserWidget, out_objects: *mut c_void),
    pub pre_net_receive: fn(this: *mut UUserWidget),
    pub post_net_receive: fn(this: *mut UUserWidget),
    pub post_rep_notifies: fn(this: *mut UUserWidget),
    pub pre_destroy_from_replication: fn(this: *mut UUserWidget),
    pub post_destroy_from_replication: fn(this: *mut UUserWidget),
    pub build_subobject_mapping: fn(this: *mut UUserWidget, new_object: *mut c_void, out_mapping: *mut c_void),
    pub get_config_override_platform: fn(this: *mut UUserWidget) -> *const c_void,
    pub override_per_object_config_section: fn(this: *mut UUserWidget, out_section: *mut c_void),
    pub process_event: fn(this: *mut UUserWidget, function: *mut c_void, params: *mut c_void),
    pub get_function_callspace: fn(this: *mut UUserWidget, function: *mut c_void, frame: *mut c_void) -> i32,
    pub call_remote_function: fn(
        this: *mut UUserWidget,
        function: *mut c_void,
        params: *mut c_void,
        out_params: *mut c_void,
        frame: *mut c_void,
    ) -> bool,
    pub process_console_exec: fn(this: *mut UUserWidget, command: *const c_void, output_device: *mut c_void, world_context: *mut c_void) -> bool,
    pub regenerate_class: fn(this: *mut UUserWidget, class: *mut c_void, new_object: *mut c_void) -> *mut c_void,
    pub mark_as_editor_only_subobject: fn(this: *mut UUserWidget),
    pub check_default_subobjects_internal: fn(this: *mut UUserWidget) -> bool,
    pub validate_generated_rep_enums: fn(this: *mut UUserWidget, rep_records: *const c_void),
    pub set_net_push_id_dynamic: fn(this: *mut UUserWidget, net_push_id: i32),
    pub get_net_push_id_dynamic: fn(this: *mut UUserWidget) -> i32,
    pub release_slate_resources: fn(this: *mut UUserWidget, release_children: bool),
    pub set_is_enabled: fn(this: *mut UUserWidget, b_in_is_enabled: bool),
    pub set_visibility: fn(this: *mut UUserWidget, visibility: u8),
    pub is_hovered: fn(this: *mut UUserWidget) -> bool,
    pub remove_from_parent: fn(this: *mut UUserWidget),
    pub get_owning_player: fn(this: *mut UUserWidget) -> *mut c_void,
    pub get_owning_local_player: fn(this: *mut UUserWidget) -> *mut c_void,
    pub synchronize_properties: fn(this: *mut UUserWidget),
    pub on_binding_changed: fn(this: *mut UUserWidget, property_name: *const c_void),
    pub rebuild_widget: fn(this: *mut UUserWidget, result: *mut c_void) -> *mut c_void,
    pub on_widget_rebuilt: fn(this: *mut UUserWidget),
    pub get_accessible_widget: fn(this: *mut UUserWidget, result: *mut c_void) -> *mut c_void,
}

#[repr(C)]
pub struct UREDWidgetBase_vtbl {
    pub user_widget_vtbl: UUserWidget_vtbl,
    pub initialize: fn(this: *mut UREDWidgetBase) -> bool,
    pub template_init_inner: fn(this: *mut UREDWidgetBase),
    pub initialize_native_class_data: fn(this: *mut UREDWidgetBase),
    pub on_animation_started_implementation: fn(this: *mut UREDWidgetBase, animation: *const c_void),
    pub on_animation_finished_implementation: fn(this: *mut UREDWidgetBase, animation: *const c_void),
    pub on_animation_started_playing: fn(this: *mut UREDWidgetBase, player: *mut c_void),
    pub on_animation_finished_playing: fn(this: *mut UREDWidgetBase, player: *mut c_void),
    pub add_to_screen: fn(this: *mut UREDWidgetBase, local_player: *mut c_void, z_order: i32),
    pub on_level_removed_from_world: fn(this: *mut UREDWidgetBase, level: *mut c_void, world: *mut c_void),
    pub native_on_initialized: fn(this: *mut UREDWidgetBase),
    pub native_pre_construct: fn(this: *mut UREDWidgetBase),
    pub native_construct: fn(this: *mut UREDWidgetBase),
    pub native_destruct: fn(this: *mut UREDWidgetBase),
    pub native_tick: fn(this: *mut UREDWidgetBase, geometry: *const c_void, delta_time: f32),
    pub native_paint: fn(this: *mut UREDWidgetBase, args: *const c_void, geometry: *const c_void, clip_rect: *const c_void, element_list: *mut c_void, layer_id: i32, widget_style: *const c_void, b_parent_enabled: bool) -> i32,
    pub native_paint_2: fn(this: *mut UREDWidgetBase, context: *mut c_void),
    pub native_is_interactable: fn(this: *mut UREDWidgetBase) -> bool,
    pub native_supports_keyboard_focus: fn(this: *mut UREDWidgetBase) -> bool,
    pub native_supports_custom_navigation: fn(this: *mut UREDWidgetBase) -> bool,
    pub native_on_focus_received: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, focus_event: *const c_void) -> *mut c_void,
    pub native_on_focus_lost: fn(this: *mut UREDWidgetBase, focus_event: *const c_void),
    pub native_on_focus_changing: fn(this: *mut UREDWidgetBase, weak_widget_path: *const c_void, widget_path: *const c_void, focus_event: *const c_void),
    pub native_on_added_to_focus_path: fn(this: *mut UREDWidgetBase, focus_event: *const c_void),
    pub native_on_removed_from_focus_path: fn(this: *mut UREDWidgetBase, focus_event: *const c_void),
    pub native_on_navigation: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, navigation_event: *const c_void) -> *mut c_void,
    pub native_on_navigation_2: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, navigation_event: *const c_void, navigation_reply: *const c_void) -> *mut c_void,
    pub native_on_key_char: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, key_event: *const c_void) -> *mut c_void,
    pub native_on_preview_key_down: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, key_event: *const c_void) -> *mut c_void,
    pub native_on_key_down: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, key_event: *const c_void) -> *mut c_void,
    pub native_on_key_up: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, key_event: *const c_void) -> *mut c_void,
    pub native_on_analog_value_changed: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, key_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_button_down: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_preview_mouse_button_down: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_button_up: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_move: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_enter: fn(this: *mut UREDWidgetBase, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_leave: fn(this: *mut UREDWidgetBase, mouse_event: *const c_void),
    pub native_on_mouse_wheel: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_button_double_click: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_drag_detected: fn(this: *mut UREDWidgetBase, geometry: *const c_void, mouse_event: *const c_void, operation: *mut *mut c_void),
    pub native_on_drag_enter: fn(this: *mut UREDWidgetBase, geometry: *const c_void, drag_event: *const c_void, operation: *mut c_void),
    pub native_on_drag_leave: fn(this: *mut UREDWidgetBase, drag_event: *const c_void, operation: *mut c_void),
    pub native_on_drag_over: fn(this: *mut UREDWidgetBase, geometry: *const c_void, drag_event: *const c_void, operation: *mut c_void) -> bool,
    pub native_on_drop: fn(this: *mut UREDWidgetBase, geometry: *const c_void, drag_event: *const c_void, operation: *mut c_void) -> bool,
    pub native_on_drag_cancelled: fn(this: *mut UREDWidgetBase, drag_event: *const c_void, operation: *mut c_void),
    pub native_on_touch_gesture: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, touch_event: *const c_void) -> *mut c_void,
    pub native_on_touch_started: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, touch_event: *const c_void) -> *mut c_void,
    pub native_on_touch_moved: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, touch_event: *const c_void) -> *mut c_void,
    pub native_on_touch_ended: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, touch_event: *const c_void) -> *mut c_void,
    pub native_on_motion_detected: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, motion_event: *const c_void) -> *mut c_void,
    pub native_on_touch_force_changed: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, touch_event: *const c_void) -> *mut c_void,
    pub native_on_cursor_query: fn(this: *mut UREDWidgetBase, result: *mut c_void, geometry: *const c_void, mouse_event: *const c_void) -> *mut c_void,
    pub native_on_mouse_capture_lost: fn(this: *mut UREDWidgetBase, capture_lost_event: *const c_void),
    pub initialize_input_component: fn(this: *mut UREDWidgetBase),
    pub end_widget_implementation: fn(this: *mut UREDWidgetBase),
    pub start_widget_implementation: fn(this: *mut UREDWidgetBase),
    pub start_widget_cpp: fn(this: *mut UREDWidgetBase),
    pub end_widget_cpp: fn(this: *mut UREDWidgetBase),
    pub red_widget_tick: fn(this: *mut UREDWidgetBase, delta_time: f32),
    pub on_end_child_animation: fn(this: *mut UREDWidgetBase, animation_manager: *mut c_void, animation_name: *const c_void),
    pub loading_screen_start: fn(this: *mut UREDWidgetBase),
    pub loading_screen_end: fn(this: *mut UREDWidgetBase),
}

#[repr(C)]
pub struct UREDUMGCommonWidget_vtbl {
    pub widget_base_vtbl: UREDWidgetBase_vtbl,
    pub is_enabled_input: fn(this: *mut UREDUMGCommonWidget) -> bool,
    pub on_input_button: fn(this: *mut UREDUMGCommonWidget, button: u32),
    pub on_input_press_trigger: fn(this: *mut UREDUMGCommonWidget, trigger: u32),
    pub on_input_release_trigger: fn(this: *mut UREDUMGCommonWidget, trigger: u32),
    pub on_input_repeat: fn(this: *mut UREDUMGCommonWidget, button: u32),
    pub on_input_decision_trigger: fn(this: *mut UREDUMGCommonWidget),
    pub on_input_cancel_trigger: fn(this: *mut UREDUMGCommonWidget),
    pub on_set_pad_id: fn(this: *mut UREDUMGCommonWidget),
    pub format_input_data: fn(this: *mut UREDUMGCommonWidget, input_data: *mut u32, output_data: *mut u32, output_size: *mut u32),
}

type ECommonWindowCloseReason = u32;

#[repr(C)]
pub struct UREDUMGCommonWindowBase_vtbl {
    pub common_widget_vtbl: UREDUMGCommonWidget_vtbl,
    pub on_opened: fn(this: *mut UREDUMGCommonWindowBase) -> bool,
    pub on_closing: fn(this: *mut UREDUMGCommonWindowBase, close_reason: ECommonWindowCloseReason, play_anim: *const bool) -> bool,
    pub can_transition_closed: fn(this: *mut UREDUMGCommonWindowBase) -> bool,
    pub on_closed: fn(this: *mut UREDUMGCommonWindowBase, close_reason: ECommonWindowCloseReason),
    pub on_resuming: fn(this: *mut UREDUMGCommonWindowBase),
    pub on_starting_for_umg: fn(this: *mut UREDUMGCommonWindowBase),
    pub on_ending_for_umg: fn(this: *mut UREDUMGCommonWindowBase),
    pub on_after_start_for_umg: fn(this: *mut UREDUMGCommonWindowBase),
    pub on_after_end_for_umg: fn(this: *mut UREDUMGCommonWindowBase),
    pub on_animation_end: fn(this: *mut UREDUMGCommonWindowBase, animation_manager: *mut c_void, animation_name: *const c_void),
}

#[repr(C)]
pub struct UREDCommonSelectorWindowBase_vtbl {
    pub common_window_base_vtbl: UREDUMGCommonWindowBase_vtbl,
    pub is_possible_enabled_input: fn(this: *mut UREDCommonSelectorWindowBase) -> bool,
    pub on_changed_cursor: fn(this: *mut UREDCommonSelectorWindowBase, x: i32, y: i32),
    pub on_added_item: fn(this: *mut UREDCommonSelectorWindowBase, item: *mut c_void),
    pub on_cleared_item: fn(this: *mut UREDCommonSelectorWindowBase),
}

#[repr(C)]
pub struct SSaveData {
    padding: [u8; 0x282f4],
    pub memory_slot_blob: [CMemorySlot; 8],
}

// bool (__fastcall *IsPossibleEnabledInput)(UREDCommonSelectorWindowBase *this);
// 00000558     void (__fastcall *OnChangedCursor)(UREDCommonSelectorWindowBase *this, int, int);
// 00000560     void (__fastcall *OnAddedItem)(UREDCommonSelectorWindowBase *this, UWidget *);
// 00000568     void (__fastcall *OnClearedItem)(UREDCommonSelectorWindowBase *this);
//
//

#[repr(C)]
pub struct UREDWidgetSelectableItem_vtbl {
    pub uredwidget_base_vtbl: UREDWidgetBase_vtbl,

}

#[repr(C)]
pub struct UREDWidgetSelectableItem {

}