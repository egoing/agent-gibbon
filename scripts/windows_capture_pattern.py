"""Asymmetric black/white native fixture and read-only PNG coordinate checks.

Call add_pattern inside the fixture process on its HWND owner thread. All sizes
and origins are physical pixels in a per-monitor-DPI-aware process. This module
does not capture screens, send input, create files, or access the clipboard.
"""
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import subprocess

CELL_WIDTH = 48
CELL_HEIGHT = 40
_PALETTES = {}


def _palette(fixture_size):
    key = tuple(map(int, fixture_size))
    if key in _PALETTES:
        return _PALETTES[key]
    user = C.WinDLL('user32')
    user.GetSysColor.argtypes = [C.c_int]
    user.GetSysColor.restype = W.DWORD
    return user.GetSysColor(6), user.GetSysColor(5)


def calibrate_palette(user, root_hwnd, fixture_size):
    """Read actual visible fixture colors; themes can differ from GetSysColor(6)."""
    width, height = map(int, fixture_size)
    column, row = (width // 2) // CELL_WIDTH, (height // 2) // CELL_HEIGHT
    left, top, w, h = _rectangle(column, row)
    user.ClientToScreen.argtypes = [W.HWND, C.POINTER(W.POINT)]
    user.WindowFromPoint.argtypes = [W.POINT]
    user.WindowFromPoint.restype = W.HWND
    user.GetAncestor.argtypes = [W.HWND, W.UINT]
    user.GetAncestor.restype = W.HWND
    origin = W.POINT()
    assert user.ClientToScreen(root_hwnd, C.byref(origin))
    user.GetDC.argtypes = [W.HWND]
    user.GetDC.restype = W.HDC
    user.ReleaseDC.argtypes = [W.HWND, W.HDC]
    gdi = C.WinDLL('gdi32')
    gdi.GetPixel.argtypes = [W.HDC, C.c_int, C.c_int]
    gdi.GetPixel.restype = W.DWORD
    dc = user.GetDC(None)
    try:
        colors = []
        for x, y in [(left + w // 2, top + h // 2), (2, 2)]:
            point = W.POINT(origin.x + x, origin.y + y)
            assert user.GetAncestor(user.WindowFromPoint(point), 2) == root_hwnd, 'Fixture palette is obscured'
            color = gdi.GetPixel(dc, point.x, point.y)
            assert color != 0xffffffff, 'Cannot sample actual fixture color'
            colors.append(color)
        assert colors[0] != colors[1], 'Fixture must have two distinct colors'
        _PALETTES[(width, height)] = tuple(colors)
        return tuple(colors)
    finally:
        user.ReleaseDC(None, dc)


def _limits(fixture_size):
    width, height = map(int, fixture_size)
    assert width >= 640 and height >= 480, 'Fixture must be at least 640 x 480 physical pixels'
    # Limit native child windows to the central selection plus a generous guard
    # region; a full monitor need not own thousands of tiny HWNDs.
    return (max(0, (width // 2 - 320) // CELL_WIDTH),
            min((width - 1) // CELL_WIDTH, (width // 2 + 320) // CELL_WIDTH),
            max(0, (height // 2 - 220) // CELL_HEIGHT),
            min((height - 1) // CELL_HEIGHT, (height // 2 + 220) // CELL_HEIGHT))


def _rectangle(column, row):
    return (column * CELL_WIDTH + 6 + (row % 3) * 3,
            row * CELL_HEIGHT + 5 + (column % 2) * 4,
            19 + (column % 3) * 3,
            11 + (row % 2) * 5)


def expected_color(x, y, fixture_size):
    """Expected native COLORREF at a fixture-local pixel, including system theme."""
    width, height = map(int, fixture_size)
    dark, light = _palette(fixture_size)
    assert 0 <= x < width and 0 <= y < height, 'Sample outside the safe fixture'
    low_column, high_column, low_row, high_row = _limits(fixture_size)
    column, row = int(x) // CELL_WIDTH, int(y) // CELL_HEIGHT
    if low_column <= column <= high_column and low_row <= row <= high_row:
        left, top, w, h = _rectangle(column, row)
        if left <= x < min(left + w, width) and top <= y < min(top + h, height):
            return dark
    return light


def add_pattern(user, root_hwnd, physical_width, physical_height):
    """Add black STATIC child rectangles; return HWNDs owned by the root.

    The parent must already be an opaque white STATIC window belonging to this
    process. Parent destruction owns child cleanup; do not destroy them from a
    different thread. WindowFromPoint -> GetAncestor(GA_ROOT) remains root_hwnd.
    """
    width, height = int(physical_width), int(physical_height)
    low_column, high_column, low_row, high_row = _limits((width, height))
    user.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
    user.GetWindowThreadProcessId.restype = W.DWORD
    owner = W.DWORD()
    thread = user.GetWindowThreadProcessId(root_hwnd, C.byref(owner))
    kernel = C.WinDLL('kernel32', use_last_error=True)
    kernel.GetCurrentThreadId.restype = W.DWORD
    assert owner.value == os.getpid() and thread == kernel.GetCurrentThreadId(), 'Fixture owner thread required'
    user.GetClassNameW.argtypes = [W.HWND, W.LPWSTR, C.c_int]
    name = C.create_unicode_buffer(128)
    assert user.GetClassNameW(root_hwnd, name, len(name)) and name.value.upper() == 'STATIC'
    user.GetWindowLongW.argtypes = [W.HWND, C.c_int]
    user.GetWindowLongW.restype = C.c_long
    user.SetWindowLongW.argtypes = [W.HWND, C.c_int, C.c_long]
    user.SetWindowLongW.restype = C.c_long
    style = user.GetWindowLongW(root_hwnd, -16)
    assert style & 0x1f == 6, 'Fixture root must use SS_WHITERECT'
    # Prevent a parent repaint from covering children. Sibling clipping keeps
    # static child painting isolated even if their placement changes later.
    previous = user.SetWindowLongW(root_hwnd, -16, style | 0x02000000)  # WS_CLIPCHILDREN
    # This visible popup's previous style is nonzero, so zero signals failure.
    assert previous != 0, 'Cannot enable child clipping'
    user.CreateWindowExW.argtypes = [W.DWORD, W.LPCWSTR, W.LPCWSTR, W.DWORD,
        C.c_int, C.c_int, C.c_int, C.c_int, W.HWND, W.HMENU, W.HINSTANCE, C.c_void_p]
    user.CreateWindowExW.restype = W.HWND
    children = []
    for row in range(low_row, high_row + 1):
        for column in range(low_column, high_column + 1):
            x, y, w, h = _rectangle(column, row)
            if x >= width or y >= height:
                continue
            # WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | SS_BLACKRECT.
            child = user.CreateWindowExW(0, 'STATIC', None, 0x54000004,
                x, y, min(w, width - x), min(h, height - y), root_hwnd, None, None, None)
            assert child, 'Cannot create safe pattern child'
            children.append(child)
    user.RedrawWindow.argtypes = [W.HWND, C.c_void_p, W.HRGN, W.UINT]
    assert user.RedrawWindow(root_hwnd, None, None, 0x0181), 'Cannot paint safe pattern'
    return children


def sample_points(selection_origin, fixture_origin, fixture_size, selection_size=(440, 260)):
    """Return PNG-local (x, y, COLORREF) samples away from all hard edges."""
    width, height = map(int, selection_size)
    ox = int(selection_origin[0]) - int(fixture_origin[0])
    oy = int(selection_origin[1]) - int(fixture_origin[1])
    fw, fh = map(int, fixture_size)
    assert width >= 16 and height >= 16
    assert 1 <= ox and 1 <= oy and ox + width < fw and oy + height < fh, 'Selection must stay inside fixture'
    samples = []
    for y in range(2, height - 2, 4):
        for x in range(2, width - 2, 4):
            local_x, local_y = ox + x, oy + y
            color = expected_color(local_x, local_y, fixture_size)
            if all(expected_color(local_x + dx, local_y + dy, fixture_size) == color
                   for dx, dy in [(-1, 0), (1, 0), (0, -1), (0, 1)]):
                samples.append((x, y, color))
    dark, light = _palette(fixture_size)
    assert sum(color == dark for _, _, color in samples) > 20
    assert sum(color == light for _, _, color in samples) > 20
    return samples


def verify_png(path, selection_origin, fixture_origin, fixture_size, selection_size=(440, 260)):
    """Check exact physical crop dimensions and thousands of expected pixels.

    Uses Windows' existing System.Drawing through PowerShell. JSON samples travel
    over stdin, avoiding command expansion and Windows environment-size limits.
    Only the caller-supplied fixture crop is read, and no image is written.
    """
    path = Path(path).resolve(strict=True)
    samples = [(x, y, ((color & 255) << 16) | (color & 0xff00) | ((color >> 16) & 255))
               for x, y, color in sample_points(selection_origin, fixture_origin, fixture_size, selection_size)]
    command = r'''
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$payload = [Console]::In.ReadToEnd() | ConvertFrom-Json
$bitmap = [Drawing.Bitmap]::FromFile($env:GIBBON_PATTERN_PNG)
try {
  $mismatches = 0
  $examples = @()
  $dimensions = ($bitmap.Width -eq $payload.width -and $bitmap.Height -eq $payload.height)
  if ($dimensions) {
    foreach ($point in $payload.samples) {
      $pixel = $bitmap.GetPixel([int]$point[0], [int]$point[1])
      $rgb = $pixel.ToArgb() -band 0xffffff
      if ($pixel.A -ne 255 -or $rgb -ne $point[2]) {
        $mismatches++
        if ($examples.Count -lt 8) {
          $examples += @{x=$point[0]; y=$point[1]; expected=$point[2]; actual=$rgb; alpha=$pixel.A}
        }
      }
    }
  }
  @{width=$bitmap.Width; height=$bitmap.Height; dimensions=$dimensions;
    samples=$payload.samples.Count; mismatches=$mismatches; examples=$examples} | ConvertTo-Json -Depth 5 -Compress
} finally { $bitmap.Dispose() }
'''
    result = subprocess.run(['powershell.exe', '-NoProfile', '-Command', command],
        env={**os.environ, 'GIBBON_PATTERN_PNG': str(path)},
        input=json.dumps({'width': int(selection_size[0]), 'height': int(selection_size[1]), 'samples': samples}),
        capture_output=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0, result.stderr
    report = json.loads(result.stdout)
    assert report['dimensions'], ('Capture dimensions differ from physical selection', report)
    assert report['mismatches'] == 0, ('Capture origin/scale/colors differ from native fixture', report)
    return report
