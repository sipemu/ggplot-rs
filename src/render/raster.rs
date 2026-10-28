//! Decoding an encoded raster image (PNG/JPEG) into the RGBA pixels it
//! covers on a canvas — shared by the raster backends' `draw_image` (#325).

use crate::render::Rect;

/// The visible part of `bytes` stretched over `rect` and clipped to `clip`
/// and to a `canvas_w`×`canvas_h` canvas: its top-left canvas pixel and the
/// RGBA pixels (nearest-neighbour sampling). `None` when the image doesn't
/// decode or nothing is visible.
pub(crate) fn decode_visible(
    bytes: &[u8],
    rect: &Rect,
    clip: &Rect,
    canvas_w: u32,
    canvas_h: u32,
) -> Option<((i32, i32), image::RgbaImage)> {
    if !(rect.width > 0.0 && rect.height > 0.0) {
        return None;
    }
    let src = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (sw, sh) = src.dimensions();
    let x0 = rect.x.max(clip.x).max(0.0).round() as i64;
    let y0 = rect.y.max(clip.y).max(0.0).round() as i64;
    let x1 = (rect.x + rect.width)
        .min(clip.x + clip.width)
        .min(canvas_w as f64)
        .round() as i64;
    let y1 = (rect.y + rect.height)
        .min(clip.y + clip.height)
        .min(canvas_h as f64)
        .round() as i64;
    if x1 <= x0 || y1 <= y0 || sw == 0 || sh == 0 {
        return None;
    }
    let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let out = image::RgbaImage::from_fn(w, h, |dx, dy| {
        // Sample at the destination pixel's centre.
        let fx = ((x0 + dx as i64) as f64 + 0.5 - rect.x) / rect.width;
        let fy = ((y0 + dy as i64) as f64 + 0.5 - rect.y) / rect.height;
        let sx = ((fx * sw as f64) as u32).min(sw - 1);
        let sy = ((fy * sh as f64) as u32).min(sh - 1);
        *src.get_pixel(sx, sy)
    });
    Some(((x0 as i32, y0 as i32), out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_fn(w, h, |x, y| image::Rgba(f(x, y)));
        let mut out = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut out),
            image::ImageOutputFormat::Png,
        )
        .unwrap();
        out
    }

    #[test]
    fn scales_and_clips_to_the_visible_part() {
        // 2×2 quadrants: red, green / blue, white.
        let bytes = png(2, 2, |x, y| match (x, y) {
            (0, 0) => [255, 0, 0, 255],
            (1, 0) => [0, 255, 0, 255],
            (0, 1) => [0, 0, 255, 255],
            _ => [255, 255, 255, 255],
        });
        let rect = Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        };
        let clip = Rect {
            x: 20.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let ((x, y), img) = decode_visible(&bytes, &rect, &clip, 100, 100).unwrap();
        // Only the right half is visible: green over white, 10×20 px.
        assert_eq!((x, y), (20, 10));
        assert_eq!(img.dimensions(), (10, 20));
        assert_eq!(img.get_pixel(0, 0).0, [0, 255, 0, 255]);
        assert_eq!(img.get_pixel(9, 19).0, [255, 255, 255, 255]);
        assert!(decode_visible(b"not an image", &rect, &clip, 100, 100).is_none());
        let away = Rect { x: 500.0, ..clip };
        assert!(decode_visible(&bytes, &rect, &away, 100, 100).is_none());
    }
}
