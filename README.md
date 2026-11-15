# minimal-clock

Windows 11 のタスクバー右側（通知領域のすぐ左）に、3都市の時刻を24時間表示で縦に並べる小さな時計です。
PCの日付と違う都市には `+1` / `−1` が付きます。

```
STO 01:05
LON 00:05
BOS 19:05 −1      ^ ⌔ ♪ ▭    9:05
                              2026/11/14
```

- 管理者権限・インストール不要。exe 1つ（約640KB）で動きます。
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

都市は3文字のコード（空港の都市コード）で、上から順に最大3つ書きます。夏時間は自動で反映されます。
アフリカと南米の都市は収録していません。

| 地域 | 使える都市 |
|---|---|
| ヨーロッパ | LON ロンドン / DUB ダブリン / LIS リスボン / PAR パリ / BRU ブリュッセル / AMS アムステルダム / FRA フランクフルト / BER ベルリン / MUC ミュンヘン / ZRH チューリッヒ / MIL ミラノ / ROM ローマ / MAD マドリード / BCN バルセロナ / VIE ウィーン / PRG プラハ / WAW ワルシャワ / BUD ブダペスト / CPH コペンハーゲン / STO ストックホルム / OSL オスロ / HEL ヘルシンキ / ATH アテネ / IST イスタンブール / IEV キーウ / MOW モスクワ |
| 中東・南アジア | DXB ドバイ / DOH ドーハ / RUH リヤド / TLV テルアビブ / THR テヘラン / KHI カラチ / DEL デリー / BOM ムンバイ / BLR ベンガルール / DAC ダッカ |
| 東南アジア・東アジア | BKK バンコク / HAN ハノイ / SGN ホーチミン / KUL クアラルンプール / SIN シンガポール / JKT ジャカルタ / MNL マニラ / HKG 香港 / TPE 台北 / SHA 上海 / BJS 北京 / SZX 深圳 / SEL ソウル / TYO 東京 |
| オセアニア・太平洋 | PER パース / ADL アデレード / BNE ブリスベン / SYD シドニー / MEL メルボルン / AKL オークランド / GUM グアム / HNL ホノルル |
| 北米 | ANC アンカレッジ / SEA シアトル / SFO サンフランシスコ / LAX ロサンゼルス / YVR バンクーバー / PHX フェニックス / DEN デンバー / DFW ダラス / HOU ヒューストン / CHI シカゴ / MEX メキシコシティ / ATL アトランタ / MIA マイアミ / WAS ワシントンD.C. / NYC ニューヨーク / BOS ボストン / YTO トロント / YMQ モントリオール |
| その他 | UTC 協定世界時 |

表示名を変えたいときは `OSA=TYO` のように「表示名=都市」と書きます（表示名は4文字まで）。

## 動作

- 時刻は分の変わり目ちょうどに書き換わります。
- タスクバーの位置、全画面アプリの有無、設定ファイルの変更は2秒ごとに確認します（Windows がほかの処理とまとめて実行できる設定にしています）。
- 描画に使うフォントやビットマップは描き終わるたびに解放するので、常駐させてもメモリは増えません。

## ビルド（macOS / Linux からのクロスビルド）

```sh
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64
cargo test                                              # 時差・日付ずれのテスト
cargo build --release --target x86_64-pc-windows-gnu
# → target/x86_64-pc-windows-gnu/release/minimal-clock.exe
```

exe に入れるタイムゾーンは `.cargo/config.toml` の `CHRONO_TZ_TIMEZONE_FILTER` で絞っています。
都市を追加するときは `src/model.rs` の `CITIES` とこの正規表現の両方に足してください（片方だけだと `cargo test` が失敗します）。

## 会社PCで動かないとき

- SmartScreen の「WindowsによってPCが保護されました」→「詳細情報」→「実行」。
- それでも起動しない場合は、AppLocker などで exe の実行自体が制限されている可能性があります。情報システム部門に確認してください。
