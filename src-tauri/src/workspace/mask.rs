use std::collections::VecDeque;
use xcap::image::{Rgba, RgbaImage};
#[derive(Clone, Copy, Debug)]
struct BoxArea {
    x: u32,
    y: u32,
    right: u32,
    bottom: u32,
    pixels: u32,
}
impl BoxArea {
    fn width(self) -> u32 {
        self.right - self.x + 1
    }
    fn height(self) -> u32 {
        self.bottom - self.y + 1
    }
    fn merge(&mut self, other: Self) {
        self.x = self.x.min(other.x);
        self.y = self.y.min(other.y);
        self.right = self.right.max(other.right);
        self.bottom = self.bottom.max(other.bottom);
        self.pixels += other.pixels;
    }
}
fn brightness(p: &Rgba<u8>) -> i32 {
    (i32::from(p[0]) * 77 + i32::from(p[1]) * 150 + i32::from(p[2]) * 29) / 256
}
pub(super) fn redact(mut source: RgbaImage, scale: f64) -> RgbaImage {
    let (width, height) = source.dimensions();
    let mut mask = vec![false; (width * height) as usize];
    // Local contrast against the median of a surrounding ring. The ring avoids
    // assuming dark mode or any particular text colour. Thresholds are fixed.
    let radius = (4.0 * scale).round().clamp(3.0, 16.0) as i32;
    let offsets = [
        (-radius, -radius),
        (0, -radius),
        (radius, -radius),
        (-radius, 0),
        (radius, 0),
        (-radius, radius),
        (0, radius),
        (radius, radius),
    ];
    let edge = radius as u32;
    for y in edge..height.saturating_sub(edge) {
        for x in edge..width.saturating_sub(edge) {
            let p = source.get_pixel(x, y);
            let mut neighbors = [0; 8];
            for (i, (dx, dy)) in offsets.iter().enumerate() {
                neighbors[i] =
                    brightness(source.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32));
            }
            neighbors.sort_unstable();
            mask[(y * width + x) as usize] = (brightness(p) - neighbors[4]).abs() >= 35;
        }
    }
    let mut components = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            if !mask[index] {
                continue;
            }
            mask[index] = false;
            let mut queue = VecDeque::from([(x, y)]);
            let mut area = BoxArea {
                x,
                y,
                right: x,
                bottom: y,
                pixels: 0,
            };
            while let Some((cx, cy)) = queue.pop_front() {
                area.merge(BoxArea {
                    x: cx,
                    y: cy,
                    right: cx,
                    bottom: cy,
                    pixels: 1,
                });
                for ny in cy.saturating_sub(1)..=(cy + 1).min(height - 1) {
                    for nx in cx.saturating_sub(1)..=(cx + 1).min(width - 1) {
                        let i = (ny * width + nx) as usize;
                        if mask[i] {
                            mask[i] = false;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
            if area.pixels >= 4
                && area.width() <= 10 * edge
                && area.height() >= 3
                && area.height() <= 8 * edge
            {
                components.push(area);
            }
        }
    }
    // Group glyph-like components by vertical overlap and a limited horizontal gap.
    components.sort_by_key(|a| (a.x, a.y));
    let mut lines: Vec<(BoxArea, u32)> = Vec::new();
    for component in &components {
        let candidate = lines
            .iter_mut()
            .filter(|(line, _)| {
                let overlap =
                    line.bottom.min(component.bottom) as i64 - line.y.max(component.y) as i64 + 1;
                component.x <= line.right + 5 * edge
                    && component.right >= line.x
                    && overlap * 2 >= i64::from(line.height().min(component.height()))
                    && line.bottom.max(component.bottom) - line.y.min(component.y) < 9 * edge
            })
            .min_by_key(|(line, _)| line.y.abs_diff(component.y));
        if let Some((line, count)) = candidate {
            line.merge(*component);
            *count += 1;
        } else {
            lines.push((*component, 1));
        }
    }
    lines.retain(|(line, count)| *count >= 2 || line.width() >= 2 * edge);
    lines.sort_by_key(|(line, _)| (line.y, line.x));

    for (line, _) in lines {
        for y in line.y.saturating_sub(edge)..=(line.bottom + edge).min(height - 1) {
            for x in line.x.saturating_sub(edge)..=(line.right + edge).min(width - 1) {
                source.put_pixel(x, y, Rgba([75, 75, 75, 255]));
            }
        }
    }
    source
}
