# Unified Firmware Policy

This document defines the behavior that should stay identical across
`mochiko52mk3_rmk` and `kotsubu_firmware_rmk`.

## Shared Policy

- BLE profile count: `5`
- BLE enabled: `true`
- Passkey entry enabled: `false`
- Passkey entry timeout: `120` seconds
- LED boot self-test enabled
- LED profile colors:
  - Profile 0: red
  - Profile 1: green
  - Profile 2: yellow
  - Profile 3: blue
  - Profile 4: magenta
- Deep sleep requires:
  - the firmware to already be in sleep state
  - no pressed keys
  - idle time >= deep sleep timeout
  - USB not configured by a host

## Allowed Board-Specific Differences

- Keymap and layer contents
- Matrix shape and scan implementation
- Pin assignments
- USB product identity
- Flash storage start address
- Optional pointing-device support

## Source of Truth

- Shared behavior constants live in `src/unified_policy.rs`
- Board-specific runtime constants live in `src/board_config.rs`
- `build.rs` validates that `keyboard.toml` and `Cargo.toml` still match the shared policy

## Porting Checklist For kotsubu_firmware_rmk

1. Add `src/unified_policy.rs`
2. Add `src/board_config.rs` with Kotsubu-specific IDs and storage address
3. Update `build.rs` to validate shared BLE and passkey settings
4. Update `src/power.rs` to use the shared deep sleep policy
5. Update `src/led_widget.rs` to use the shared LED policy
6. Update `src/main.rs` to read board-specific constants from `src/board_config.rs`
7. Align `keyboard.toml` and `Cargo.toml` with the shared BLE and passkey policy
