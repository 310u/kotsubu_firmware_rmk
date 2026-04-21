use embassy_time::Duration;
use rmk::combo::{Combo, ComboConfig};
use rmk::config::{BehaviorConfig, CombosConfig, Hand, MorsesConfig, PositionalConfig};
use rmk::types::action::{KeyAction, MorseMode, MorseProfile};
use rmk::types::modifier::ModifierCombination;
use rmk::{a, k, kbctrl, layer, ltp, mo, mtp, shifted, user};

pub const ROW: usize = 4;
pub const COL: usize = 10;
pub const NUM_LAYER: usize = 4;

const BASE: u8 = 0;
const LOWER: u8 = 1;

const HOME_ROW_PROFILE: MorseProfile =
    MorseProfile::new(Some(true), Some(MorseMode::Normal), None, None);
const THUMB_PROFILE: MorseProfile =
    MorseProfile::new(None, Some(MorseMode::Normal), None, None);

const HRM_A: KeyAction = mtp!(A, ModifierCombination::LCTRL, HOME_ROW_PROFILE);
const HRM_Z: KeyAction = mtp!(Z, ModifierCombination::LSHIFT, HOME_ROW_PROFILE);
const HRM_X: KeyAction = mtp!(X, ModifierCombination::LGUI, HOME_ROW_PROFILE);
const HRM_C: KeyAction = mtp!(C, ModifierCombination::LALT, HOME_ROW_PROFILE);
const HRM_COMMA: KeyAction = mtp!(Comma, ModifierCombination::RALT, HOME_ROW_PROFILE);
const HRM_DOT: KeyAction = mtp!(Dot, ModifierCombination::RCTRL, HOME_ROW_PROFILE);
const HRM_SLASH: KeyAction = mtp!(Slash, ModifierCombination::RSHIFT, HOME_ROW_PROFILE);

const LT_SPACE: KeyAction = ltp!(1, Space, THUMB_PROFILE);
const LT_BSPC: KeyAction = ltp!(2, Backspace, THUMB_PROFILE);

#[rustfmt::skip]
pub const fn get_default_keymap() -> [[[KeyAction; COL]; ROW]; NUM_LAYER] {
    [
        layer!([
            [k!(Q),    k!(W), k!(E), k!(R), k!(T),    k!(Y), k!(U), k!(I),    k!(O),   k!(P)],
            [HRM_A,    k!(S), k!(D), k!(F), k!(G),    k!(H), k!(J), k!(K),    k!(L),   k!(Enter)],
            [HRM_Z,  HRM_X, HRM_C, k!(V), k!(B),    k!(N), k!(M), HRM_COMMA, HRM_DOT, HRM_SLASH],
            [a!(No), a!(No), a!(No), a!(No), LT_SPACE, LT_BSPC, a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [k!(AudioVolDown), k!(Home), k!(Up),   k!(PageUp), k!(AudioVolUp), k!(Kp7), k!(Kp8), k!(Kp9),    k!(KpSlash),   k!(KpAsterisk)],
            [a!(Transparent),  k!(Left), k!(Down), k!(Right),  k!(AudioMute),  k!(Kp4), k!(Kp5), k!(Kp6),    k!(KpMinus),   k!(KpPlus)],
            [a!(Transparent),  k!(End),  k!(Insert), k!(PageDown), k!(Kp0),    k!(Kp1), k!(Kp2), k!(Kp3),    k!(KpDot),     k!(KpEqual)],
            [a!(No), a!(No), a!(No), a!(No), a!(Transparent), k!(Delete), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [k!(Kc1), k!(Kc2), k!(Kc3),      k!(Kc4),        k!(Kc5), k!(Kc6),   k!(Kc7), k!(Kc8), k!(Kc9), k!(Kc0)],
            [k!(F1),  k!(F2),  k!(F3),       k!(F4),         k!(F5),  k!(F6),    k!(F7),  k!(F8),  k!(F9),  k!(Quote)],
            [a!(Transparent), k!(Minus), k!(Equal), k!(Backslash), k!(Grave), k!(CapsLock), k!(F10), k!(F11), k!(F12), k!(RShift)],
            [a!(No), a!(No), a!(No), a!(No), k!(Tab), a!(Transparent), a!(No), a!(No), a!(No), a!(No)]
        ]),
        layer!([
            [a!(No), kbctrl!(OutputUsb), a!(No), a!(No), a!(No), user!(0), user!(1), user!(2), user!(3), user!(5)],
            [a!(No), kbctrl!(OutputBluetooth), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), user!(6)],
            [a!(No), a!(No), kbctrl!(Bootloader), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), user!(4)],
            [a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No), a!(No)]
        ]),
    ]
}

pub fn get_behavior_config() -> BehaviorConfig {
    BehaviorConfig {
        combo: CombosConfig {
            combos: [
                Some(Combo::new(ComboConfig::new(
                    [k!(Q), k!(W)],
                    k!(Escape),
                    Some(BASE),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [k!(O), k!(I)],
                    k!(LeftBracket),
                    Some(BASE),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [k!(O), k!(P)],
                    k!(RightBracket),
                    Some(BASE),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [k!(L), k!(Enter)],
                    k!(Semicolon),
                    Some(BASE),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [LT_SPACE, LT_BSPC],
                    mo!(3),
                    Some(BASE),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [k!(N), LT_BSPC],
                    k!(Minus),
                    Some(BASE),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [k!(KpSlash), k!(KpAsterisk)],
                    shifted!(Backslash),
                    Some(LOWER),
                ))),
                Some(Combo::new(ComboConfig::new(
                    [k!(Kp9), k!(KpSlash)],
                    shifted!(Grave),
                    Some(LOWER),
                ))),
            ],
            timeout: Duration::from_millis(150),
        },
        morse: MorsesConfig {
            enable_flow_tap: true,
            prior_idle_time: Duration::from_millis(150),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn get_positional_config() -> PositionalConfig<ROW, COL> {
    PositionalConfig::new([
        [
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Right,
            Hand::Right,
            Hand::Right,
            Hand::Right,
            Hand::Right,
        ],
        [
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Right,
            Hand::Right,
            Hand::Right,
            Hand::Right,
            Hand::Right,
        ],
        [
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Left,
            Hand::Right,
            Hand::Right,
            Hand::Right,
            Hand::Right,
            Hand::Right,
        ],
        [
            Hand::Unknown,
            Hand::Unknown,
            Hand::Unknown,
            Hand::Unknown,
            Hand::Left,
            Hand::Right,
            Hand::Unknown,
            Hand::Unknown,
            Hand::Unknown,
            Hand::Unknown,
        ],
    ])
}
