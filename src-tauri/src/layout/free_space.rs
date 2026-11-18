//! Local, single-frame placement for the left gesture. Pixels never leave this call.
use super::*;
use xcap::image::RgbaImage;

fn intersects(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

fn capture(monitors: &[xcap::Monitor], screen: Rect) -> Option<RgbaImage> {
    #[cfg(target_os = "macos")]
    if !crate::capture::permission_status().0 {
        return None;
    }
    let monitor = monitors.iter().find(|m| {
        m.x().ok().map(i64::from) == Some(screen.x)
            && m.y().ok().map(i64::from) == Some(screen.y)
            && m.width().ok().map(i64::from) == Some(screen.width)
            && m.height().ok().map(i64::from) == Some(screen.height)
    })?;
    monitor.capture_image().ok()
}

// Mark tiles with contrasting pixels, including thin strokes on flat backgrounds.
fn busy_tiles(image: &RgbaImage, screen: Rect, step: i64) -> Vec<Rect> {
    let mut busy = Vec::new();
    let px = |x| (x * i64::from(image.width()) / screen.width) as u32;
    let py = |y| (y * i64::from(image.height()) / screen.height) as u32;
    for y in (0..screen.height).step_by(step as usize) {
        for x in (0..screen.width).step_by(step as usize) {
            let right = (x + step).min(screen.width);
            let bottom = (y + step).min(screen.height);
            let mut low = [255; 3];
            let mut high = [0; 3];
            let mut contrast = false;
            'scan: for row in (py(y)..py(bottom)).step_by(2) {
                for col in (px(x)..px(right)).step_by(2) {
                    let pixel = image.get_pixel(col, row);
                    for c in 0..3 {
                        low[c] = low[c].min(pixel[c]);
                        high[c] = high[c].max(pixel[c]);
                        if high[c] - low[c] > 28 {
                            contrast = true;
                            break 'scan;
                        }
                    }
                }
            }
            if contrast {
                busy.push(Rect {
                    x: screen.x + x,
                    y: screen.y + y,
                    width: right - x,
                    height: bottom - y,
                });
            }
        }
    }
    busy
}

#[cfg(target_os = "macos")]
fn app_windows() -> Result<(Vec<Rect>, Vec<Rect>), String> {
    use std::ffi::c_void;
    extern "C" {
        fn gibbon_visit_app_windows(
            context: *mut c_void,
            visit: extern "C" fn(*mut c_void, f64, f64, f64, f64, bool),
        ) -> bool;
    }
    extern "C" fn collect(
        context: *mut c_void,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        gutter: bool,
    ) {
        let (left, top, right, bottom) = if gutter {
            (left.ceil(), top.ceil(), right.floor(), bottom.floor())
        } else {
            (left.floor(), top.floor(), right.ceil(), bottom.ceil())
        };
        let (x, y) = (left as i64, top as i64);
        // The visitor runs synchronously, with an exclusive borrow of the vector.
        let lists = unsafe { &mut *context.cast::<(Vec<Rect>, Vec<Rect>)>() };
        let target = if gutter { &mut lists.1 } else { &mut lists.0 };
        target.push(Rect {
            x,
            y,
            width: right.ceil() as i64 - x,
            height: bottom.ceil() as i64 - y,
        });
    }
    let mut windows = (Vec::new(), Vec::new());
    if unsafe {
        gibbon_visit_app_windows(
            (&mut windows as *mut (Vec<Rect>, Vec<Rect>)).cast(),
            collect,
        )
    } {
        Ok(windows)
    } else {
        Err("표시 중인 앱 창 경계를 조회하지 못했습니다".into())
    }
}

#[cfg(not(target_os = "macos"))]
fn app_windows() -> Result<(Vec<Rect>, Vec<Rect>), String> {
    let mut result = Vec::new();
    for w in xcap::Window::all().map_err(|e| e.to_string())? {
        if w.pid().ok() == Some(std::process::id()) || w.is_minimized().unwrap_or(false) {
            continue;
        }
        if let (Ok(x), Ok(y), Ok(width), Ok(height)) = (w.x(), w.y(), w.width(), w.height()) {
            if width > 0 && height > 0 {
                result.push(Rect {
                    x: x.into(),
                    y: y.into(),
                    width: width.into(),
                    height: height.into(),
                });
            }
        }
    }
    Ok((result, Vec::new()))
}

