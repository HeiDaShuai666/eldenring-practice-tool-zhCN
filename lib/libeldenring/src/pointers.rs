use std::ptr::null_mut;

use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

use crate::base_addresses::{self, BaseAddresses};
use crate::memedit::*;
use crate::prelude::Version;

#[derive(Debug)]
pub struct Pointers {
    pub one_shot: Bitflag<u8>,
    pub no_damage: Bitflag<u8>,
    pub no_dead: Bitflag<u8>,
    pub no_hit: Bitflag<u8>,
    pub no_goods_consume: Bitflag<u8>,
    pub no_stamina_consume: Bitflag<u8>,
    pub no_fp_consume: Bitflag<u8>,
    pub no_arrows_consume: Bitflag<u8>,
    pub no_attack: Bitflag<u8>,
    pub no_move: Bitflag<u8>,
    pub no_update_ai: Bitflag<u8>,
    pub no_ashes_of_war_fp_consume: Bitflag<u8>,

    pub runes: PointerChain<u32>,
    pub igt: PointerChain<usize>,

    pub weapon_hitbox1: Bitflag<u8>, // debug sphere 1
    pub weapon_hitbox2: Bitflag<u8>, // debug sphere 2
    pub weapon_hitbox3: Bitflag<u8>, // damipoli

    pub quitout: PointerChain<u8>,
    pub cursor_show: Bitflag<u8>,

    pub gravity: Bitflag<u8>,
    pub display_stable_pos: Bitflag<u8>,
    pub position: Position,
    pub stable_position: Position,
    pub world_position: Position,
    pub animation_speed: PointerChain<f32>,

    // HitIns
    pub hitbox_high: Bitflag<u8>,
    pub hitbox_low: Bitflag<u8>,
    pub hitbox_character: Bitflag<u8>,

    // FieldArea
    pub field_area_direction: Bitflag<u8>,
    pub field_area_altimeter: Bitflag<u8>,
    pub field_area_compass: Bitflag<u8>,

    // GroupMask
    pub show_map: Bitflag<u8>,
    pub show_chr: Bitflag<u8>,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub x: PointerChain<f32>,
    pub y: PointerChain<f32>,
    pub z: PointerChain<f32>,
    pub angle: PointerChain<f32>,
}

