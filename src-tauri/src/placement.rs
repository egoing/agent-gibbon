// Deterministic bounded column layout in physical pixels.
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
    #[cfg(test)]
    fn fits(self, area: Self) -> bool {
        self.x >= area.x
            && self.y >= area.y
            && self.right() <= area.right()
            && self.bottom() <= area.bottom()
    }
    #[cfg(test)]
    fn conflicts(self, other: Self, gap: i64) -> bool {
        self.x < other.right() + gap
            && self.right() + gap > other.x
            && self.y < other.bottom() + gap
            && self.bottom() + gap > other.y
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub width: i64,
    pub min: i64,
    pub max: i64,
}

/// Right-to-left columns, top-to-bottom free spans. Allocate bounded heights
/// across each span; integer remainders belong to the first items (no gaps).
pub fn stack(area: Rect, width: i64, items: &[Item], obstacles: &[Rect]) -> Vec<Rect> {
    if width <= 0 || area.height <= 0 {
        return Vec::new();
    }
    let mut slots = Vec::new();
    let mut right = area.right();
    while right - width >= area.x {
        let x = right - width;
        let mut spans = vec![(area.y, area.bottom())];
        for o in obstacles.iter().filter(|o| o.x < right && o.right() > x) {
            spans = spans
                .into_iter()
                .flat_map(|(a, b)| {
                    if o.y >= b || o.bottom() <= a {
                        vec![(a, b)]
                    } else {
                        [(a, o.y.min(b)), (o.bottom().max(a), b)]
                            .into_iter()
                            .filter(|(a, b)| a < b)
                            .collect()
                    }
                })
                .collect();
        }
        spans.sort_unstable();
        slots.extend(spans.into_iter().map(|(top, bottom)| (right, top, bottom)));
        right -= width;
    }
    let mut result = Vec::new();
    for (index, &(right, top, bottom)) in slots.iter().enumerate() {
        let later_capacity: i64 = slots[index + 1..].iter().map(|(_, a, b)| b - a).sum();
        let remaining = &items[result.len()..];
        let capacity = bottom - top;
        let (mut count, mut minimum, mut maximum) = (0, 0, 0);
        for (i, item) in remaining.iter().enumerate() {
            let suffix_min: i64 = remaining[i..].iter().map(|item| item.min).sum();
            if minimum + item.min > capacity
                || (maximum >= capacity && suffix_min <= later_capacity)
            {
                break;
            }
            minimum += item.min;
            maximum += item.max;
            count += 1;
        }
        if count == 0 {
            continue;
        }
        let chosen = &remaining[..count];
        let mut heights: Vec<_> = chosen.iter().map(|i| i.min).collect();
        let mut extra = capacity.min(maximum) - minimum;
        while extra > 0 {
            let active = heights
                .iter()
                .zip(chosen)
                .filter(|(h, i)| **h < i.max)
                .count() as i64;
            if active == 0 {
                break;
            }
            let share = (extra / active).max(1);
            for (h, item) in heights.iter_mut().zip(chosen) {
                let add = share.min(item.max - *h).min(extra);
                *h += add;
                extra -= add;
            }
        }
        let mut y = top;
        for (h, item) in heights.into_iter().zip(chosen) {
            result.push(Rect {
                x: right - item.width,
                y,
                width: item.width,
                height: h,
            });
            y += h;
        }
    }
    result
}

