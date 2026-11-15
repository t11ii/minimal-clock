# minimal-clock

Windows 11 のタスクバー右側（通知領域のすぐ左）に、3都市の時刻を24時間表示で縦に並べる小さな時計です。
PCの日付と違う都市には `+1` / `−1` が付きます。

```
STO 01:05
LON 00:05
BOS 19:05 −1      ^ ⌔ ♪ ▭    9:05
                              2026/11/14
```

- 管理者権限・インストール不要。exe 1つ（約1.5MB）で動きます。
- explorer.exe には手を入れません（タスクバーの上に透明な小窓を重ねて表示しているだけです）。
- 全画面のアプリや動画の表示中は自動で隠れます。

## ダウンロード

[Releases](https://github.com/t11ii/minimal-clock/releases/latest) から `minimal-clock.exe` をダウンロードします。
exe のダウンロードがブロックされる場合は、同じページの zip を使ってください（中身は同じ exe です）。

Edge で「一般的にダウンロードされていません」と出たら、`…` →「保存」→「保持する」で保存できます。

## 使い方

1. `minimal-clock.exe` を好きなフォルダ（例: `%USERPROFILE%\Tools\minimal-clock\`）に置いて、ダブルクリックで起動します。
2. 初回起動時に、同じフォルダに設定ファイル `minimal-clock.ini` が作られます。
3. 時計を **右クリック** するとメニューが出ます。
   - 設定ファイルを開く（メモ帳で開きます。保存するとすぐ反映されます）
   - 位置を自動に戻す
   - Windows 起動時に開始（`HKCU\...\Run` に登録。管理者権限は不要です）
   - 終了
4. 時計を **左ドラッグ** すると左右に動かせます。位置は設定ファイルに保存されます。

## 設定（minimal-clock.ini）

```ini
cities = STO, LON, BOS
right_margin =
```

| ラベル | 都市 | タイムゾーン |
|---|---|---|
| STO | ストックホルム | Europe/Stockholm |
| LON | ロンドン | Europe/London |
| BOS | ボストン | America/New_York |
| SIN | シンガポール | Asia/Singapore |
| NYC | ニューヨーク | America/New_York |
| LAX | ロサンゼルス | America/Los_Angeles |
| TYO | 東京 | Asia/Tokyo |
| UTC | 協定世界時 | Etc/UTC |

一覧にない都市は `PAR=Europe/Paris` のように「ラベル=IANA名」で書けます。最大3つです。
夏時間は自動で反映されます。

## ビルド（macOS / Linux からのクロスビルド）

```sh
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64
cargo test                                              # 時差・日付ずれのテスト
cargo build --release --target x86_64-pc-windows-gnu
# → target/x86_64-pc-windows-gnu/release/minimal-clock.exe
```

## 会社PCで動かないとき

- SmartScreen の「WindowsによってPCが保護されました」→「詳細情報」→「実行」。
- それでも起動しない場合は、AppLocker などで exe の実行自体が制限されている可能性があります。情報システム部門に確認してください。
