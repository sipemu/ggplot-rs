//! `annotation_map_tile()` (feature `tiles`) — a raster basemap from an XYZ
//! tile provider (OpenStreetMap and friends) behind `geom_sf` layers in Web
//! Mercator (Highcharts' `tiledwebmap`, ggspatial's `annotation_map_tile`).
//!
//! The crate does no networking: tiles come from a caller-supplied
//! [`TileFetcher`] (an HTTP client, a local cache directory, bytes bundled
//! for wasm or tests). Fetched tiles are cached per geom, so re-rendering
//! the same plot fetches nothing twice.
//!
//! Units: the layer reads the trained X/Y domain as
//! [`SfProjection::Mercator`](crate::spatial::SfProjection::Mercator) map
//! units (x = λ in radians, y = ln tan(π/4 + φ/2)), which map onto the
//! Web-Mercator tile grid exactly — project the `geom_sf` layers with
//! `SfProjection::Mercator` and add `coord_sf()`.

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::DrawBackend;
use crate::render::{Rect, RenderError};
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Supplies the encoded image (PNG or JPEG) of tile `z/x/y`, or `None`
/// when it isn't available (the tile is then left blank). Any
/// `Fn(u32, u32, u32) -> Option<Vec<u8>>` closure is a fetcher.
pub trait TileFetcher: Send + Sync {
    fn fetch(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>>;
}

impl<F: Fn(u32, u32, u32) -> Option<Vec<u8>> + Send + Sync> TileFetcher for F {
    fn fetch(&self, z: u32, x: u32, y: u32) -> Option<Vec<u8>> {
        self(z, x, y)
    }
}

/// An XYZ tile provider: its URL template (`{z}`, `{x}`, `{y}`, for the
/// fetcher's use — see [`TileSource::url`]), attribution and zoom range.
#[derive(Clone, Debug)]
pub struct TileSource {
    pub url_template: String,
    /// Shown in a corner via `annotation_attribution` (most providers'
    /// terms require it). Empty: none.
    pub attribution: String,
    pub min_zoom: u32,
    pub max_zoom: u32,
    /// Edge length of one tile, in pixels (256 for nearly every provider).
    pub tile_size: f64,
}

impl TileSource {
    /// A source with zoom levels 0–19 and 256 px tiles.
    pub fn xyz(url_template: impl Into<String>) -> Self {
        TileSource {
            url_template: url_template.into(),
            attribution: String::new(),
            min_zoom: 0,
            max_zoom: 19,
            tile_size: 256.0,
        }
    }

    pub fn attribution(mut self, text: impl Into<String>) -> Self {
        self.attribution = text.into();
        self
    }

    pub fn zoom_range(mut self, min: u32, max: u32) -> Self {
        self.min_zoom = min.min(max);
        self.max_zoom = max;
        self
    }

    /// The template with `{z}`/`{x}`/`{y}` filled in.
    pub fn url(&self, z: u32, x: u32, y: u32) -> String {
        self.url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
    }
}

/// Web-Mercator map units → fractional tile coordinates at zoom `z`.
pub fn tile_xy(z: u32, mx: f64, my: f64) -> (f64, f64) {
    let n = (1u64 << z) as f64;
    ((mx + PI) / (2.0 * PI) * n, (1.0 - my / PI) / 2.0 * n)
}

/// The map-unit rectangle `(x0, y_top, x1, y_bottom)` of tile `tx/ty` at
/// zoom `z`.
pub fn tile_bounds(z: u32, tx: i64, ty: i64) -> (f64, f64, f64, f64) {
    let n = (1u64 << z) as f64;
    let mx = |t: f64| t / n * 2.0 * PI - PI;
    let my = |t: f64| (1.0 - 2.0 * t / n) * PI;
    (
        mx(tx as f64),
        my(ty as f64),
        mx(tx as f64 + 1.0),
        my(ty as f64 + 1.0),
    )
}

/// Most tiles one layer draws; a coarser zoom is used beyond it.
const MAX_TILES: usize = 64;

type TileCache = Arc<Mutex<HashMap<(u32, u32, u32), Option<Arc<Vec<u8>>>>>>;

/// The basemap layer — see the module docs and
/// [`GGPlot::annotation_map_tile`](crate::GGPlot::annotation_map_tile).
#[derive(Clone)]
pub struct GeomMapTile {
    pub source: TileSource,
    fetcher: Arc<dyn TileFetcher>,
    /// A fixed zoom instead of the automatic one.
    pub zoom: Option<u32>,
    /// Pixels per tile pixel the automatic zoom aims for (1 = native
    /// resolution; 2 = sharper, four times the tiles).
    pub detail: f64,
    pub alpha: f64,
    cache: TileCache,
}

impl std::fmt::Debug for GeomMapTile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeomMapTile")
            .field("source", &self.source)
            .field("zoom", &self.zoom)
            .finish_non_exhaustive()
    }
}

impl GeomMapTile {
    pub fn new(source: TileSource, fetcher: impl TileFetcher + 'static) -> Self {
        GeomMapTile {
            source,
            fetcher: Arc::new(fetcher),
            zoom: None,
            detail: 1.0,
            alpha: 1.0,
            cache: Arc::default(),
        }
    }

    pub fn zoom(mut self, z: u32) -> Self {
        self.zoom = Some(z);
        self
    }

    pub fn detail(mut self, detail: f64) -> Self {
        self.detail = detail;
        self
    }

