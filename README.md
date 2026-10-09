# mvtcurl

[![Test](https://github.com/notfounds/mvtcurl/actions/workflows/test.yml/badge.svg)](https://github.com/notfounds/mvtcurl/actions/workflows/test.yml)
[![Release](https://github.com/notfounds/mvtcurl/actions/workflows/release.yml/badge.svg)](https://github.com/notfounds/mvtcurl/actions/workflows/release.yml)

`mvtcurl` は、[Mapbox Vector Tile（MVT）](https://github.com/mapbox/vector-tile-spec) 形式のデータを取得し、JSON 形式に変換する Rust 製の CLI ツールです。

## 機能

- URL から MVT タイルを取得
- タイル座標のプレースホルダー対応（`{z}/{x}/{y}`）
- HTTP ヘッダーの追加可能
- ローカルファイル（`file://`）・標準入力（`-`）からの読み込み
- gzip 圧縮されたタイルの自動展開
- HTTP ステータスが 2xx 以外の場合はエラー終了

## インストール

### プリビルドバイナリ（推奨）

[GitHub Releases](https://github.com/notfounds/mvtcurl/releases) から最新のバイナリをダウンロードできます。

```bash
# Linux (x86_64)
curl -L https://github.com/notfounds/mvtcurl/releases/latest/download/mvtcurl-linux-x86_64.tar.gz | tar xz

# macOS (Apple Silicon)
curl -L https://github.com/notfounds/mvtcurl/releases/latest/download/mvtcurl-macos-aarch64.tar.gz | tar xz

# macOS (Intel)
curl -L https://github.com/notfounds/mvtcurl/releases/latest/download/mvtcurl-macos-x86_64.tar.gz | tar xz

# Windows
# mvtcurl-windows-x86_64.zip をダウンロードして展開
```

### ソースからビルド

```bash
# リポジトリをクローン
git clone https://github.com/notfounds/mvtcurl.git
cd mvtcurl

# ビルド
cargo build --release

# バイナリは target/release/mvtcurl に作成されます
```

## 使い方

### 基本的な使い方

```bash
# MVT タイル を取得して JSON に変換
mvtcurl "https://example.com/tiles/14/14551/6449.mvt"
```

### タイル座標のプレースホルダーを使用

```bash
# {z}/{x}/{y} プレースホルダーを使用
mvtcurl "https://example.com/tiles/{z}/{x}/{y}.mvt" --zoom 14 --x 14551 --y 6449
```

### 事前定義された位置を使用

```bash
# 東京駅のタイルを取得（ズームレベル14）
mvtcurl "https://example.com/tiles/{z}/{x}/{y}.mvt" --tokyo --zoom 14

# 富士山頂上のタイルを取得（ズームレベル10）
mvtcurl "https://example.com/tiles/{z}/{x}/{y}.mvt" --fuji --zoom 10
```

### 緯度経度を指定

```bash
# 指定した緯度経度を含むタイルを取得（ズームレベル14）
mvtcurl "https://example.com/tiles/{z}/{x}/{y}.mvt" --zoom 14 --latitude 35.681236 --longitude 139.767125
```

### コンパクト出力

```bash
# 改行やインデントなしのコンパクトな JSON 出力
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" --compact
```

### カスタムHTTPヘッダーを追加

```bash
# API キーなどのカスタムヘッダーを追加
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" \
  --header "Authorization: Bearer YOUR_TOKEN" \
  --header "User-Agent: MyApp/1.0"
```

### ローカルファイル・標準入力から読み込む

```bash
# ローカルファイル（gzip 圧縮されていても自動で展開）
mvtcurl "file:///path/to/tiles/{z}/{x}/{y}.pbf" -z 14 -x 14551 -y 6449

# 標準入力
cat tile.mvt | mvtcurl -
```

### 出力先・出力形式

```bash
# JSON をファイルに保存
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" -o tile.json

# JSON に変換せず MVT バイナリ（gzip 展開済み）を保存
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" --raw -o tile.mvt
```

### レイヤーを絞り込む

```bash
# road レイヤーだけを出力（複数回指定可）
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" --layer road --layer building
```

存在しないレイヤー名を指定するとエラー終了し、タイルに含まれるレイヤー名の一覧を表示します。

### タイルの概要を表示

```bash
# レイヤーごとの地物数・ジオメトリ種別ごとの件数・属性キーの一覧を表示
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" --summary
```

```json
{
  "layers": [
    {
      "name": "road",
      "features": 3,
      "geometry_types": { "LineString": 2, "Point": 1 },
      "keys": ["class", "name", "oneway"]
    }
  ]
}
```

`--layer` と組み合わせると、絞り込んだレイヤーだけを集計します。

### リクエスト・レスポンスの詳細を表示

```bash
# 送信したリクエストと受け取ったレスポンスのヘッダーを標準エラー出力に表示
mvtcurl "https://example.com/tiles/14/14551/6449.mvt" -v
```

## オプション

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--zoom` | `-z` | ズームレベル（`{z}` プレースホルダー用） |
| `--x` | `-x` | X座標（`{x}` プレースホルダー用） |
| `--y` | `-y` | Y座標（`{y}` プレースホルダー用） |
| `--tokyo` | - | 東京駅の座標を使用（`--zoom` 必須） |
| `--fuji` | - | 富士山頂上の座標を使用（`--zoom` 必須） |
| `--latitude` | - | 緯度を指定（`--longitude`・`--zoom` 必須） |
| `--longitude` | - | 経度を指定（`--latitude`・`--zoom` 必須） |
| `--compact` | `-c` | コンパクトなJSON出力 |
| `--header` | `-H` | カスタムHTTPヘッダーを追加（形式: `'Name: Value'`） |
| `--output` | `-o` | 標準出力の代わりにファイルへ書き出す |
| `--raw` | - | JSON に変換せず MVT バイナリ（gzip 展開済み）を出力（`--compact` とは併用不可） |
| `--layer` | `-l` | 指定したレイヤーだけを出力（複数回指定可、`--raw` とは併用不可） |
| `--summary` | - | レイヤーごとの地物数・ジオメトリ種別ごとの件数・属性キーの一覧を出力（`--raw` とは併用不可） |
| `--verbose` | `-v` | リクエスト・レスポンスの詳細を標準エラー出力に表示 |


### 事前定義座標

- 東京駅: 緯度 35.681236, 経度 139.767125
- 富士山頂上: 緯度 35.360556, 経度 138.727778

## ライセンス

MIT License © 2026 Iori Ikeda

## 貢献

バグ報告や機能リクエストは、GitHubのIssueでお願いします。

## 参考リンク

- [Mapbox Vector Tile Specification](https://github.com/mapbox/vector-tile-spec)
- [Protocol Buffers](https://developers.google.com/protocol-buffers)