struct Screen {
    bounds: Rect,
    area: Rect,
    scale: f64,
    busy: Option<Vec<Rect>>,
    gutters: Vec<Rect>,
    apps: Vec<Rect>,
}

impl Screen {
    fn density(&self) -> f64 {
        self.scale / unit(self.scale)
    }
    fn step(&self) -> i64 {
        (24.0 * self.density()).round().max(1.0) as i64
    }
}

struct Memo {
    rect: Rect,
    home: usize,
    fixed: bool,
    stack_size: placement::Item,
}

fn pending_moves(screens: &[Screen], memos: &[Memo]) -> (Vec<(usize, bool)>, Vec<Rect>) {
    let mut occupied: Vec<_> = memos.iter().filter(|m| m.fixed).map(|m| m.rect).collect();
    let mut pending = Vec::new();
    for (i, m) in memos.iter().enumerate().filter(|(_, m)| !m.fixed) {
        if m.rect.fits(screens[m.home].area)
            && (!screens[m.home].apps.iter().any(|a| intersects(*a, m.rect))
                || screens[m.home].gutters.iter().any(|g| m.rect.fits(*g)))
            && !occupied.iter().any(|r| intersects(*r, m.rect))
        {
            occupied.push(m.rect);
        } else {
            pending.push((i, true));
        }
    }
    // A right-aligned fallback column can fill an entire screen without any
    // rectangle technically overlapping. Keep its first window as an anchor.
    // These optional moves keep their original slot if no clear space exists.
    for (i, m) in memos.iter().enumerate() {
        if m.fixed || pending.contains(&(i, true)) {
            continue;
        }
        let a = m.rect;
        let stacked = memos.iter().any(|other| {
            let b = other.rect;
            other.home == m.home
                && (a.x + a.width - b.x - b.width).abs() <= 1
                && (b.y - a.y - a.height).abs() <= 1
        });
        if stacked {
            pending.push((i, false));
        }
    }
    (pending, occupied)
}

fn clear_spot(
    screen: &Screen,
    old: Rect,
    ratio: f64,
    occupied: &[Rect],
    memos: &[Memo],
    gutter: Option<Rect>,
) -> Option<Rect> {
    let busy = screen.busy.as_ref()?;
    let area = gutter.unwrap_or(screen.area);
    let width = (old.width as f64 * ratio).round().max(1.) as i64;
    let height = (old.height as f64 * ratio).round().max(1.) as i64;
    let step = screen.step() as usize;
    let mut best = None;
    let last_x = area.x + area.width - width;
    let last_y = area.y + area.height - height;
    if last_x < area.x || last_y < area.y {
        return None;
    }
    // Include native window edges: a memo-width strip can fall between grid points.
    let axis = |start, end, length, horizontal| {
        let mut points: Vec<i64> = (start..end).step_by(step).chain([end]).collect();
        for app in &screen.apps {
            let (origin, extent) = if horizontal {
                (app.x, app.width)
            } else {
                (app.y, app.height)
            };
            points.extend(
                [origin - length, origin + extent]
                    .into_iter()
                    .filter(|v| (start..=end).contains(v)),
            );
        }
        points.sort_unstable();
        points.dedup();
        points
    };
    let xs = axis(area.x, last_x, width, true);
    for y in axis(area.y, last_y, height, false) {
        for &x in &xs {
            let distance = x.abs_diff(old.x) + y.abs_diff(old.y);
            if best.is_some_and(|(cost, _)| cost <= distance) {
                continue;
            }
            let r = Rect {
                x,
                y,
                width,
                height,
            };
            let gap = (6.0 * screen.density()).ceil() as i64;
            let padded = Rect {
                x: x - gap,
                y: y - gap,
                width: width + 2 * gap,
                height: height + 2 * gap,
            };
            if !r.fits(screen.area)
                || (gutter.is_none() && screen.apps.iter().any(|a| intersects(*a, r)))
                || occupied
                    .iter()
                    .any(|b| intersects(*b, screen.bounds) && intersects(*b, padded))
                || memos
                    .iter()
                    .any(|m| intersects(m.rect, screen.bounds) && intersects(m.rect, padded))
                || busy.iter().any(|b| intersects(*b, r))
            {
                continue;
            }
            best = Some((distance, r));
        }
    }
    best.map(|(_, r)| r)
}

