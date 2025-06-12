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