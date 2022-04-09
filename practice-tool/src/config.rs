use libeldenring::prelude::*;

use std::str::FromStr;

use log::LevelFilter;
use serde::Deserialize;

use crate::util;
use crate::util::KeyState;
use crate::widgets::cycle_speed::CycleSpeed;
use crate::widgets::flag::Flag;
use crate::widgets::multiflag::MultiFlag;
// use crate::widgets::item_spawn::ItemSpawner;
use crate::widgets::position::SavePosition;
use crate::widgets::quitout::Quitout;
use crate::widgets::savefile_manager::SavefileManager;
use crate::widgets::runes::Runes;
use crate::widgets::Widget;

#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) settings: Settings,
    commands: Vec<CfgCommand>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    pub(crate) log_level: LevelFilterSerde,
    pub(crate) display: KeyState,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Deserialize)]
#[serde(untagged)]
enum CfgCommand {
    SavefileManager {
        #[serde(rename = "savefile_manager")]
        hotkey_load: KeyState,
        hotkey_back: KeyState,
        hotkey_close: KeyState,
    },
    // ItemSpawner {
    //     #[serde(rename = "item_spawner")]
    //     hotkey_load: KeyState,
    //     hotkey_back: KeyState,
    //     hotkey_close: KeyState,
    // },
    Flag {
        flag: FlagSpec,
        hotkey: Option<KeyState>,
    },
    MultiFlag {
        flags: Vec<FlagSpec>,
        hotkey: Option<KeyState>,
        label: String,
    },
    Position {
        #[serde(rename = "position")]
        hotkey: KeyState,
        modifier: KeyState,
    },
    CycleSpeed {
        #[serde(rename = "cycle_speed")]
        cycle_speed: Vec<f32>,
        hotkey: KeyState,
    },
    Runes {
        #[serde(rename = "runes")]
        amount: u32,
        hotkey: KeyState,
    },
    Quitout {
        #[serde(rename = "quitout")]
        hotkey: KeyState,
    },
}

#[derive(Deserialize, Debug)]
#[serde(try_from = "String")]
pub(crate) struct LevelFilterSerde(log::LevelFilter);

impl LevelFilterSerde {
    pub(crate) fn inner(&self) -> log::LevelFilter {
        self.0
    }
}

impl TryFrom<String> for LevelFilterSerde {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(LevelFilterSerde(
            log::LevelFilter::from_str(&value)
                .map_err(|e| format!("Couldn't parse log level filter: {}", e))?,
        ))
    }
}

impl Config {
    pub(crate) fn parse(cfg: &str) -> Result<Self, String> {
        toml::from_str::<Config>(cfg).map_err(|e| format!("TOML configuration parse error: {}", e))
    }

    pub(crate) fn make_commands(&self, chains: &Pointers) -> Vec<Box<dyn Widget>> {
        self.commands
            .iter()
            .map(|cmd| match cmd {
                CfgCommand::Flag { flag, hotkey } => Box::new(Flag::new(
                    &flag.label,
                    (flag.getter)(chains).clone(),
                    hotkey.clone(),
                )) as Box<dyn Widget>,
                CfgCommand::MultiFlag { flags, hotkey, label } => Box::new(MultiFlag::new(
                    label,
                    flags.into_iter().map(|flag| (flag.getter)(chains).clone()).collect(),
                    hotkey.clone(),
                )) as Box<dyn Widget>,
                CfgCommand::SavefileManager {
                    hotkey_load,
                    hotkey_back,
                    hotkey_close,
                } => SavefileManager::new_widget(
                    hotkey_load.clone(),
                    hotkey_back.clone(),
                    hotkey_close.clone(),
                ),
                // CfgCommand::ItemSpawner {
                //     hotkey_load,
                //     hotkey_back,
                //     hotkey_close,
                // } => Box::new(ItemSpawner::new(
                //     chains.spawn_item_func_ptr,
                //     chains.map_item_man,
                //     chains.gravity.clone(),
                //     hotkey_load.clone(),
                //     hotkey_back.clone(),
                //     hotkey_close.clone(),
                // )),
                CfgCommand::Position { hotkey, modifier } => Box::new(SavePosition::new(
                    chains.world_position.clone(),
                    hotkey.clone(),
                    modifier.clone(),
                )),
                CfgCommand::CycleSpeed {
                    cycle_speed,
                    hotkey,
                } => Box::new(CycleSpeed::new(
                    cycle_speed,
                    chains.animation_speed.clone(),
                    hotkey.clone(),
                )),
                CfgCommand::Runes { amount, hotkey } => Box::new(Runes::new(
                    *amount,
                    chains.runes.clone(),
                    hotkey.clone(),
                )),
                CfgCommand::Quitout { hotkey } => Box::new(Quitout::new(
                    chains.quitout.clone(),
                    hotkey.clone(),
                )),
                _ => todo!()
            })
            .collect()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            settings: Settings {
                log_level: LevelFilterSerde(LevelFilter::Debug),
                display: KeyState::new(util::get_key_code("0").unwrap()),
            },
            commands: Vec::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct FlagSpec {
    label: String,
    getter: fn(&Pointers) -> &Bitflag<u8>,
}

impl std::fmt::Debug for FlagSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FlagSpec {{ label: {:?} }}", self.label)
    }
}

impl FlagSpec {
    fn new(label: &str, getter: fn(&Pointers) -> &Bitflag<u8>) -> FlagSpec {
        FlagSpec {
            label: label.to_string(),
            getter,
        }
    }
}

impl TryFrom<String> for FlagSpec {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        macro_rules! flag_spec {
            ($x:expr, [ $( ($flag_name:ident, $flag_label:expr), )* ]) => {
                match $x {
                    $(stringify!($flag_name) => Ok(FlagSpec::new($flag_label, |c| &c.$flag_name)),)*
                    e => Err(format!("\"{}\" is not a valid flag specifier", e)),
                }
            }
        }
        flag_spec!(value.as_str(), [
            (one_shot, "One shot"),
            (no_damage, "All no damage"),
            (no_dead, "No death"),
            (no_hit, "No hit"),
            (no_goods_consume, "Inf consumables"),
            (no_stamina_consume, "Inf stamina"),
            (no_fp_consume, "Inf focus"),
            (no_ashes_of_war_fp_consume, "Inf focus (AoW)"),
            (no_arrows_consume, "Inf arrows"),
            (no_attack, "No attack"),
            (no_move, "No move"),
            (no_update_ai, "No update AI"),
            (gravity, "Gravity"),
            (display_stable_pos, "Display stable position"),
            (weapon_hitbox1, "Weapon hitbox #1"),
            (weapon_hitbox2, "Weapon hitbox #2"),
            (weapon_hitbox3, "Weapon hitbox #3"),
            (hitbox_high, "High world hitbox"),
            (hitbox_low, "Low world hitbox"),
            (hitbox_character, "Character hitbox"),
            (field_area_direction, "Direction HUD"),
            (field_area_altimeter, "Altimeter HUD"),
            (field_area_compass, "Compass HUD"),
            (show_map, "Show/hide map"),
            (show_chr, "Show/hide character"),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn test_parse() {
        println!(
            "{:?}",
            toml::from_str::<toml::Value>(include_str!("../jdsd_er_practice_tool.toml"))
        );
        println!(
            "{:?}",
            Config::parse(include_str!("../jdsd_er_practice_tool.toml"))
        );
    }

    #[test]
    fn test_parse_errors() {
        println!(
            "{:#?}",
            Config::parse(
                r#"commands = [ { boh = 3 } ]
                [settings]
                log_level = "DEBUG"
                "#
            )
        );
    }
}