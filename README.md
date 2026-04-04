# Kotsubu RMK

Kotsubu keyboard firmware ported to RMK for `Seeed Studio XIAO nRF52840`.

## 機能 (Features)

- **Bluetooth & USB有線接続**: 無線（BLE）と有線（USB）両方の接続にネイティブ対応。
- **マルチペアリング対応**: 最大4台までのデバイス（PCやスマホなど）を記憶でき、プロファイルの切り替えが可能。
- **Vial / VIA対応**: 専用ソフト不要で、ブラウザ上のVial Webなどからリアルタイムにキーマップや設定を書き換え可能。
- **ステータス表示LED（RGB LED Widget）**: XIAO BLE本体のフルカラーLEDを活用し、現在のBluetooth接続状況、選択中のプロファイル、およびバッテリー残量を光の色と点滅で直感的に表示。
- **ドラッグ＆ドロップでの書き込み**: UF2ブートローダーに対応しており、USBドライブとして認識させた本体にファームウェアのファイルをコピーするだけで簡単にアップデート可能。

## Prerequisites

### Common

```bash
PATH="$HOME/.cargo/bin:$PATH" rustup target add thumbv7em-none-eabihf
```

### For debug-probe flashing

Install `probe-rs`, then connect your SWD probe to the XIAO BLE.

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo install --locked probe-rs-tools
```

On Linux, you may also need the `probe-rs` udev rules before the probe becomes visible to the CLI.

### For USB bootloader flashing

This project already uses the Adafruit nRF52 bootloader layout in [memory.x](/home/satoyu/ドキュメント/git/kotsubu-firmware/kotsubu_firmware_rmk/memory.x#L4), so UF2 flashing is the easiest path if your board exposes a UF2 drive.

Install the helper tools once:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo install --locked cargo-binutils cargo-hex-to-uf2 flip-link
PATH="$HOME/.cargo/bin:$PATH" rustup component add llvm-tools
```

## Flashing

### 1. Flash with a debug probe

`cargo run --release` uses the runner from [.cargo/config.toml](/home/satoyu/ドキュメント/git/kotsubu-firmware/kotsubu_firmware_rmk/.cargo/config.toml#L1) and writes directly with `probe-rs`.

```bash
bash scripts/flash_probe.sh
```

If you prefer to run the raw command directly:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo run --release --bin kotsubu-rmk
```

If `probe-rs` says `No connected probes were found`, check the USB cable, SWD wiring, and the Linux udev setup for your debug probe first.

### 2. Build UF2 and copy it over USB

Build the UF2 firmware:

```bash
bash scripts/build_uf2.sh
```

This generates:

- `kotsubu-rmk.hex`
- `kotsubu-rmk.uf2`

Put the XIAO BLE into bootloader mode, wait for the UF2 USB drive to appear, then copy:

```bash
bash scripts/flash_uf2.sh
```

The copy task uses [scripts/copy_uf2.sh](/home/satoyu/ドキュメント/git/kotsubu-firmware/kotsubu_firmware_rmk/scripts/copy_uf2.sh), which looks for a mounted drive containing `INFO_UF2.TXT`.
If normal reset is not enough, press the XIAO reset button twice quickly to enter bootloader mode.

If you want to copy manually instead:

```bash
cp kotsubu-rmk.uf2 /path/to/UF2_DRIVE/
sync
```

If you prefer the RMK-style `cargo-make` task, [Makefile.toml](/home/satoyu/ドキュメント/git/kotsubu-firmware/kotsubu_firmware_rmk/Makefile.toml#L1) also provides `uf2`, `flash-probe`, and `flash-uf2`.

## Bootloader Notes

- This repository is currently configured for the Adafruit nRF52 UF2 bootloader layout:
  - `FLASH : ORIGIN = 0x00001000`
  - `RAM : ORIGIN = 0x20000008`
- If you erase the bootloader and want to flash the board as raw nRF52840 over SWD, switch [memory.x](/home/satoyu/ドキュメント/git/kotsubu-firmware/kotsubu_firmware_rmk/memory.x#L8) to the non-bootloader values first.

## Quick Reference

```bash
# Debug probe flash
bash scripts/flash_probe.sh

# Build UF2
bash scripts/build_uf2.sh

# Copy UF2 to the mounted bootloader drive
bash scripts/flash_uf2.sh
```
