// 신규 창만 배치한다. 작업 영역 안의 창 모서리 후보를 검사해 기존 창을 피한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}
impl Rect {
    fn right(self) -> i64 {
        self.x + self.width
    }
    fn bottom(self) -> i64 {
        self.y + self.height
    }
    fn fits(self, area: Self) -> bool {
        self.x >= area.x
            && self.y >= area.y
            && self.right() <= area.right()
            && self.bottom() <= area.bottom()
    }
    fn conflicts(self, other: Self, gap: i64) -> bool {
        self.x < other.right() + gap
            && self.right() + gap > other.x
            && self.y < other.bottom() + gap
            && self.bottom() + gap > other.y
    }
}

pub fn find_space(
    area: Rect,
    size: (i64, i64),
    preferred: (i64, i64),
    occupied: &[Rect],
    gap: i64,
) -> Option<Rect> {
    if size.0 <= 0 || size.1 <= 0 || size.0 > area.width || size.1 > area.height {
        return None;
    }
    let mut xs = vec![
        area.x,
        area.right() - size.0,
        preferred.0.clamp(area.x, area.right() - size.0),
    ];
    let mut ys = vec![
        area.y,
        area.bottom() - size.1,
        preferred.1.clamp(area.y, area.bottom() - size.1),
    ];
    for other in occupied {
        xs.extend([other.right() + gap, other.x - size.0 - gap]);
        ys.extend([other.bottom() + gap, other.y - size.1 - gap]);
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    xs.into_iter()
        .flat_map(|x| {
            ys.iter().map(move |&y| Rect {
                x,
                y,
                width: size.0,
                height: size.1,
            })
        })
        .filter(|candidate| {
            candidate.fits(area)
                && occupied
                    .iter()
                    .all(|&other| !candidate.conflicts(other, gap))
        })
        .min_by_key(|candidate| {
            let dx = i128::from(candidate.x - preferred.0);
            let dy = i128::from(candidate.y - preferred.1);
            (dx * dx + dy * dy, candidate.y, candidate.x)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i64, y: i64, width: i64, height: i64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    #[test]
    fn keeps_free_preferred_position() {
        assert_eq!(
            find_space(rect(0, 30, 1000, 700), (300, 240), (200, 100), &[], 8),
            Some(rect(200, 100, 300, 240))
        );
    }
    #[test]
    fn avoids_existing_window_with_gap() {
        let occupied = [rect(200, 100, 300, 240)];
        let placed =
            find_space(rect(0, 30, 1000, 700), (300, 240), (200, 100), &occupied, 8).unwrap();
        assert!(!placed.conflicts(occupied[0], 8));
        assert_eq!(occupied[0], rect(200, 100, 300, 240));
    }
    #[test]
    fn fills_remaining_hole_between_windows() {
        let occupied = [rect(0, 0, 308, 480), rect(616, 0, 308, 480)];
        assert_eq!(
            find_space(rect(0, 0, 924, 480), (292, 240), (0, 0), &occupied, 8),
            Some(rect(316, 0, 292, 240))
        );
    }
    #[test]
    fn supports_negative_monitor_origins_and_large_scale() {
        let area = rect(-2560, -300, 2560, 1440);
        let occupied = [rect(-1800, 0, 600, 480)];
        let placed = find_space(area, (600, 480), (-1800, 0), &occupied, 16).unwrap();
        assert!(placed.fits(area));
        assert!(!placed.conflicts(occupied[0], 16));
    }
    #[test]
    fn full_or_undersized_screen_has_no_position() {
        let area = rect(0, 0, 300, 240);
        assert_eq!(find_space(area, (300, 240), (0, 0), &[area], 8), None);
        assert_eq!(find_space(area, (301, 240), (0, 0), &[], 8), None);
    }
    #[test]
    fn repeated_creation_never_overlaps_until_full() {
        let area = rect(0, 0, 1000, 750);
        let mut occupied = Vec::new();
        while let Some(next) = find_space(area, (300, 240), (350, 255), &occupied, 8) {
            assert!(occupied.iter().all(|&other| !next.conflicts(other, 8)));
            occupied.push(next);
            assert!(occupied.len() <= 9);
        }
        assert!(occupied.len() >= 4);
    }
}
