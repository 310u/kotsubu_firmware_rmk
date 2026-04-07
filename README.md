# 星見薄荷・小粒 RMK

「星見薄荷・小粒」向けに RMK へ移植したキーボードファームウェアです。`Seeed Studio XIAO nRF52840` を使用します。

## 機能 (Features)

- **Bluetooth & USB のデュアル接続**: `Seeed Studio XIAO nRF52840` 上で BLE と USB の両方を使えます。無線キーボードとして常用しつつ、USB 接続時は有線デバッグやファーム更新の導線も確保しています。
- **4プロファイルのマルチペアリング**: BLE プロファイルを `4つ` 保持でき、PC・タブレット・スマホなど複数ホストを使い分けられます。`BT0` から `BT3` で特定スロットへ直接切り替え、`NEXT_BT` / `PREV_BT` で順送り、`CLR_BT` で現在スロットのボンド情報削除ができます。
- **Vial / VIA 対応**: ブラウザの Vial Web などから、ファームを書き直さずにキーマップを即時変更できます。BLE プロファイル切り替え系のカスタムキーコードも `vial.json` に定義済みで、必要に応じて任意レイヤーへ再配置できます。
- **ステータス表示 LED（RGB LED Widget）**: XIAO BLE 本体のフルカラー LED で、起動直後のバッテリー残量、BLE の接続待ち、接続成立したプロファイル、USB 接続イベント、低バッテリー警告、スリープ状態を色と点滅パターンで表現します。
- **省電力動作**: 打鍵が止まるとバッテリー測定周期と LED 表示を抑え、さらに長時間無操作なら deep sleep へ移行します。挙動の詳細は下の `Power Behavior` を参照してください。
- **ドラッグ＆ドロップでの書き込み**: UF2 ブートローダーに対応しており、USB ドライブとして認識させた本体に `kotsubu-rmk.uf2` をコピーするだけで簡単に更新できます。

## RGB LED Widget の表示ルール

オンボード RGB LED は、接続状態、選択中の BLE プロファイル、バッテリー警告、スリープ状態を表示します。通常待機中は消灯し、状態変化や警告時にだけ点灯または点滅します。

| 状態 | 色 | 点灯パターン | 意味 |
| --- | --- | --- | --- |
| 起動直後のバッテリー表示 | 緑 / 黄 / 赤 | 2秒間点灯 | 初回バッテリー測定結果を表示。`50%以上 = 緑`、`20〜49% = 黄`、`19%以下 = 赤` |
| BLE 接続待ち（Advertising） | プロファイル色 | `200ms` 点灯 + `800ms` 消灯の点滅 | 現在の BLE プロファイルでペアリング待ち・再接続待ちであることを表示 |
| BLE 接続成立直後 | プロファイル色 | 3秒間点灯 | どのプロファイルで接続されたかを確認しやすくするための表示 |
| USB 接続イベント | 白 | 2秒間点灯 | USB 側の接続変化を通知 |
| 低バッテリー警告 | 赤 | 1.5秒間点灯後、`200ms` 点灯 + `800ms` 消灯の点滅 | バッテリー残量 `20%未満` の警告 |
| スリープ中 | 消灯 | 常時消灯 | 省電力のため LED 表示を止める |
| 通常待機中 | 消灯 | 常時消灯 | 接続済みで特別なイベントがない通常状態 |

BLE プロファイル色の対応は次の通りです。

- `Profile 0`: 赤
- `Profile 1`: 緑
- `Profile 2`: 黄
- `Profile 3`: 青

## BLE プロファイル運用メモ

- `keyboard.toml` で BLE プロファイル数を `4` に設定しています。
- 1つのプロファイルは「1台のホストとのボンド情報」を持つスロットとして使います。
- 既に別の機器と結びついているプロファイルへ新しい機器をつなぎたい場合は、先に `CLR_BT` で現在スロットのボンド情報を消すと切り替えがスムーズです。
- `vial.json` には `SWITCH` も定義されているため、Vial 上で「USB / BLE のデフォルト出力をトグルするキー」を後から割り当てることもできます。

## Power Behavior

このファームウェアには、無操作時の省電力段階が `2段階` あります。  
前半は「接続を維持したまま、裏で消費を落とす」ための軽い sleep、後半は `System OFF` に入る deep sleep です。

- キー入力が止まると、matrix scan は常時ポーリングを続けず、キー入力待ち中心の待機に入ります。これは明示的な sleep タイマーより前から効いている内部的な節電です。
- 無操作 `5分` で、ファームウェア内部の sleep 状態に入ります。この段階では BLE 接続は維持したまま、RGB LED を消灯し、バッテリー測定周期を `300秒` に落とします。
- 無操作 `30分` で `System OFF` に入り、deep sleep へ移行します。ここでは BLE 接続は切れ、次にキーを押したときは「スリープから続きで復帰」ではなく `cold boot` として起動し直します。
- deep sleep を止める条件は「USB ケーブルが刺さっていること」ではなく、「USB がホストから実際に `configured` されていること」です。つまり、BLE 接続で使っていて、USB は給電だけの状態なら deep sleep の対象になります。
- deep sleep の確認は `probe-rs run` のようなデバッグ接続なしで行うのがおすすめです。デバッグプローブ接続中は、実機単体と同じように sleep しないように見えることがあります。

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

## License

This project is licensed under either of the following, at your option:

- Apache License, Version 2.0 ([LICENSE-APACHE](/home/satoyu/ドキュメント/git/kotsubu_firmware_rmk/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](/home/satoyu/ドキュメント/git/kotsubu_firmware_rmk/LICENSE-MIT))
