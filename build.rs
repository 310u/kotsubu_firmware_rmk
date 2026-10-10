//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.
//!
//! The build script also sets the linker flags to tell it which link script to use.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};

use const_gen::*;
use xz2::read::XzEncoder;

#[path = "src/unified_policy.rs"]
mod unified_policy;

fn main() {
    // Generate vial config at the root of project
    generate_vial_config();
    validate_unified_policy();

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // Specify linker arguments.

    // `--nmagic` is required if memory section addresses are not aligned to 0x10000,
    // for example the FLASH and RAM sections in your `memory.x`.
    // See https://github.com/rust-embedded/cortex-m-quickstart/pull/95
    println!("cargo:rustc-link-arg=--nmagic");

    // Set the linker script to the one provided by cortex-m-rt.
    println!("cargo:rustc-link-arg=-Tlink.x");

    // Set the extra linker script from defmt
    println!("cargo:rustc-link-arg=-Tdefmt.x");
}

fn validate_unified_policy() {
    let keyboard_toml = fs::read_to_string("keyboard.toml").expect("Cannot read keyboard.toml");
    let expected_profiles = unified_policy::BLE_PROFILE_COUNT.to_string();
    assert_toml_value(&keyboard_toml, "rmk", "ble_profiles_num", &expected_profiles);
    assert_toml_value(&keyboard_toml, "ble", "enabled", "true");
    assert_toml_value(
        &keyboard_toml,
        "ble",
        "passkey_entry",
        if unified_policy::PASSKEY_ENTRY_ENABLED {
            "true"
        } else {
            "false"
        },
    );
    let expected_timeout = unified_policy::PASSKEY_ENTRY_TIMEOUT_SECS.to_string();
    assert_toml_value(
        &keyboard_toml,
        "ble",
        "passkey_entry_timeout",
        &expected_timeout,
    );

    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Cannot read Cargo.toml");
    let has_passkey_feature = cargo_toml.contains("\"passkey_entry\"");
    if has_passkey_feature != unified_policy::PASSKEY_ENTRY_ENABLED {
        panic!(
            "Cargo.toml passkey_entry feature must match unified policy (expected {}, found {})",
            unified_policy::PASSKEY_ENTRY_ENABLED,
            has_passkey_feature
        );
    }
}

fn assert_toml_value(content: &str, section: &str, key: &str, expected: &str) {
    match find_toml_value(content, section, key) {
        Some(actual) if actual == expected => {}
        Some(actual) => panic!(
            "keyboard.toml [{}].{} must be {}, found {}",
            section, key, expected, actual
        ),
        None => panic!("keyboard.toml is missing [{}].{}", section, key),
    }
}

fn find_toml_value(content: &str, section: &str, key: &str) -> Option<String> {
    let mut current_section = String::new();

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        let Some((lhs, rhs)) = line.split_once('=') else {
            continue;
        };

        if current_section == section && lhs.trim() == key {
            return Some(rhs.trim().trim_matches('"').to_string());
        }
    }

    None
}

fn generate_vial_config() {
    // Generated vial config file
    println!("cargo:rerun-if-changed=vial.json");
    let out_file = Path::new(&env::var_os("OUT_DIR").unwrap()).join("config_generated.rs");

    let p = Path::new("vial.json");
    let mut content = String::new();
    match File::open(p) {
        Ok(mut file) => {
            file.read_to_string(&mut content)
                .expect("Cannot read vial.json");
        }
        Err(e) => println!("Cannot find vial.json {:?}: {}", p, e),
    };

    let vial_cfg = json::stringify(json::parse(&content).unwrap());
    let mut keyboard_def_compressed: Vec<u8> = Vec::new();
    XzEncoder::new(vial_cfg.as_bytes(), 6)
        .read_to_end(&mut keyboard_def_compressed)
        .unwrap();

    let keyboard_id: Vec<u8> = vec![0x6B, 0x6F, 0x74, 0x73, 0x75, 0x62, 0x75, 0x01];
    let const_declarations = [
        const_declaration!(pub VIAL_KEYBOARD_DEF = keyboard_def_compressed),
        const_declaration!(pub VIAL_KEYBOARD_ID = keyboard_id),
    ]
    .map(|s| "#[allow(clippy::redundant_static_lifetimes)]\n".to_owned() + s.as_str())
    .join("\n");
    fs::write(out_file, const_declarations).unwrap();
}
