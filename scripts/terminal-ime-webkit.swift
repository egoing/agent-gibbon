// Opt-in macOS WebKit rendering check with synthetic InputEvents, not physical IME.
// Usage: swift scripts/terminal-ime-webkit.swift <repo-root> <output-directory>
import AppKit
import WebKit

let root = CommandLine.arguments[1]
let output = CommandLine.arguments[2]
try FileManager.default.createDirectory(atPath: output, withIntermediateDirectories: true)
func asset(_ path: String) -> String { try! String(contentsOfFile: root + "/ui/" + path, encoding: .utf8) }
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let web = WKWebView(frame: NSRect(x: 0, y: 0, width: 640, height: 240))
let window = NSWindow(contentRect: web.frame, styleMask: [.borderless], backing: .buffered, defer: false)
window.contentView = web
// No activation or key-window request. Snapshot only this independent test view.
window.orderFrontRegardless()
class Check: NSObject, WKNavigationDelegate {
    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        webView.evaluateJavaScript("""
        (() => {
          const sent=[];window.imeSent=sent;
          const terminal=new Terminal({cols:60,rows:10,fontFamily:'Menlo, Monaco, monospace',fontSize:13,theme:{background:'#20252e',foreground:'#f0f0f0'}});
          terminal.open(document.getElementById('fixture'));
          window.installTerminalIme(terminal,s=>sent.push(s));
          const input=value=>{terminal.textarea.value=value;terminal.textarea.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertReplacementText',data:value}));};
          for(const value of ['ㅎ','하','한','한ㄱ','한그','한글'])input(value);
          const preview=terminal.element.querySelector('.composition-view');
          window.commitCheck=()=>{input('한글 ');return sent.length===1&&sent[0]==='한글 '&&!preview.classList.contains('active')&&!preview.classList.contains('gibbon-ime-fallback');};
          window.spacingCheck=async()=>{
            const results=[];
            const frame=()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
            const write=text=>new Promise(resolve=>terminal.write(text,resolve));
            for(const fontSize of [13,14,16,18]) {
              terminal.options.fontSize=fontSize;
              terminal.resize(fontSize===14?47:60,10);
              await frame();
              for(const word of ['한글','안녕하세요','하나','ㅎㅏ','한a글']) {
                await write('\\r\\x1b[2K> ');await frame();
                input(word);await frame();
                const screen=terminal.element.querySelector('.xterm-screen');
                const cellWidth=screen.getBoundingClientRect().width/terminal.cols;
                const beforeWidth=preview.getBoundingClientRect().width;
                const beforeCurrentX=preview.querySelector('.gibbon-ime-current').getBoundingClientRect().left-screen.getBoundingClientRect().left;
                const start=terminal.buffer.active.cursorX;
                input(word+' ');
                await write(sent.at(-1));await frame();
                const line=terminal.buffer.active.getLine(terminal.buffer.active.cursorY);
                const end=terminal.buffer.active.cursorX-1;
                let last=start;
                for(let col=start;col<end;col++)if(line.getCell(col).getWidth()>0)last=col;
                const committedWidth=(end-start)*cellWidth;
                // Compare actual rendered glyph ranges as well as public buffer cell widths.
                const row=terminal.element.querySelector('.xterm-rows').children[terminal.buffer.active.cursorY];
                const walker=document.createTreeWalker(row,NodeFilter.SHOW_TEXT);let node,index=0;
                const origins=[];
                while(node=walker.nextNode())for(let offset=0;offset<node.textContent.length;offset++,index++){
                  if(index<2||index>=2+word.length)continue;
                  const range=document.createRange();range.setStart(node,offset);range.setEnd(node,offset+1);
                  origins.push(range.getBoundingClientRect().left-screen.getBoundingClientRect().left);
                }
                results.push({fontSize,cols:terminal.cols,word,cellWidth,beforeWidth,committedWidth,widthDelta:beforeWidth-committedWidth,currentDelta:beforeCurrentX-last*cellWidth,renderedCurrentDelta:beforeCurrentX-origins.at(-1)});
              }
            }
            terminal.options.fontSize=13;terminal.resize(60,10);await frame();
            await write('\\x1b[2J\\x1b[H> ');await frame();input('안녕하세요');await frame();
            window.spacingComparison=async()=>{input('안녕하세요 ');await write('\\r\\n> '+sent.at(-1));await frame();};
            return JSON.stringify(results);
          };
          return true;
        })()
        """) { value, error in
            if let error = error { fputs("\(error)\n", stderr); exit(1) }
            self.capture(webView)
        }
    }
    func capture(_ webView: WKWebView) {
        // Let the fixture's two animation frames settle before inspecting and capturing.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            webView.evaluateJavaScript("JSON.stringify({buffered:window.imeSent?.length===0,text:document.querySelector('.composition-view').textContent,background:getComputedStyle(document.querySelector('.composition-view')).backgroundColor,current:document.querySelector('.gibbon-ime-current')?.textContent,currentBackground:document.querySelector('.gibbon-ime-current')?getComputedStyle(document.querySelector('.gibbon-ime-current')).backgroundColor:null})") { value,error in
                guard error == nil, let json = value as? String else { fputs("WebKit fixture failed\n",stderr);exit(1) }
                print(json)
                let data = json.data(using:.utf8)!
                try! data.write(to:URL(fileURLWithPath:output+"/render.json"))
                let result = try! JSONSerialization.jsonObject(with:data) as! [String:Any]
                let passed = result["buffered"] as? Bool == true && result["text"] as? String == "한글" && result["background"] as? String == "rgb(32, 37, 46)" && result["current"] as? String == "글" && result["currentBackground"] as? String == "rgb(0, 0, 0)"
                let snapshot = WKSnapshotConfiguration()
                snapshot.rect = NSRect(x:0,y:0,width:640,height:72)
                webView.takeSnapshot(with:snapshot) { image,error in
                    guard let image=image, let tiff=image.tiffRepresentation, let png=NSBitmapImageRep(data:tiff)?.representation(using:.png,properties:[:]) else { fputs("Snapshot failed\n",stderr);exit(1) }
                    try! png.write(to:URL(fileURLWithPath:output+"/composition.png"))
                    webView.evaluateJavaScript("window.commitCheck()") { committed,error in
                        guard passed, error == nil, committed as? Bool == true else { fputs("FAIL: highlight or commit check\n",stderr);exit(1) }
                        guard #available(macOS 11.0, *) else { fputs("Spacing check requires macOS 11+\n",stderr);exit(1) }
                        webView.callAsyncJavaScript("return await window.spacingCheck()", arguments:[:], in:nil, in:.page) { result in
                            switch result {
                            case .failure(let error): fputs("\(error)\n",stderr);exit(1)
                            case .success(let value):
                                guard let json=value as? String, let data=json.data(using:.utf8), let rows=(try? JSONSerialization.jsonObject(with:data)) as? [[String:Any]] else { exit(1) }
                                print(json)
                                try! data.write(to:URL(fileURLWithPath:output+"/spacing.json"))
                                let aligned = rows.count == 20 && rows.allSatisfy { row in
                                    // CSS layout quantizes fractional widths; xterm's merged text
                                    // ranges can additionally differ by less than one CSS pixel.
                                    ["widthDelta","currentDelta"].allSatisfy { abs(row[$0] as? Double ?? 999) < 0.1 }
                                      && abs(row["renderedCurrentDelta"] as? Double ?? 999) < 1
                                }
                                guard aligned else { fputs("FAIL: composition and committed glyph positions differ\n",stderr);exit(1) }
                                webView.takeSnapshot(with:snapshot) { image,error in
                                    guard let image=image, let tiff=image.tiffRepresentation, let png=NSBitmapImageRep(data:tiff)?.representation(using:.png,properties:[:]) else { exit(1) }
                                    try! png.write(to:URL(fileURLWithPath:output+"/spacing-pending.png"))
                                    webView.callAsyncJavaScript("await window.spacingComparison(); return true",arguments:[:],in:nil,in:.page) { comparison in
                                        if case .failure(let error)=comparison { fputs("\(error)\n",stderr);exit(1) }
                                        webView.takeSnapshot(with:snapshot) { image,error in
                                            guard let image=image, let tiff=image.tiffRepresentation, let png=NSBitmapImageRep(data:tiff)?.representation(using:.png,properties:[:]) else { exit(1) }
                                            try! png.write(to:URL(fileURLWithPath:output+"/spacing-committed.png"))
                                            print("PASS: WebKit fallback highlight, commit, and 20 spacing cases (synthetic events)")
                                            exit(0)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
let check = Check()
web.navigationDelegate = check
let html = "<html><head><style>"+asset("vendor/xterm.css")+asset("terminal.css")+"body{margin:0;background:#20252e}#fixture{height:240px}</style></head><body><div id='fixture'></div><script>"+asset("vendor/xterm.js")+"</script><script>"+asset("terminal-ime.js")+"</script></body></html>"
web.loadHTMLString(html,baseURL:nil)
DispatchQueue.main.asyncAfter(deadline:.now()+15) { fputs("WebKit check timed out\n",stderr);exit(1) }
app.run()
