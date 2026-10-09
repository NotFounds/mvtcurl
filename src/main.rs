use anyhow::{Context, Result};
use clap::Parser;
use mvtcurl::{LatLon, PredefinedLocation, TileCoord, fetch_mvt, mvt_to_json};

/// Web メルカトルで表現できる緯度の上限
const MAX_LATITUDE: f64 = 85.051_128_78;

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
