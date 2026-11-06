"""Read a named control only inside a smoke test's verified HWND.

Uses Windows' built-in UI Automation assemblies. Reading bounds never invokes
the control; callers still hit-test and click their own window explicitly.
"""
import json
import os
import subprocess


def control_rect(handle, name):
    command = r'''
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]([long]$env:GIBBON_UIA_HWND))
$condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty, $env:GIBBON_UIA_NAME)
$element = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
if ($null -eq $element) { throw 'Test control not found' }
$rect = $element.Current.BoundingRectangle
@{x=$rect.X; y=$rect.Y; width=$rect.Width; height=$rect.Height;
  offscreen=$element.Current.IsOffscreen; enabled=$element.Current.IsEnabled} | ConvertTo-Json -Compress
'''
    result = subprocess.run(
        ['powershell.exe', '-NoProfile', '-Command', command],
        env={**os.environ, 'GIBBON_UIA_HWND': str(int(handle)), 'GIBBON_UIA_NAME': name},
        capture_output=True, encoding='utf-8', timeout=20,
    )
    assert result.returncode == 0, result.stderr
    rect = json.loads(result.stdout)
    assert not rect['offscreen'] and rect['enabled'], (name, rect)
    assert rect['width'] > 0 and rect['height'] > 0, (name, rect)
    return rect


def control_center(handle, name):
    rect = control_rect(handle, name)
    return rect['x'] + rect['width'] / 2, rect['y'] + rect['height'] / 2