fn plan(
    screens: &[Screen],
    memos: &[Memo],
    pending: Vec<(usize, bool)>,
    mut occupied: Vec<Rect>,
) -> Vec<(usize, usize, Rect)> {
    let mut moves = Vec::new();
    let mut remaining = Vec::new();
    for (i, required) in pending {
        let m = &memos[i];
        let mut monitors: Vec<_> = (0..screens.len()).collect();
        monitors.sort_by_key(|&d| {
            (
                d != m.home,
                screens[d].area.x.abs_diff(m.rect.x) + screens[d].area.y.abs_diff(m.rect.y),
            )
        });
        let spot = monitors.iter().copied().find_map(|d| {
            let ratio = screens[d].density() / screens[m.home].density();
            clear_spot(&screens[d], m.rect, ratio, &occupied, memos, None).map(|r| (d, r))
        });
        let spot = spot.or_else(|| {
            monitors.iter().copied().find_map(|d| {
                let ratio = screens[d].density() / screens[m.home].density();
                screens[d].gutters.iter().find_map(|g| {
                    clear_spot(&screens[d], m.rect, ratio, &occupied, memos, Some(*g))
                        .map(|r| (d, r))
                })
            })
        });
        if let Some((d, r)) = spot {
            occupied.push(r);
            moves.push((i, d, r));
        } else if required {
            remaining.push(i);
        }
    }
    for (index, &i) in remaining.iter().enumerate() {
        let m = &memos[i];
        // Until a later memo has a destination, its current footprint must remain available.
        let mut obstacles = occupied.clone();
        obstacles.extend(remaining[index + 1..].iter().map(|&j| memos[j].rect));
        let slot = placement::stack(
            screens[m.home].area,
            m.stack_size.width,
            &[m.stack_size],
            &obstacles,
        )
        .into_iter()
        .next();
        let r = slot.unwrap_or(m.rect);
        occupied.push(r);
        if r != m.rect {
            moves.push((i, m.home, r));
        }
    }
    moves
}

#[derive(Default)]
pub(crate) struct LastLayout(Mutex<Option<LayoutSnapshot>>);
#[derive(PartialEq)]
struct LayoutSnapshot {
    fixed: Vec<String>,
    gutters: Vec<Rect>,
    screens: Vec<Rect>,
    apps: Vec<Rect>,
    windows: BTreeMap<String, Rect>,
}

