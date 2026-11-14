(() => {
  const svg=document.querySelector('svg'), path=svg.querySelector('.connector');
  const text=svg.querySelector('text'), box=svg.querySelector('rect'), group=svg.querySelector('.label');
  let labelKey='',sizeKey='',painted='',latestGeneration=0;
  const stats=window.relationRenderStats={updates:0,frames:0,pending:false};
  window.drawRelation=(payload,generation)=>{
    const {width,height,points,label,scale}=payload;
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
      const [a,b,c,d]=points;
      path.setAttribute('d',`M${a} C${b} ${c} ${d}`);
      group.setAttribute('transform',`translate(${(a[0]+3*b[0]+3*c[0]+d[0])/8} ${(a[1]+3*b[1]+3*c[1]+d[1])/8})`);
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
