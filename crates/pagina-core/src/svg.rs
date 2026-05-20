/// SVG rendering via resvg (rasterize to pixels, then embed as image).

use crate::layout::LoadedImage;

/// Render SVG source to a rasterized image at the given DPI.
pub fn render_svg(svg_data: &str, dpi: f32) -> Option<LoadedImage> {
    let opts = resvg::usvg::Options {
        dpi,
        ..Default::default()
    };

    let tree = resvg::usvg::Tree::from_str(svg_data, &opts).ok()?;
    let size = tree.size();

    let scale = dpi / 96.0;
    let pixel_w = (size.width() * scale).ceil() as u32;
    let pixel_h = (size.height() * scale).ceil() as u32;

    if pixel_w == 0 || pixel_h == 0 {
        return None;
    }

    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixel_w, pixel_h)?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert RGBA to RGB (PDF doesn't need alpha for most cases)
    let rgba = pixmap.data();
    let mut rgb = Vec::with_capacity((pixel_w * pixel_h * 3) as usize);
    for chunk in rgba.chunks(4) {
        rgb.push(chunk[0]); // R
        rgb.push(chunk[1]); // G
        rgb.push(chunk[2]); // B
    }

    Some(LoadedImage {
        pixels: rgb,
        width: pixel_w,
        height: pixel_h,
    })
}

/// Render SVG from a file path.
pub fn render_svg_file(path: &str, dpi: f32) -> Option<LoadedImage> {
    let data = std::fs::read_to_string(path).ok()?;
    render_svg(&data, dpi)
}

/// Get the natural size of an SVG in mm (at 96 DPI).
pub fn svg_natural_size_mm(svg_data: &str) -> Option<(f64, f64)> {
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(svg_data, &opts).ok()?;
    let size = tree.size();
    // SVG size is in px (at 96 DPI), convert to mm
    let w_mm = size.width() as f64 * 25.4 / 96.0;
    let h_mm = size.height() as f64 * 25.4 / 96.0;
    Some((w_mm, h_mm))
}
