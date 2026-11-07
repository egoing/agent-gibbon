// Issue #86 전용 외부 WinForms 검증 앱. 제품 코드가 아니다.
using System;
using System.Drawing;
using System.IO;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Windows.Forms;

class TestWindow : Form {
    string dir, role;
    TextBox editor;
    Label status;
    int clicks;
    int moveDelay;
    bool ready;
    protected override void WndProc(ref Message message) {
        if (ready && moveDelay > 0 && message.Msg == 0x46) System.Threading.Thread.Sleep(moveDelay);
        base.WndProc(ref message);
    }
    void Log(string name, string value) {
        File.AppendAllText(Path.Combine(dir, role + "-events.tsv"),
            DateTime.UtcNow.ToString("o") + "\t" + name + "\t" + value.Replace("\r", "").Replace("\n", "\\n") + "\n");
    }
    TestWindow(string output, string name, int delay) {
        moveDelay = delay;
        dir = output; role = name;
        Text = "ISSUE86-PRODUCT-" + role + "-task6b023e9f";
        StartPosition = FormStartPosition.Manual;
        Bounds = role == "A" ? new Rectangle(80, 100, 640, 420) : new Rectangle(140, 140, 820, 580);
        MinimumSize = new Size(360, 240);
        AutoScaleMode = AutoScaleMode.None;
        BackColor = role == "A" ? Color.LightCyan : Color.MistyRose;
        var title = new Label { Text = Text + "\nDedicated external process / ordinary WinForms editor", Location = new Point(20,15), Size = new Size(560,65), Font = new Font("Consolas", 14) };
        editor = new TextBox { Name="PoCEditor", AccessibleName="PoCEditor", Multiline=true, Location=new Point(30,100), Size=new Size(500,120), Font=new Font("Consolas",16) };
        var button = new Button { Text="CLICK PROBE", AccessibleName="ClickProbe", Location=new Point(30,240), Size=new Size(170,45) };
        status = new Label { Text="clicks=0", Location=new Point(220,250), Size=new Size(300,35), Font=new Font("Consolas",14) };
        Controls.AddRange(new Control[] { title, editor, button, status });
        editor.MouseDown += (s,e) => Log("editor-mousedown", e.Button + " " + e.X + "," + e.Y);
        editor.KeyDown += (s,e) => Log("editor-keydown", e.KeyCode.ToString());
        editor.KeyPress += (s,e) => Log("editor-keypress", ((int)e.KeyChar).ToString());
        editor.TextChanged += (s,e) => { File.WriteAllText(Path.Combine(dir, role + "-document.txt"),editor.Text); Log("document",editor.Text); };
        button.MouseDown += (s,e) => Log("button-mousedown", e.Button.ToString());
        button.Click += (s,e) => { clicks++; status.Text="clicks="+clicks; Log("button-click", clicks.ToString()); };
        Activated += (s,e) => Log("activated", "true");
        Deactivate += (s,e) => Log("deactivated", "true");
        FormClosed += (s,e) => Log("closed", "true");
        Shown += (s,e) => {
            Log("cwd", Environment.CurrentDirectory);
            File.WriteAllText(Path.Combine(dir, role+"-ready.json"), "{\"pid\":"+Process.GetCurrentProcess().Id+",\"hwnd\":"+Handle.ToInt64()+",\"editor\":"+editor.Handle.ToInt64()+",\"button\":"+button.Handle.ToInt64()+"}");
            Log("shown", Bounds.ToString());
            ready = true;
        };
    }
    [DllImport("user32.dll")] static extern bool SetProcessDPIAware();
    [STAThread] static void Main(string[] args) {
        if (args.Length > 2) System.Threading.Thread.Sleep(Int32.Parse(args[2]));
        SetProcessDPIAware();
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new TestWindow(args[0],args[1],args.Length > 3 ? Int32.Parse(args[3]) : 0));
    }
}
