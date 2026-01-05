use anyhow::{Context, Result};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod vector_tile {
    include!(concat!(env!("OUT_DIR"), "/vector_tile.rs"));
}

pub const DEFAULT_EXTENT: u32 = 4096;
pub const TOKYO_STATION_LAT: f64 = 35.681236;
pub const TOKYO_STATION_LON: f64 = 139.767125;
pub const MT_FUJI_LAT: f64 = 35.360556;
pub const MT_FUJI_LON: f64 = 138.727778;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCoord {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

impl TileCoord {
    pub fn new(z: u32, x: u32, y: u32) -> Self {
        Self { z, x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent(u32);

impl Extent {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn normalize(&self, value: i32) -> f64 {
        value as f64 / self.0 as f64
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for Extent {
    fn default() -> Self {
        Self(DEFAULT_EXTENT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    pub fn to_tile_coord(&self, zoom: u32) -> TileCoord {
        let n = 2_f64.powi(zoom as i32);
        let x = ((self.lon + 180.0) / 360.0 * n).floor() as u32;
        let lat_rad = self.lat.to_radians();
        let y = ((1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f64::consts::PI) / 2.0 * n)
            .floor() as u32;
        TileCoord::new(zoom, x, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PredefinedLocation {
    TokyoStation,
    MtFuji,
}

impl PredefinedLocation {
    pub fn coordinates(&self) -> LatLon {
        match self {
            Self::TokyoStation => LatLon::new(TOKYO_STATION_LAT, TOKYO_STATION_LON),
            Self::MtFuji => LatLon::new(MT_FUJI_LAT, MT_FUJI_LON),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoJsonFeature {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub geometry: GeoJsonGeometry,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeoJsonGeometry {
    #[serde(rename = "type")]
    pub type_: String,
    pub coordinates: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub extent: u32,
    pub version: u32,
    pub features: Vec<GeoJsonFeature>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TileData {
    pub layers: Vec<Layer>,
}

/// MVTタイルをフェッチして、生のバイト列として返す
///
/// # Arguments
/// * `url` - MVTタイルのURL
/// * `headers` - カスタムHTTPヘッダーのリスト（"Name: Value"形式）
///
/// # Errors
/// HTTPリクエストが失敗した場合やレスポンスの読み取りに失敗した場合
pub fn fetch_mvt(url: &str, headers: &[String]) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::new();
    let mut request = client.get(url);

    for header in headers {
        let parts: Vec<&str> = header.splitn(2, ':').collect();
        if parts.len() == 2 {
            let name = parts[0].trim();
            let value = parts[1].trim();
            request = request.header(name, value);
        } else {
            anyhow::bail!("Invalid header format: '{}'. Expected 'Name: Value'", header);
        }
    }

    let response = request
        .send()
        .context("Failed to fetch URL")?
        .bytes()
        .context("Failed to read response body")?;
    Ok(response.to_vec())
}

/// ジグザグエンコーディングをデコード
///
/// ref: https://protobuf.dev/programming-guides/encoding/
pub fn decode_zigzag(value: u32) -> i32 {
    ((value >> 1) as i32) ^ (-((value & 1) as i32))
}

pub fn parse_command(cmd_int: u32) -> (u32, usize) {
    let cmd = cmd_int & 0x7;
    let count = (cmd_int >> 3) as usize;
    (cmd, count)
}

fn decode_geometry(
    geometry: &[u32],
    geom_type: vector_tile::tile::GeomType,
    extent: Extent,
) -> serde_json::Value {
    let mut coordinates = Vec::new();
    let mut x = 0i32;
    let mut y = 0i32;
    let mut i = 0;

    while i < geometry.len() {
        let cmd_int = geometry[i];
        let (cmd, count) = parse_command(cmd_int);
        i += 1;

        match cmd {
            1 => {
                for _ in 0..count {
                    if i + 1 >= geometry.len() {
                        break;
                    }
                    let dx = decode_zigzag(geometry[i]);
                    let dy = decode_zigzag(geometry[i + 1]);
                    x += dx;
                    y += dy;
                    i += 2;

                    let norm_x = extent.normalize(x);
                    let norm_y = extent.normalize(y);

                    match geom_type {
                        vector_tile::tile::GeomType::Point => {
                            coordinates.push(serde_json::json!([norm_x, norm_y]));
                        }
                        vector_tile::tile::GeomType::Linestring
                        | vector_tile::tile::GeomType::Polygon => {
                            if coordinates.is_empty() {
                                coordinates.push(serde_json::json!([]));
                            }
                            if let Some(last) = coordinates.last_mut() {
                                if let Some(arr) = last.as_array_mut() {
                                    arr.push(serde_json::json!([norm_x, norm_y]));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            2 => {
                for _ in 0..count {
                    if i + 1 >= geometry.len() {
                        break;
                    }
                    let dx = decode_zigzag(geometry[i]);
                    let dy = decode_zigzag(geometry[i + 1]);
                    x += dx;
                    y += dy;
                    i += 2;

                    let norm_x = extent.normalize(x);
                    let norm_y = extent.normalize(y);

                    if let Some(last) = coordinates.last_mut() {
                        if let Some(arr) = last.as_array_mut() {
                            arr.push(serde_json::json!([norm_x, norm_y]));
                        }
                    }
                }
            }
            7 => {}
            _ => {}
        }
    }

    match geom_type {
        vector_tile::tile::GeomType::Point if coordinates.len() == 1 => coordinates[0].clone(),
        vector_tile::tile::GeomType::Linestring if coordinates.len() == 1 => {
            coordinates[0].clone()
        }
        _ => serde_json::json!(coordinates),
    }
}

fn convert_value(value: &vector_tile::tile::Value) -> serde_json::Value {
    if let Some(v) = value.string_value.as_ref() {
        serde_json::Value::String(v.clone())
    } else if let Some(v) = value.float_value {
        serde_json::json!(v)
    } else if let Some(v) = value.double_value {
        serde_json::json!(v)
    } else if let Some(v) = value.int_value {
        serde_json::json!(v)
    } else if let Some(v) = value.uint_value {
        serde_json::json!(v)
    } else if let Some(v) = value.sint_value {
        serde_json::json!(v)
    } else if let Some(v) = value.bool_value {
        serde_json::Value::Bool(v)
    } else {
        serde_json::Value::Null
    }
}

/// MVTバイナリデータをJSONに変換
///
/// # Arguments
/// * `data` - MVTのバイナリデータ
///
/// # Errors
/// Protocol Buffersのデコードに失敗した場合
pub fn mvt_to_json(data: &[u8]) -> Result<TileData> {
    let tile = vector_tile::Tile::decode(data).context("Failed to decode MVT protobuf")?;

    let mut layers = Vec::new();

    for layer in tile.layers {
        let extent = layer.extent.map(Extent::new).unwrap_or_default();
        let version = layer.version;
        let mut features = Vec::new();

        for feature in layer.features {
            let geom_type = vector_tile::tile::GeomType::try_from(feature.r#type.unwrap_or(0))
                .unwrap_or(vector_tile::tile::GeomType::Unknown);

            let geometry_type = match geom_type {
                vector_tile::tile::GeomType::Point => "Point",
                vector_tile::tile::GeomType::Linestring => "LineString",
                vector_tile::tile::GeomType::Polygon => "Polygon",
                _ => "Unknown",
            };

            let coordinates = decode_geometry(&feature.geometry, geom_type, extent);

            let mut properties = HashMap::new();
            let tags = feature.tags;

            for i in (0..tags.len()).step_by(2) {
                if i + 1 < tags.len() {
                    let key_idx = tags[i] as usize;
                    let val_idx = tags[i + 1] as usize;

                    if key_idx < layer.keys.len() && val_idx < layer.values.len() {
                        let key = layer.keys[key_idx].clone();
                        let value = convert_value(&layer.values[val_idx]);
                        properties.insert(key, value);
                    }
                }
            }

            features.push(GeoJsonFeature {
                type_: "Feature".to_string(),
                id: feature.id,
                geometry: GeoJsonGeometry {
                    type_: geometry_type.to_string(),
                    coordinates,
                },
                properties,
            });
        }

        layers.push(Layer {
            name: layer.name,
            extent: extent.value(),
            version,
            features,
        });
    }

    Ok(TileData { layers })
}
