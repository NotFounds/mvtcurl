use mvtcurl::*;

#[test]
fn test_lat_lon_to_tile_tokyo_station() {
    let latlon = LatLon::new(TOKYO_STATION_LAT, TOKYO_STATION_LON);
    let tile = latlon.to_tile_coord(14);
    assert_eq!(tile, TileCoord::new(14, 14552, 6451));
}

#[test]
fn test_lat_lon_to_tile_mt_fuji() {
    let latlon = LatLon::new(MT_FUJI_LAT, MT_FUJI_LON);
    let tile = latlon.to_tile_coord(10);
    assert_eq!(tile, TileCoord::new(10, 906, 404));
}

#[test]
fn test_decode_zigzag() {
    assert_eq!(decode_zigzag(0), 0);
    assert_eq!(decode_zigzag(1), -1);
    assert_eq!(decode_zigzag(2), 1);
    assert_eq!(decode_zigzag(3), -2);
    assert_eq!(decode_zigzag(4), 2);
}

#[test]
fn test_parse_command() {
    assert_eq!(parse_command(9), (1, 1));
    assert_eq!(parse_command(18), (2, 2));
    assert_eq!(parse_command(15), (7, 1));
}

#[test]
fn test_extent_normalize() {
    let extent = Extent::new(4096);
    assert_eq!(extent.normalize(0), 0.0);
    assert_eq!(extent.normalize(4096), 1.0);
    assert_eq!(extent.normalize(2048), 0.5);
}

#[test]
fn test_extent_default() {
    let extent = Extent::default();
    assert_eq!(extent.value(), DEFAULT_EXTENT);
}

#[test]
fn test_predefined_location_tokyo() {
    let location = PredefinedLocation::TokyoStation;
    let coords = location.coordinates();
    assert_eq!(coords.lat, TOKYO_STATION_LAT);
    assert_eq!(coords.lon, TOKYO_STATION_LON);
}

#[test]
fn test_predefined_location_fuji() {
    let location = PredefinedLocation::MtFuji;
    let coords = location.coordinates();
    assert_eq!(coords.lat, MT_FUJI_LAT);
    assert_eq!(coords.lon, MT_FUJI_LON);
}

#[test]
fn test_tile_coord_new() {
    let tile = TileCoord::new(14, 14551, 6449);
    assert_eq!(tile.z, 14);
    assert_eq!(tile.x, 14551);
    assert_eq!(tile.y, 6449);
}

#[test]
fn test_extent_value() {
    let extent = Extent::new(8192);
    assert_eq!(extent.value(), 8192);
}

#[test]
fn test_lat_lon_new() {
    let latlon = LatLon::new(35.6812, 139.7671);
    assert_eq!(latlon.lat, 35.6812);
    assert_eq!(latlon.lon, 139.7671);
}

#[test]
fn test_latlon_to_tile_coord_clamps_max_longitude() {
    let tile = LatLon::new(0.0, 180.0).to_tile_coord(2);
    assert_eq!(tile, TileCoord::new(2, 3, 2));
}

#[test]
fn test_decompress_if_gzipped_passes_through_plain_data() {
    let data = vec![0x1a, 0x00];
    assert_eq!(decompress_if_gzipped(data.clone()).unwrap(), data);
}

#[test]
fn test_decompress_if_gzipped_decompresses_gzip_data() {
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    let original = vec![0x1a, 0x00, 0x01, 0x02];
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&original).unwrap();
    let compressed = encoder.finish().unwrap();

    assert_eq!(decompress_if_gzipped(compressed).unwrap(), original);
}

#[test]
fn test_decompress_if_gzipped_fails_on_broken_gzip() {
    assert!(decompress_if_gzipped(vec![0x1f, 0x8b, 0x00]).is_err());
}

/// 指定したステータス行を 1 回だけ返す HTTP サーバーを立てて URL を返す
fn serve_once(status_line: &'static str, body: &'static [u8]) -> String {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let header = format!(
            "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            status_line,
            body.len()
        );
        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(body).unwrap();
    });
    format!("http://{}/tile.mvt", addr)
}

#[test]
fn test_fetch_mvt_returns_body_on_success() {
    let url = serve_once("200 OK", b"\x1a\x00");
    assert_eq!(fetch_mvt(&url, &[], false).unwrap(), b"\x1a\x00");
}

#[test]
fn test_fetch_mvt_fails_on_http_error_status() {
    let url = serve_once("404 Not Found", b"not found");
    let err = fetch_mvt(&url, &[], false).unwrap_err();
    assert!(err.to_string().contains("404"), "{}", err);
}