impl Pointers {
    pub fn new() -> Self {
        let base_module_address = unsafe { GetModuleHandleA(PCSTR(null_mut())) }.0 as usize;
        let BaseAddresses {
            chr_dbg_flags,
            csfd4_virtual_memory_flag,
            cs_flipper,
            cs_lua_event_manager,
            cs_menu_man,
            cs_menu_man_imp,
            cs_net_man,
            cs_regulation_manager,
            cs_session_manager,
            damage_ctrl,
            field_area,
            game_data_man,
            game_man,
            group_mask,
            hit_ins,
            map_item_man,
            menu_man_ins,
            msg_repository,
            solo_param_repository,
            world_chr_man,
            world_chr_man_dbg,
            world_chr_man_imp,
        } = match *crate::version::VERSION {
            Version::V1_02_0 => base_addresses::BASE_ADDRESSES_1_02_0,
            Version::V1_02_1 => base_addresses::BASE_ADDRESSES_1_02_1,
            Version::V1_02_2 => base_addresses::BASE_ADDRESSES_1_02_2,
            Version::V1_02_3 => base_addresses::BASE_ADDRESSES_1_02_3,
            Version::V1_03_0 => base_addresses::BASE_ADDRESSES_1_03_0,
            Version::V1_03_1 => base_addresses::BASE_ADDRESSES_1_03_1,
            Version::V1_03_2 => base_addresses::BASE_ADDRESSES_1_03_2,
        }
        .with_module_base_addr(base_module_address);

        Self {
            one_shot: bitflag!(0b1; chr_dbg_flags + 0x3),
            no_damage: bitflag!(0b1; chr_dbg_flags + 0xC),
            no_dead: bitflag!(0b1; chr_dbg_flags + 0xB),
            no_hit: bitflag!(0b1; chr_dbg_flags + 0xD),
            no_goods_consume: bitflag!(0b1; chr_dbg_flags + 0x4),
            no_stamina_consume: bitflag!(0b1; chr_dbg_flags + 0x5),
            no_fp_consume: bitflag!(0b1; chr_dbg_flags + 0x6),
            no_arrows_consume: bitflag!(0b1; chr_dbg_flags + 0x7),
            no_attack: bitflag!(0b1; chr_dbg_flags + 0xE),
            no_move: bitflag!(0b1; chr_dbg_flags + 0xF),
            no_update_ai: bitflag!(0b1; chr_dbg_flags + 0x10),
            no_ashes_of_war_fp_consume: bitflag!(0b1; chr_dbg_flags + 0x12),
            runes: pointer_chain!(game_data_man, 0x8, 0x6C),
            igt: pointer_chain!(game_data_man, 0xA0),
            quitout: pointer_chain!(cs_menu_man_imp, 0x8, 0x5d),
            cursor_show: bitflag!(0b1; cs_menu_man_imp, 0xAC),
            gravity: bitflag!(0b1; world_chr_man, 0x18468, 0x190, 0x68, 0x1d3),
            display_stable_pos: bitflag!(0b1; world_chr_man, 0x18468, 0x6FD),
            position: Position {
                x: pointer_chain!(world_chr_man, 0x18468, 0x6b8),
                y: pointer_chain!(world_chr_man, 0x18468, 0x6bc),
                z: pointer_chain!(world_chr_man, 0x18468, 0x6c0),
                angle: pointer_chain!(world_chr_man, 0x18468, 0x6c4),
            },
            stable_position: Position {
                x: pointer_chain!(world_chr_man, 0x18468, 0x6cc),
                y: pointer_chain!(world_chr_man, 0x18468, 0x6d0),
                z: pointer_chain!(world_chr_man, 0x18468, 0x6d4),
                angle: pointer_chain!(world_chr_man, 0x18468, 0x6d8),
            },
            world_position: Position {
                x: pointer_chain!(world_chr_man, 0x18468, 0x190, 0x68, 0x70),
                y: pointer_chain!(world_chr_man, 0x18468, 0x190, 0x68, 0x74),
                z: pointer_chain!(world_chr_man, 0x18468, 0x190, 0x68, 0x78),
                angle: pointer_chain!(world_chr_man, 0x18468, 0x190, 0x68, 0x54),
            },
            animation_speed: pointer_chain!(world_chr_man, 0xB658, 0, 0x190, 0x28, 0x17C8),
            field_area_direction: bitflag!(0b1; field_area + 0x9),
            field_area_altimeter: bitflag!(0b1; field_area + 0xA),
            field_area_compass: bitflag!(0b1; field_area + 0xB),
            weapon_hitbox1: bitflag!(0b1; damage_ctrl, 0xA0),
            weapon_hitbox2: bitflag!(0b1; damage_ctrl, 0xA1),
            weapon_hitbox3: bitflag!(0b1; damage_ctrl, 0xA4),
            hitbox_high: bitflag!(0b1; hit_ins + 0xC),
            hitbox_low: bitflag!(0b1; hit_ins + 0xD),
            hitbox_character: bitflag!(0b1; hit_ins + 0xF),
            show_map: bitflag!(0b1; group_mask + 0x1),
            show_chr: bitflag!(0b1; group_mask + 0xD),
        }
    }
}

// PlayerIns = WorldChrMan, 18468
//   Position  = WorldChrMan, 18468, 6B8
//                                   6BC
//                                   6C0
//                                   6C4   x y z rad   floats
//   StablePos = WorldChrMan, 18468, 6CC
//                                   6D0
//                                   6D4
//                                   6D8
//   Display stable pos = WorldChrMan, 18468, 6FD byte
//   Chunks = WorldChrMan, 18468, 190, 68, 54 rad <- this is the good position
//                                         70 x
//                                         74 y
//                                         78 z
// HIT_INS = 48 8B 05 ?? ?? ?? ?? 48 8D 4C 24 ?? 48 89 4c 24 ?? 0F 10 44 24 70
//   Low hit = HIT_INS + C
//   High hit = HIT_INS + D
//   Char Hitbox = HIT_INS  F
// FieldArea, 9/A/B direction/altimeter/compass
// GroupMask = ?? 80 3D ?? ?? ?? ?? 00 0F 10 00 0F 11 45 D0 0F 84 ?? ?? ?? ?? 80 3D
//   Show Map = +1
//   Show Chr = +D

/*
[ENABLE]
alloc(ItemData,512,eldenring.exe)
registersymbol(ItemData)

aobscanmodule(ItemDropCall,eldenring.exe,48 8B C4 56 57 41 56 48 81 EC ?? ?? ?? ?? 48 C7 44 24 ?? ?? ?? ?? ?? 48 89 58 ?? 48 89 68 ?? 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 84 24 ?? ?? ?? ?? 41 0F B6 F9)
registersymbol(ItemDropCall)

alloc(ItemDropData,16)
registersymbol(ItemDropData)

ItemDropData:
dd #150
dd #01
dd 40000000
dd FFFFFFFF

ItemData:
mov rcx,[MapItemMan]
lea rdx,[ItemDropData]

xor r9d,r9d
lea r8d,[r9+01]

mov eax,[ItemDropData+08]
add [ItemDropData],eax

sub rsp,28
call ItemDropCall
add rsp,28

mov eax,[ItemDropData+08]
sub [ItemDropData],eax
ret
*/