use crate::placement::{self, Item, Rect};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arrangement {
    FreeSpace,
    Left,
    Right,
    Grid,
}

impl Arrangement {
    pub(super) fn columns(self, area: Rect, width: i64, items: &[Item]) -> Vec<Rect> {
        let mut frames = placement::stack(area, width, items, &[]);
        if self == Self::Left {
            for frame in &mut frames {
                let inset = area.x + area.width - (frame.x + frame.width);
                frame.x = area.x + inset;
            }
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_columns_preserve_order_and_align_different_widths_on_a_negative_screen() {
        let area = Rect {
            x: -720,
            y: -160,
            width: 640,
            height: 200,
        };
        let note = Item {
            width: 300,
            min: 100,
            max: 100,
        };
        let narrow = Item { width: 140, ..note };
        let items = [note, narrow, narrow, note];
        let left = Arrangement::Left.columns(area, 300, &items);
        assert_eq!(
            left.iter().map(|r| (r.x, r.y)).collect::<Vec<_>>(),
            [(-720, -60), (-720, -160), (-420, -60), (-420, -160)]
        );
        for (l, r) in left
            .iter()
            .zip(Arrangement::Right.columns(area, 300, &items))
        {
            assert_eq!((l.width, l.height, l.y), (r.width, r.height, r.y));
        }
    }
}
