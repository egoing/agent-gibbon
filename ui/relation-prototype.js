(() => {
  const svg=document.querySelector('svg'), path=svg.querySelector('.connector');
  const text=svg.querySelector('text'), box=svg.querySelector('rect'), group=svg.querySelector('.label');
  const marker=svg.querySelector('marker');
  let labelKey='',sizeKey='',painted='',latestGeneration=0;
  const stats=window.relationRenderStats={updates:0,frames:0,pending:false};
  window.drawRelation=(payload,generation)=>{
    const {width,height,points,label,scale,arrow='end'}=payload;
    stats.updates++;
    const key=JSON.stringify(payload);
    if(painted!==key){
      painted=key;stats.frames++;
      const size=`${width},${height},${scale}`;
      if(size!==sizeKey){
        sizeKey=size;svg.setAttribute('viewBox',`0 0 ${width} ${height}`);
        path.style.strokeWidth=2*scale;box.setAttribute('rx',4*scale);box.style.strokeWidth=scale;
      }
      const nextLabel=JSON.stringify([label,scale]);
      if(nextLabel!==labelKey){
        labelKey=nextLabel;text.textContent=label;text.style.fontSize=`${12*scale}px`;
        const b=text.getBBox();
        for(const [k,v] of Object.entries({x:b.x-7*scale,y:b.y-4*scale,width:b.width+14*scale,height:b.height+8*scale}))box.setAttribute(k,v);
        group.style.display=label?'':'none';
      }
      path.setAttribute('marker-start',arrow==='both'?'url(#arrow)':'none');
      path.setAttribute('marker-end',arrow==='none'?'none':'url(#arrow)');
      const curve=window.relationCurve(points,arrow,scale,marker);
      path.setAttribute('d',curve.d);
      group.setAttribute('transform',`translate(${curve.center.join(' ')})`);
    }
    // Confirm DOM geometry through a one-shot microtask. A hidden native WebView
    // may stop RAF entirely, so reveal must never wait for an invisible frame.
    latestGeneration=generation;stats.pending=true;
    Promise.resolve().then(()=>{
      if(latestGeneration!==generation)return;
      stats.pending=false;
      return window.__TAURI_INTERNALS__?.invoke('relation_ready',{generation});
    }).catch(()=>{});
  };
})();
