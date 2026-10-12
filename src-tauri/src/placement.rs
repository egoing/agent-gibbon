// 신규 창을 오른쪽부터 배치하며 겹치지 않는 세로 좌표를 무작위 선택한다.
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
    random: u64,
    occupied: &[Rect],
    gap: i64,
) -> Option<Rect> {
    if size.0 <= 0 || size.1 <= 0 || size.0 > area.width || size.1 > area.height {
        return None;
    }
    // Prefer the rightmost available column; choose among every legal y pixel.
    let mut xs = vec![area.x, area.right() - size.0];
    for other in occupied {
        xs.push(other.x - size.0 - gap);
    }
    xs.sort_unstable();
    xs.dedup();
    for x in xs.into_iter().rev() {
        let candidates: Vec<_> = (area.y..=area.bottom() - size.1)
            .map(|y| Rect {
                x,
                y,
                width: size.0,
                height: size.1,
            })
            .filter(|candidate| {
                candidate.fits(area)
                    && occupied
                        .iter()
                        .all(|&other| !candidate.conflicts(other, gap))
            })
            .collect();
        if !candidates.is_empty() {
            return Some(candidates[(random % candidates.len() as u64) as usize]);
        }
    }
    None
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
    fn right_edge_and_random_vertical_position() {
        assert_eq!(
            find_space(rect(0, 30, 1000, 700), (300, 240), 70, &[], 8),
            Some(rect(700, 100, 300, 240))
        );
    }
    #[test]
    fn samples_distinct_legal_heights_and_skips_blocked_ranges() {
        let area = rect(0, 0, 1000, 1000);
        let occupied = [rect(700, 300, 300, 240)];
        let mut heights = std::collections::BTreeSet::new();
        for random in 0..=1000 {
            let placed = find_space(area, (300, 240), random, &occupied, 8).unwrap();
            assert_eq!(placed.x, 700);
            assert!(!placed.conflicts(occupied[0], 8));
            assert!(placed.fits(area));
            heights.insert(placed.y);
        }
        assert!(heights.len() > 200);
    }
    #[test]
    fn avoids_existing_window_with_gap() {
        let occupied = [rect(200, 100, 300, 240)];
        let placed = find_space(rect(0, 30, 1000, 700), (300, 240), 70, &occupied, 8).unwrap();
        assert!(!placed.conflicts(occupied[0], 8));
        assert_eq!(occupied[0], rect(200, 100, 300, 240));
    }
    #[test]
    fn fills_remaining_hole_between_windows() {
        let occupied = [rect(0, 0, 308, 480), rect(616, 0, 308, 480)];
        assert_eq!(
            find_space(rect(0, 0, 924, 480), (292, 240), 0, &occupied, 8),
            Some(rect(316, 0, 292, 240))
        );
    }
    #[test]
    fn supports_negative_monitor_origins_and_large_scale() {
        let area = rect(-2560, -300, 2560, 1440);
        let occupied = [rect(-1800, 0, 600, 480)];
        let placed = find_space(area, (600, 480), 42, &occupied, 16).unwrap();
        assert!(placed.fits(area));
        assert!(!placed.conflicts(occupied[0], 16));
    }
    #[test]
    fn full_or_undersized_screen_has_no_position() {
        let area = rect(0, 0, 300, 240);
        assert_eq!(find_space(area, (300, 240), 0, &[area], 8), None);
        assert_eq!(find_space(area, (301, 240), 0, &[], 8), None);
    }
    #[test]
    fn repeated_creation_never_overlaps_until_full() {
        let area = rect(0, 0, 1000, 750);
        let mut occupied = Vec::new();
        while let Some(next) = find_space(area, (300, 240), 0, &occupied, 8) {
            assert!(occupied.iter().all(|&other| !next.conflicts(other, 8)));
            occupied.push(next);
            assert!(occupied.len() <= 9);
        }
        assert!(occupied.len() >= 4);
    }
}