pub(super) fn frames(
    app: &tauri::AppHandle,
    windows: Vec<WebviewWindow>,
    pointer: &tauri::Monitor,
) -> Result<Vec<HostMove>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    if monitors.is_empty() {
        return Ok(vec![]);
    }
    let (mut apps, gutters) = app_windows()?;
    apps.sort_unstable_by_key(|a| (a.x, a.y, a.width, a.height));
    let mut screens: Vec<_> = monitors
        .iter()
        .map(|m| {
            let scale = m.scale_factor();
            Screen {
                bounds: native_rect(*m.position(), *m.size(), scale),
                area: native_rect(m.work_area().position, m.work_area().size, scale),
                scale,
                apps: apps
                    .iter()
                    .copied()
                    .filter(|r| intersects(*r, native_rect(*m.position(), *m.size(), scale)))
                    .collect(),
                gutters: gutters
                    .iter()
                    .copied()
                    .filter(|g| intersects(*g, native_rect(*m.position(), *m.size(), scale)))
                    .collect(),
                busy: None,
            }
        })
        .collect();
    let mut memos = Vec::new();
    for w in &windows {
        let current = w.current_monitor().map_err(|e| e.to_string())?;
        let flight = entrance::destination(w);
        let rect = match flight {
            Some(d) => native_rect(d.position, d.size, d.scale),
            None => window_rect(w)?,
        };
        let home = current
            .as_ref()
            .and_then(|m| {
                monitors
                    .iter()
                    .position(|other| other.position() == m.position())
            })
            .or_else(|| {
                monitors
                    .iter()
                    .position(|m| m.position() == pointer.position())
            })
            .unwrap_or(0);
        memos.push(Memo {
            rect,
            home,
            fixed: flight.is_some()
                || w.is_focused().unwrap_or(true)
                || crate::workspace::fixed_for_gesture(w),
            stack_size: arrangement_item(w, screens[home].area, screens[home].scale),
        });
    }
    let mut layout = LayoutSnapshot {
        // Losing focus makes a previously protected memo eligible even if no frame changed.
        fixed: windows
            .iter()
            .zip(&memos)
            .filter(|(_, m)| m.fixed)
            .map(|(w, _)| w.label().to_owned())
            .collect(),
        gutters,
        screens: screens.iter().map(|s| s.area).collect(),
        apps,
        windows: windows
            .iter()
            .zip(&memos)
            .map(|(w, m)| (w.label().to_owned(), m.rect))
            .collect(),
    };
    let state = app.state::<LastLayout>();
    let (pending, occupied) = pending_moves(&screens, &memos);
    if state.0.lock().map_err(|e| e.to_string())?.as_ref() == Some(&layout) {
        return Ok(Vec::new());
    }
    if pending.is_empty() {
        return Ok(vec![]);
    }
    let sources = xcap::Monitor::all().unwrap_or_default();
    for screen in &mut screens {
        screen.busy = capture(&sources, screen.bounds)
            .map(|img| busy_tiles(&img, screen.bounds, screen.step()));
    }
    let moves = plan(&screens, &memos, pending, occupied);
    for &(i, _, r) in &moves {
        layout.windows.insert(windows[i].label().to_owned(), r);
    }
    *state.0.lock().map_err(|e| e.to_string())? = Some(layout);
    Ok(moves
        .into_iter()
        .map(|(i, d, rect)| HostMove {
            window: windows[i].clone(),
            rect,
            scale: screens[d].scale,
            work_y: monitors[d].work_area().position.y,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i64, y: i64, w: i64, h: i64) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }
    fn display(x: i64) -> Screen {
        let bounds = rect(x, 0, 144, 96);
        Screen {
            bounds,
            area: bounds,
            scale: 1.,
            gutters: vec![],
            busy: Some(vec![bounds]),
            apps: vec![],
        }
    }
    fn memo(x: i64, y: i64) -> Memo {
        Memo {
            rect: rect(x, y, 48, 48),
            home: 0,
            fixed: false,
            stack_size: placement::Item {
                width: 48,
                min: 48,
                max: 48,
            },
        }
    }
    fn run(s: &[Screen], m: &[Memo]) -> Vec<(usize, usize, Rect)> {
        let (pending, occupied) = pending_moves(s, m);
        plan(s, m, pending, occupied)
    }
    #[test]
    fn third_monitor_receives_only_overlap_then_second_run_is_idle() {
        let mut screens = [display(0), display(144), display(-144)];
        screens[2].busy = Some(vec![]);
        let mut memos = [memo(0, 0), memo(0, 0), memo(72, 0)];
        let moves = run(&screens, &memos);
        assert_eq!(moves.len(), 1);
        let (i, d, r) = moves[0];
        assert_eq!((i, d), (1, 2));
        assert!(r.fits(screens[d].area));
        memos[i].rect = r;
        memos[i].home = d;
        assert!(run(&screens, &memos).is_empty());
    }
    #[test]
    fn fixed_wins_and_bottom_right_stack_skips_a_protected_slot() {
        let mut m = [memo(0, 0), memo(0, 0), memo(96, 48), memo(0, 0)];
        m[1].fixed = true;
        m[2].fixed = true;
        assert_eq!(
            run(&[display(0), display(144), display(-144)], &m),
            [(0, 0, rect(96, 0, 48, 48)), (3, 0, rect(48, 48, 48, 48))]
        );
    }
    #[test]
    fn mixed_plan_preserves_clear_destination_and_stacks_only_remainder() {
        let mut screens = [display(0), display(144)];
        screens[1].area = rect(144, 0, 48, 48);
        screens[1].busy = Some(vec![]);
        assert_eq!(
            run(&screens, &[memo(0, 0), memo(0, 0), memo(0, 0)]),
            [(1, 1, rect(144, 0, 48, 48)), (2, 0, rect(96, 48, 48, 48))]
        );
    }
    #[test]
    fn touching_column_uses_empty_secondary_without_churning_on_repeat() {
        let mut screens = [display(0), display(144)];
        screens[1].busy = Some(vec![]);
        let mut m = [memo(96, 48), memo(96, 0)];
        let moves = run(&screens, &m);
        assert_eq!(moves.len(), 1);
        let (i, d, r) = moves[0];
        assert_eq!((i, d), (1, 1));
        m[i].home = d;
        m[i].rect = r;
        assert!(run(&screens, &m).is_empty());
        assert!(run(&[display(0)], &[memo(96, 48), memo(96, 0)]).is_empty());
    }
    #[test]
    fn same_display_preferred_and_no_capture_still_stacks_offscreen_window() {
        let mut screens = [display(0), display(-144)];
        screens[0].busy = Some(vec![]);
        screens[1].busy = Some(vec![]);
        assert_eq!(run(&screens, &[memo(-8, 0)])[0].1, 0);
        screens.iter_mut().for_each(|s| s.busy = None);
        assert_eq!(
            run(&screens, &[memo(-8, 0)]),
            [(0, 0, rect(96, 48, 48, 48))]
        );
    }
    #[test]
    fn blank_pixels_inside_app_force_secondary_then_original_bottom_right() {
        let mut screens = [display(0), display(144), display(-144)];
        for s in &mut screens {
            s.busy = Some(vec![]);
            s.apps = vec![s.area];
        }
        screens[2].apps.clear();
        let notes = [memo(0, 0)];
        let result = run(&screens, &notes);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 2);
        screens[2].apps.push(screens[2].area);
        assert_eq!(run(&screens, &notes), [(0, 0, rect(96, 48, 48, 48))]);
    }
    #[test]
    fn active_window_margin_is_second_to_desktop_and_requires_clear_pixels() {
        let mut screens = [display(0), display(144)];
        let gutter = rect(72, 0, 48, 48);
        screens[0].apps.push(screens[0].area);
        screens[0].gutters.push(gutter);
        for screen in &mut screens {
            screen.busy = Some(vec![]);
        }
        let notes = [memo(0, 0)];
        assert_eq!(run(&screens, &notes)[0].1, 1);
        screens[1].apps.push(screens[1].area);
        assert_eq!(run(&screens, &notes), [(0, 0, gutter)]);
        assert!(run(&screens, &[memo(72, 0)]).is_empty());
        screens[0].busy = Some(vec![gutter]);
        assert_eq!(run(&screens, &notes), [(0, 0, rect(96, 48, 48, 48))]);
    }
    #[test]
    fn image_tiles_map_retina_pixels_to_negative_screen_coordinates() {
        let screen = rect(-96, -48, 96, 48);
        let img = RgbaImage::from_fn(192, 96, |x, _| {
            xcap::image::Rgba([if x < 48 && x % 8 < 4 { 0 } else { 255 }; 4])
        });
        assert_eq!(
            busy_tiles(&img, screen, 24),
            [rect(-96, -48, 24, 24), rect(-96, -24, 24, 24)]
        );
    }
}
