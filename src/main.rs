use anyhow::{Context, Result};
use clap::Parser;
use mvtcurl::{
    LatLon, PredefinedLocation, TileCoord, decompress_if_gzipped, fetch_mvt, mvt_to_json,
};
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;

/// Web メルカトルで表現できる緯度の上限
const MAX_LATITUDE: f64 = 85.051_128_78;

#[derive(Parser)]
#[command(name = "mvtcurl")]
#[command(about = "Fetch MVT (Mapbox Vector Tile) and convert to JSON", long_about = None)]
struct Cli {
    #[arg(
        help = "URL of the MVT tile to fetch (supports {z}/{x}/{y} placeholders). Use file://PATH for a local file or - for stdin"
    )]
    url: String,

    #[arg(short, long, help = "Output compact JSON instead of pretty-printed")]
    compact: bool,

    #[arg(
        long,
        conflicts_with = "compact",
        help = "Output the MVT binary (gzip decompressed) instead of JSON"
    )]
    raw: bool,

    #[arg(
        short,
        long = "layer",
        value_name = "NAME",
        conflicts_with = "raw",
        help = "Output only the named layer (can be repeated)"
    )]
    layers: Vec<String>,

    #[arg(
        long,
        conflicts_with = "raw",
        help = "Output feature counts, geometry types and property keys per layer instead of features"
    )]
    summary: bool,

    #[arg(short, long, help = "Write output to FILE instead of stdout")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Print request/response details to stderr")]
    verbose: bool,

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

    #[arg(
        long,
        allow_negative_numbers = true,
        requires_all = ["longitude", "zoom"],
        conflicts_with_all = ["x", "y", "tokyo", "fuji"],
        help = "Latitude for tile calculation (requires --longitude and --zoom)"
    )]
    latitude: Option<f64>,

    #[arg(
        long,
        allow_negative_numbers = true,
        requires_all = ["latitude", "zoom"],
        conflicts_with_all = ["x", "y", "tokyo", "fuji"],
        help = "Longitude for tile calculation (requires --latitude and --zoom)"
    )]
    longitude: Option<f64>,

    #[arg(
        short = 'H',
        long = "header",
        help = "Add custom HTTP header (format: 'Name: Value')"
    )]
    headers: Vec<String>,
}

fn build_url(cli: &Cli) -> Result<String> {
    let mut url = cli.url.clone();

    if !url.contains("{z}") && !url.contains("{x}") && !url.contains("{y}") {
        return Ok(url);
    }

    let zoom = if cli.tokyo || cli.fuji {
        cli.zoom
            .context("--zoom is required when using --tokyo or --fuji")?
    } else {
        cli.zoom.unwrap_or(0)
    };

    let tile_coord = if let (Some(lat), Some(lon)) = (cli.latitude, cli.longitude) {
        if !(-MAX_LATITUDE..=MAX_LATITUDE).contains(&lat) {
            anyhow::bail!(
                "--latitude must be between -{} and {}",
                MAX_LATITUDE,
                MAX_LATITUDE
            );
        }
        if !(-180.0..=180.0).contains(&lon) {
            anyhow::bail!("--longitude must be between -180 and 180");
        }
        LatLon::new(lat, lon).to_tile_coord(zoom)
    } else if cli.tokyo {
        let location = PredefinedLocation::TokyoStation;
        location.coordinates().to_tile_coord(zoom)
    } else if cli.fuji {
        let location = PredefinedLocation::MtFuji;
        location.coordinates().to_tile_coord(zoom)
    } else {
        let x = cli.x.unwrap_or(0);
        let y = cli.y.unwrap_or(0);
        TileCoord::new(zoom, x, y)
    };

    url = url.replace("{z}", &tile_coord.z.to_string());
    url = url.replace("{x}", &tile_coord.x.to_string());
    url = url.replace("{y}", &tile_coord.y.to_string());

    Ok(url)
}

/// URL のスキームに応じて HTTP・ファイル・標準入力のいずれかから読み込む
fn read_input(url: &str, cli: &Cli) -> Result<Vec<u8>> {
    if url == "-" {
        if cli.verbose {
            eprintln!("* Reading from stdin");
        }
        let mut data = Vec::new();
        std::io::stdin()
            .read_to_end(&mut data)
            .context("Failed to read from stdin")?;
        return Ok(data);
    }
    if let Some(path) = url.strip_prefix("file://") {
        if cli.verbose {
            eprintln!("* Reading file {}", path);
        }
        return std::fs::read(path).with_context(|| format!("Failed to read file: {}", path));
    }
    fetch_mvt(url, &cli.headers, cli.verbose)
}

fn to_json<T: serde::Serialize>(value: &T, compact: bool) -> Result<String> {
    Ok(if compact {
        serde_json::to_string(value)?
    } else {
        serde_json::to_string_pretty(value)?
    })
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.raw && cli.output.is_none() && std::io::stdout().is_terminal() {
        anyhow::bail!(
            "Refusing to write binary output to a terminal. Use --output or redirect stdout"
        );
    }

    let url = build_url(&cli)?;
    let data = read_input(&url, &cli)?;
    let received_len = data.len();
    let data = decompress_if_gzipped(data)?;
    if cli.verbose && data.len() != received_len {
        eprintln!(
            "* Decompressed gzip data ({} -> {} bytes)",
            received_len,
            data.len()
        );
    }

    let output = if cli.raw {
        data
    } else {
        let mut tile_data = mvt_to_json(&data)?;
        if !cli.layers.is_empty() {
            tile_data = tile_data.filter_layers(&cli.layers)?;
        }
        let mut json = if cli.summary {
            to_json(&tile_data.summary(), cli.compact)?
        } else {
            to_json(&tile_data, cli.compact)?
        };
        json.push('\n');
        json.into_bytes()
    };

    match &cli.output {
        Some(path) => std::fs::write(path, &output)
            .with_context(|| format!("Failed to write output file: {}", path.display()))?,
        None => std::io::stdout()
            .write_all(&output)
            .context("Failed to write to stdout")?,
    }

    Ok(())
}
