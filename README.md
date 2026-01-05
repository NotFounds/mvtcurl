# mvtcurl

`mvtcurl` は、[Mapbox Vector Tile（MVT）](https://github.com/mapbox/vector-tile-spec) 形式のデータを取得し、JSON 形式に変換する Rust 製の CLI ツールです。

## 機能

- URL から MVT タイルを取得
- タイル座標のプレースホルダー対応（`{z}/{x}/{y}`）
- HTTP ヘッダーの追加可能

## インストール

### ビルド

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

## オプション

| オプション | 短縮形 | 説明 |
|-----------|--------|------|
| `--zoom` | `-z` | ズームレベル（`{z}` プレースホルダー用） |
| `--x` | `-x` | X座標（`{x}` プレースホルダー用） |
| `--y` | `-y` | Y座標（`{y}` プレースホルダー用） |
| `--tokyo` | - | 東京駅の座標を使用（`--zoom` 必須） |
| `--fuji` | - | 富士山頂上の座標を使用（`--zoom` 必須） |
| `--compact` | `-c` | コンパクトなJSON出力 |
| `--header` | `-H` | カスタムHTTPヘッダーを追加（形式: `'Name: Value'`） |


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