    pub fn alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    fn tile(&self, z: u32, x: u32, y: u32) -> Option<Arc<Vec<u8>>> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry((z, x, y))
            .or_insert_with(|| self.fetcher.fetch(z, x, y).map(Arc::new))
            .clone()
    }

    /// The zoom for a view `span_x` map units wide over `width_px` pixels:
    /// the level whose tiles come closest to their native size, clamped to
    /// the source's range and to at most [`MAX_TILES`] tiles.
    pub(crate) fn pick_zoom(&self, span: (f64, f64), size_px: (f64, f64)) -> u32 {
        let (lo, hi) = (self.source.min_zoom, self.source.max_zoom.min(30));
        let mut z = match self.zoom {
            Some(z) => z.clamp(lo, hi),
            None => {
                let px_per_unit = size_px.0 / span.0.max(1e-12);
                let n = 2.0 * PI * px_per_unit * self.detail.max(0.01) / self.source.tile_size;
                (n.max(1.0).log2().round() as u32).clamp(lo, hi)
            }
        };
        let count = |z: u32| {
            let n = (1u64 << z) as f64;
            let tiles = |span: f64| (span / (2.0 * PI) * n).ceil() + 1.0;
            tiles(span.0) * tiles(span.1)
        };
        while z > lo && count(z) > MAX_TILES as f64 {
            z -= 1;
        }
        z
    }
}

impl Geom for GeomMapTile {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let (Some(xs), Some(ys)) = (scales.get(&Aesthetic::X), scales.get(&Aesthetic::Y)) else {
            return Ok(());
        };
        let (Some((x0, x1)), Some((y0, y1))) = (xs.expanded_domain(), ys.expanded_domain()) else {
            return Ok(());
        };
        let plot_area = backend.plot_area();
        let view = coord
            .data_rect(&plot_area)
            .unwrap_or_else(|| plot_area.clone());
        let px = |mx: f64, my: f64| {
            coord.transform(
                (xs.map(&Value::Float(mx)), ys.map(&Value::Float(my))),
                &plot_area,
            )
        };
        let z = self.pick_zoom((x1 - x0, y1 - y0), (view.width, view.height));
        let n = 1i64 << z;
        let (tx0, ty0) = tile_xy(z, x0, y1);
        let (tx1, ty1) = tile_xy(z, x1, y0);
        let ty_range = (ty0.floor().max(0.0) as i64)..=(ty1.ceil() as i64 - 1).min(n - 1);
        for ty in ty_range {
            for tx in tx0.floor() as i64..=tx1.ceil() as i64 - 1 {
                // Wrap across the antimeridian; rows don't wrap.
                let wx = tx.rem_euclid(n) as u32;
                let Some(bytes) = self.tile(z, wx, ty as u32) else {
                    continue;
                };
                let (bx0, by0, bx1, by1) = tile_bounds(z, tx, ty);
                let (a, b) = (px(bx0, by0), px(bx1, by1));
                let rect = Rect {
                    x: a.0.min(b.0),
                    y: a.1.min(b.1),
                    width: (b.0 - a.0).abs(),
                    height: (b.1 - a.1).abs(),
                };
                backend.set_mark_attrs(vec![("data-tile".to_string(), format!("{z}/{wx}/{ty}"))]);
                backend.draw_image(&bytes, &rect, &view, self.alpha)?;
            }
        }
        backend.set_mark_attrs(Vec::new());
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
    }
    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "map_tile"
    }
    fn inherit_aes(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::SfProjection;

    #[test]
    fn tile_grid_matches_the_slippy_map_convention() {
        // Berlin (13.4°E, 52.52°N) is tile 8/137/83 on OSM.
        let [mx, my] = SfProjection::Mercator.project([13.4, 52.52]);
        let (tx, ty) = tile_xy(8, mx, my);
        assert_eq!((tx.floor(), ty.floor()), (137.0, 83.0));
        // Bounds round-trip: the tile's corners land on integer tile coords.
        let (x0, yt, x1, yb) = tile_bounds(8, 137, 83);
        let (a, b) = (tile_xy(8, x0, yt), tile_xy(8, x1, yb));
        assert!((a.0 - 137.0).abs() < 1e-9 && (a.1 - 83.0).abs() < 1e-9);
        assert!((b.0 - 138.0).abs() < 1e-9 && (b.1 - 84.0).abs() < 1e-9);
        assert_eq!(
            TileSource::xyz("https://t/{z}/{x}/{y}.png").url(8, 137, 83),
            "https://t/8/137/83.png"
        );
    }

    #[test]
    fn zoom_follows_the_view_and_is_capped() {
        let geom = GeomMapTile::new(TileSource::xyz(""), |_, _, _| None);
        // The whole world over 256 px → zoom 0; over 1024 px → zoom 2.
        assert_eq!(geom.pick_zoom((2.0 * PI, 2.0 * PI), (256.0, 256.0)), 0);
        assert_eq!(geom.pick_zoom((2.0 * PI, 2.0 * PI), (1024.0, 1024.0)), 2);
        // A tiny view at huge size would need thousands of tiles: capped.
        let z = geom.pick_zoom((0.5, 0.5), (20_000.0, 20_000.0));
        let n = (1u64 << z) as f64;
        assert!((0.5 / (2.0 * PI) * n + 1.0).powi(2) <= MAX_TILES as f64);
        assert_eq!(geom.clone().zoom(5).pick_zoom((1.0, 1.0), (10.0, 10.0)), 5);
    }

    #[test]
    fn fetched_tiles_are_cached() {
        let calls = Arc::new(Mutex::new(0));
        let c = calls.clone();
        let geom = GeomMapTile::new(TileSource::xyz(""), move |_, _, _| {
            *c.lock().unwrap() += 1;
            Some(vec![1, 2, 3])
        });
        assert!(geom.tile(1, 0, 0).is_some());
        assert!(geom.tile(1, 0, 0).is_some());
        assert_eq!(*calls.lock().unwrap(), 1);
    }
}
