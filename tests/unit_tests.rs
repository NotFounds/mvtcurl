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