// When every monitor is occupied, keep the panel on screen with minimal overlap.
pub fn least_overlap(area: Rect, size: (i64, i64), occupied: &[Rect]) -> Option<(i64, Rect)> {
    if size.0 <= 0 || size.1 <= 0 || size.0 > area.width || size.1 > area.height {
        return None;
    }
    let mut xs = vec![area.x, area.right() - size.0];
    let mut ys = vec![area.y, area.bottom() - size.1];
    for r in occupied {
        xs.extend([r.x - size.0, r.right()]);
        ys.extend([r.y - size.1, r.bottom()]);
    }
    let mut best = None;
    for x in xs {
        for &y in &ys {
            let candidate = Rect {
                x: x.clamp(area.x, area.right() - size.0),
                y: y.clamp(area.y, area.bottom() - size.1),
                width: size.0,
                height: size.1,
            };
            let score = occupied
                .iter()
                .map(|r| {
                    (candidate.right().min(r.right()) - candidate.x.max(r.x)).max(0)
                        * (candidate.bottom().min(r.bottom()) - candidate.y.max(r.y)).max(0)
                })
                .sum();
            if best.as_ref().is_none_or(|(old, _)| score < *old) {
                best = Some((score, candidate));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    fn area() -> Rect {
        Rect {
            x: 0,
            y: 30,
            width: 900,
            height: 1000,
        }
    }
    fn text() -> Item {
        Item {
            width: 300,
            min: 48,
            max: 420,
        }
    }
    #[test]
    fn short_content_is_never_stretched_to_fill_the_screen() {
        let short = Item {
            width: 300,
            min: 48,
            max: 48,
        };
        let medium = Item {
            width: 300,
            min: 48,
            max: 123,
        };
        let r = stack(area(), 300, &[short, medium, short], &[]);
        assert_eq!(
            r.iter().map(|r| r.height).collect::<Vec<_>>(),
            vec![48, 123, 48]
        );
        assert_eq!(r[1].y, r[0].bottom());
        assert_eq!(r[2].y, r[1].bottom());
    }
    #[test]
    fn zero_gap_fills_exact_horizontal_and_vertical_slots() {
        let rect = |x, y, width, height| Rect {
            x,
            y,
            width,
            height,
        };
        let area = rect(-600, 30, 600, 480);
        let item = Item {
            width: 300,
            min: 240,
            max: 240,
        };
        assert_eq!(
            stack(area, 300, &[item; 4], &[]),
            vec![
                rect(-300, 30, 300, 240),
                rect(-300, 270, 300, 240),
                rect(-600, 30, 300, 240),
                rect(-600, 270, 300, 240),
            ]
        );
        assert_eq!(stack(area, 300, &[item; 5], &[]).len(), 4);
    }
    #[test]
    fn ordinary_notes_fill_top_down_then_left() {
        let r = stack(area(), 300, &[text(); 5], &[]);
        assert_eq!(r.len(), 5);
        assert_eq!((r[0].x, r[0].y), (600, 30));
        assert_eq!(r[0].height + r[1].height + r[2].height, 1000);
        assert_eq!(r[1].y, r[0].bottom());
        assert_eq!(r[2].y, r[1].bottom());
        assert_eq!((r[3].x, r[3].y), (300, 30));
        assert_eq!(r[4].y, r[3].bottom());
        assert!(r
            .iter()
            .all(|r| r.fits(area()) && r.height >= 48 && r.height <= 420));
    }
    #[test]
    fn obstacles_and_negative_origins() {
        let a = Rect {
            x: -900,
            y: -30,
            ..area()
        };
        let obstacle = Rect {
            x: -300,
            y: 200,
            width: 300,
            height: 200,
        };
        let r = stack(a, 300, &[text(); 7], &[obstacle]);
        assert_eq!(r[0].y, -30);
        assert_eq!(r[0].bottom(), 200);
        assert_eq!(r[1].y, 400);
        for (i, rect) in r.iter().enumerate() {
            assert!(rect.fits(a));
            assert!(!rect.conflicts(obstacle, 0));
            assert!(rect.height <= 420);
            for other in &r[..i] {
                assert!(!rect.conflicts(*other, 0));
            }
        }
    }
    #[test]
    fn bounded_single_and_exact_remainders() {
        assert_eq!(stack(area(), 300, &[text()], &[])[0].height, 420);
        let a = Rect {
            height: 1001,
            ..area()
        };
        let r = stack(a, 300, &[text(); 3], &[]);
        assert_eq!(r.iter().map(|r| r.height).sum::<i64>(), 1001);
        assert_eq!(r.last().unwrap().bottom(), a.bottom());
    }
    #[test]
    fn full_screen_and_short_slots() {
        assert!(stack(area(), 300, &[text()], &[area()]).is_empty());
        let a = Rect {
            height: 47,
            ..area()
        };
        assert!(stack(a, 300, &[text()], &[]).is_empty());
        let r = stack(area(), 300, &[text(); 100], &[]);
        assert_eq!(r.len(), 60);
        assert_eq!(stack(area(), 300, &[text(); 10], &[]).len(), 10);
    }
    #[test]
    fn fixed_ratio_images_and_folded_heights() {
        let items = [
            Item {
                width: 210,
                min: 420,
                max: 420,
            },
            Item {
                width: 300,
                min: 12,
                max: 12,
            },
            text(),
            text(),
        ];
        let r = stack(area(), 300, &items, &[]);
        assert_eq!((r[0].width, r[0].height), (210, 420));
        assert_eq!(r[1].height, 12);
        assert_eq!(r.last().unwrap().bottom(), area().bottom());
    }
}
