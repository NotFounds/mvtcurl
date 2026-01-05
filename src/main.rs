use anyhow::{Context, Result};
use clap::Parser;
use prost::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod vector_tile {
    include!(concat!(env!("OUT_DIR"), "/vector_tile.rs"));
}

#[derive(Parser)]
#[command(name = "mvtcurl")]
#[command(about = "Fetch MVT (Mapbox Vector Tile) and convert to JSON", long_about = None)]
struct Cli {
    #[arg(help = "URL of the MVT tile to fetch (supports {z}/{x}/{y} placeholders)")]
    url: String,

    #[arg(short, long, help = "Output compact JSON instead of pretty-printed")]
    compact: bool,

    #[arg(short = 'z', long, help = "Zoom level for {z} placeholder")]
    zoom: Option<u32>,

    #[arg(short = 'x', long, help = "X tile coordinate for {x} placeholder")]
    x: Option<u32>,

    #[arg(short = 'y', long, help = "Y tile coordinate for {y} placeholder")]
    y: Option<u32>,

    #[arg(long, help = "Use Tokyo Station coordinates (requires --zoom)")]
    tokyo: bool,

    #[arg(long, help = "Use Mt. Fuji summit coordinates (requires --zoom)")]
    fuji: bool,

    #[arg(short = 'H', long = "header", help = "Add custom HTTP header (format: 'Name: Value')")]
    headers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeoJsonFeature {
    #[serde(rename = "type")]
    type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    geometry: GeoJsonGeometry,
    properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeoJsonGeometry {
    #[serde(rename = "type")]
    type_: String,
    coordinates: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Layer {
    name: String,
    extent: u32,
    version: u32,
    features: Vec<GeoJsonFeature>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TileData {
    layers: Vec<Layer>,
}

fn fetch_mvt(url: &str, headers: &[String]) -> Result<Vec<u8>> {
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

fn decode_geometry(
    geometry: &[u32],
    geom_type: vector_tile::tile::GeomType,
    extent: u32,
) -> serde_json::Value {
    let mut coordinates = Vec::new();
    let mut x = 0i32;
    let mut y = 0i32;
    let mut i = 0;

    let extent_f64 = extent as f64;

    while i < geometry.len() {
        let cmd_int = geometry[i];
        let cmd = cmd_int & 0x7;
        let count = (cmd_int >> 3) as usize;
        i += 1;

        match cmd {
            1 => {
                for _ in 0..count {
                    if i + 1 >= geometry.len() {
                        break;
                    }
                    let dx = ((geometry[i] >> 1) as i32) ^ (-((geometry[i] & 1) as i32));
                    let dy = ((geometry[i + 1] >> 1) as i32) ^ (-((geometry[i + 1] & 1) as i32));
                    x += dx;
                    y += dy;
                    i += 2;

                    let norm_x = (x as f64) / extent_f64;
                    let norm_y = (y as f64) / extent_f64;

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
                    let dx = ((geometry[i] >> 1) as i32) ^ (-((geometry[i] & 1) as i32));
                    let dy = ((geometry[i + 1] >> 1) as i32) ^ (-((geometry[i + 1] & 1) as i32));
                    x += dx;
                    y += dy;
                    i += 2;

                    let norm_x = (x as f64) / extent_f64;
                    let norm_y = (y as f64) / extent_f64;

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

fn mvt_to_json(data: &[u8]) -> Result<TileData> {
    let tile = vector_tile::Tile::decode(data).context("Failed to decode MVT protobuf")?;

    let mut layers = Vec::new();

    for layer in tile.layers {
        let extent = layer.extent.unwrap_or(4096);
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
            extent,
            version,
            features,
        });
    }

    Ok(TileData { layers })
}

fn lat_lon_to_tile(lat: f64, lon: f64, zoom: u32) -> (u32, u32) {
    let n = 2_f64.powi(zoom as i32);
    let x = ((lon + 180.0) / 360.0 * n).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f64::consts::PI) / 2.0 * n)
        .floor() as u32;
    (x, y)
}

fn build_url(cli: &Cli) -> Result<String> {
    let mut url = cli.url.clone();

    if !url.contains("{z}") && !url.contains("{x}") && !url.contains("{y}") {
        return Ok(url);
    }

    let zoom = if cli.tokyo || cli.fuji {
        cli.zoom.context("--zoom is required when using --tokyo or --fuji")?
    } else {
        cli.zoom.unwrap_or(0)
    };

    let (x, y) = if cli.tokyo {
        lat_lon_to_tile(35.681236, 139.767125, zoom) // Tokyo Station
    } else if cli.fuji {
        lat_lon_to_tile(35.360556, 138.727778, zoom) // Mt. Fuji summit
    } else {
        let x = cli.x.unwrap_or(0);
        let y = cli.y.unwrap_or(0);
        (x, y)
    };

    url = url.replace("{z}", &zoom.to_string());
    url = url.replace("{x}", &x.to_string());
    url = url.replace("{y}", &y.to_string());

    Ok(url)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let url = build_url(&cli)?;
    let data = fetch_mvt(&url, &cli.headers)?;
    let tile_data = mvt_to_json(&data)?;

    let output = if cli.compact {
        serde_json::to_string(&tile_data)?
    } else {
        serde_json::to_string_pretty(&tile_data)?
    };

    println!("{}", output);

    Ok(())
}
