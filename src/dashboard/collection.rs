//! Aperture collection. Geometry is generated from design/collection.json.
//! Text uses cap heights, bounded widths and the same six-reading model as Grid.
use super::*;

struct Widget { kind: &'static str, r: R, mode: &'static str }
include!("geometry.rs");

fn surface(p: &mut Painter, r: R, ink: &Ink, k: f32) {
    p.layer.fill_rrect_solid(r, 16.0 * k, ink.card, 255);
}

fn reading(p: &mut Painter, c: &Cell, r: R, cap: f32, ink: &Ink, k: f32, text_scale: f32) {
    if c.label.is_empty(){return;}
    let (x,y,w,_) = r;
    caps(p,x,y+16.0*k,c.label,16.0*k,ink.muted,k);
    let size = vu_fit(p,&c.value,&c.unit,cap*text_scale,w,W::Regular);
    let color = if c.value == "--" { ink.dim } else if c.color == th().text { ink.text } else { c.color };
    vu(p,x,y+25.0*k+cap,&c.value,&c.unit,size,W::Regular,color,ink.muted);
}

fn hardware(p: &mut Painter, s: &Sec, r: R, mode: &str, ink: &Ink, k: f32, text_scale: f32) {
    let (x,y,w,h)=r;
    let pd=20.0*k;
    surface(p,r,ink,k);
    let x=x+pd;
    let y=y+pd;
    let iw=w-2.0*pd;
    caps(p,x,y+17.0*k,s.name,17.0*k,th().cpu[0],k);
    let side=if s.side.is_empty(){s.device.clone()}else{s.side.clone()};
    let side=ellipsize(p,&side,16.0*k,W::Regular,iw*0.58);
    if mode!="row" {text_r(p,x+iw,y+17.0*k,&side,16.0*k,W::Regular,ink.muted);}
    let hc=if mode=="row" {if h<120.0*k {52.0*k}else{70.0*k}} else if mode=="wide" {116.0*k} else if mode=="ring" {58.0*k} else if h<310.0*k {64.0*k} else {84.0*k};
    let hero_w=if mode=="row" {210.0*k} else if mode=="wide" {236.0*k} else {iw};
    let hc=hero_fit(p,&s.hero,s.hero_unit,hc,hero_w);
    let color=if s.hero_color==th().text {ink.text} else {s.hero_color};
    if mode=="ring" {
        let cx=x+iw/2.0;
        let cy=y+86.0*k;
        let rr=78.0*k;
        arc2(p,cx,cy,rr,5.0*k,-135.0,270.0,&|_| ink.muted,35);
        if let Some(f)=s.hero_frac {
            arc2(p,cx,cy,rr,5.0*k,-135.0,270.0*f.clamp(0.0,1.0),&|_| th().cpu[0],255);
        }
        let hw=p.width(&s.hero,hc,W::Light)+p.width(s.hero_unit,hc*0.31,W::Light);
        hero(p,cx-hw/2.0,cy+hc*0.45,&s.hero,s.hero_unit,hc,color,ink.muted);
    } else {
        hero(p,x,y+if mode=="row" {22.0*k+hc}else{34.0*k+hc},&s.hero,s.hero_unit,hc,color,ink.muted);
    }
    let (gx,gy,gw,gh,cols,vc)=match mode {
        "row" => (x+238.0*k,y+12.0*k,iw-238.0*k,h-54.0*k,6usize,32.0*k),
        "wide" => (x+270.0*k,y+25.0*k,iw-270.0*k,h-55.0*k,3usize,38.0*k),
        "column" => (x,y+154.0*k,iw,h-194.0*k,2usize,36.0*k),
        "ring" => (x,y+174.0*k,iw,h-217.0*k,2usize,32.0*k),
        _ if h<310.0*k => (x,y+106.0*k,iw,h-146.0*k,3usize,30.0*k),
        _ => (x,y+126.0*k,iw,h-166.0*k,3usize,40.0*k),
    };
    let rows=6/cols;
    let cw=gw/cols as f32;
    let rh=gh/rows as f32;
    for (i,c) in s.cells.iter().enumerate() {
        let cx=gx+(i%cols) as f32*cw;
        let cy=gy+(i/cols) as f32*rh;
        reading(p,c,(cx,cy,cw-16.0*k,rh),vc,ink,k,text_scale);
    }
    // A small real history line, below the hero and away from the readings.
    if mode=="wide" {
        if let Some((hist,max))=s.hist {
            trend(p,(x,y+h-38.0*k,hero_w,16.0*k),hist,max,s.colors,k);
        }
    }
}

fn clock(p: &mut Painter,r:R,v:&Vm,ink:&Ink,k:f32) {
    let (x,y,w,h)=r;
    surface(p,r,ink,k);
    if h<90.0*k {
        p.text(x+16.0*k,y+53.0*k,&v.hhmm,44.0*k,W::Light,ink.text,255);
        let date=ellipsize(p,&v.date,17.0*k,W::Regular,w*0.45);
        text_r(p,x+w-16.0*k,y+53.0*k,&date,17.0*k,W::Regular,ink.muted);return;
    }
    let cap=p.fit(&v.hhmm,if h<110.0*k {48.0*k}else{78.0*k},w-40.0*k,W::Light);
    let base=y+20.0*k+cap;
    p.text(x+20.0*k,base,&v.hhmm,cap,W::Light,ink.text,255);
    let date=format!("{} · {}",v.wd,v.date);
    let date=ellipsize(p,&date,16.0*k,W::Regular,w-40.0*k);
    p.text(x+20.0*k,y+h-10.0*k,&date,16.0*k,W::Regular,ink.muted,255);
}

fn weather(p:&mut Painter,r:R,v:&Vm,ink:&Ink,k:f32) {
    let (x,y,w,h)=r;
    surface(p,r,ink,k);
    let Some(wx)=&v.wx else {
        caps(p,x+20.0*k,y+34.0*k,"Weather",17.0*k,ink.muted,k);
        p.text(x+20.0*k,y+68.0*k,"Waiting for weather",20.0*k,W::Regular,ink.dim,255);
        return;
    };
    if h<90.0*k {
        let summary=format!("{} · {} · {}",wx.temp,wx.cond,wx.city);
        let summary=ellipsize(p,&summary,26.0*k,W::Regular,w-32.0*k);
        p.text(x+16.0*k,y+50.0*k,&summary,26.0*k,W::Regular,ink.text,255);return;
    }
    let compact=h<150.0*k && w>500.0*k;
    let head=if compact {w*0.42} else {w-40.0*k};
    let tall=h>200.0*k;
    let cap=if tall {72.0*k}else if compact {42.0*k}else{40.0*k};
    let temp=ellipsize(p,&wx.temp,cap,W::Regular,head-40.0*k);
    let tw=p.text(x+20.0*k,y+20.0*k+cap,&temp,cap,W::Regular,ink.text,255);
    let label=if compact {format!("{} · {}",wx.cond,wx.city)}else{wx.cond.clone()};
    let label=ellipsize(p,&label,17.0*k,W::Regular,head-40.0*k);
    p.text(x+20.0*k,y+if tall {124.0*k}else{80.0*k},&label,17.0*k,W::Regular,ink.muted,255);
    if !compact && h>200.0*k {
        let city=ellipsize(p,&wx.city,17.0*k,W::Regular,w-40.0*k);
        p.text(x+20.0*k,y+150.0*k,&city,17.0*k,W::Regular,ink.dim,255);
        weather_icon(p,x+(w-108.0*k)/2.0,y+188.0*k,108.0*k,wx.icon,wx.night);
    }
    let (fx,fy,fw)=if compact {(x+head,y+20.0*k,w-head-20.0*k)}else{(x+20.0*k,y+h-if tall{72.0*k}else{58.0*k},w-40.0*k)};
    wx_days(p,fx,fy,fw,v,16.0*k,if tall{30.0*k}else{24.0*k},ink,k);
    let _=tw;
}

fn album(p:&mut Painter,r:R,d:&Data,cover:Option<&Cover>,ink:&Ink,k:f32) {
    let (x,y,w,h)=r;
    surface(p,r,ink,k);
    let size=h.min(w);
    cover_or_note(p,cover,x+(w-size)/2.0,y,size,16.0*k,ink.dim,(ink.card,ink.bg));
    p.layer.fill_rrect_solid((x,y+h-108.0*k,w,108.0*k),16.0*k,ink.bg,235);
    let title_y=y+h-78.0*k;
    let lines=track_lines(d);
    let (title,artist)=lines.unwrap_or_else(||("Nothing playing".into(),"Your music, when you're ready".into()));
    title_line(p,x+20.0*k,title_y,w-40.0*k,&title,20.0*k,W::Medium,ink.text);
    let artist=ellipsize(p,&artist,16.0*k,W::Regular,w-40.0*k);
    p.text(x+20.0*k,title_y+23.0*k,&artist,16.0*k,W::Regular,ink.muted,255);
    if let Some((elapsed,total,f))=track_times(d) {
        bar(p,x+20.0*k,y+h-16.0*k,w-40.0*k,3.0*k,Some(f),th().cpu,(ink.muted,40));
        p.text(x+20.0*k,y+h-24.0*k,&elapsed,16.0*k,W::Regular,ink.muted,255);
        text_r(p,x+w-20.0*k,y+h-24.0*k,&total,16.0*k,W::Regular,ink.muted);
    }
}

pub(super) fn paint(layout:Layout,p:&mut Painter,w:f32,h:f32,_sc:&Scale,d:&Data,v:&Vm,hist:Hist,cover:Option<&Cover>)->bool {
    let Some(widgets)=geometry(layout) else {return false;};
    let k=(w/1920.0).min(h/462.0);
    let ink=Ink { card: th().panel, ..dark_ink() };
    let settings = opts().pro;
    let editing = settings.edit_layout == layout.name();
    let text_scale = if editing { settings.edit_text_scale } else { 1.0 };
    if !(settings.edit_background && editing) {
        rect(p,(0.0,0.0,w,h),ink.bg,255);
    }
    let mut secs=sections(d,v,hist);
    if settings.edit_layout==layout.name() {
        for (group,section) in ["cpu","gpu","mem"].iter().zip(secs.iter_mut()) {
            for cell in &mut section.cells {
                if settings.edit_hidden.split(',').any(|key|key==format!("{}:{}",group,cell.label.to_ascii_lowercase())) {cell.label="";}
            }
        }
    }
    for widget in widgets {
        let edit = if editing {
            settings.edit_widgets.iter()
                .find(|w| w.sensor.strip_prefix("group_") == Some(widget.kind))
        } else {
            None
        };
        if edit.is_some_and(|w|!w.panel) {continue;}
        let r=edit.map(|w|(w.x*k,w.y*k,w.w*k,w.h*k)).unwrap_or((widget.r.0*k,widget.r.1*k,widget.r.2*k,widget.r.3*k));
        let mut ink=ink;
        if let Some(w)=edit {if w.color!=[242,243,239] {ink.text=(w.color[0],w.color[1],w.color[2]);}}
        match widget.kind {
            "cpu"=>hardware(p,&secs[0],r,widget.mode,&ink,k,text_scale),
            "gpu"=>hardware(p,&secs[1],r,widget.mode,&ink,k,text_scale),
            "mem"=>hardware(p,&secs[2],r,widget.mode,&ink,k,text_scale),
            "time"=>clock(p,r,v,&ink,k),
            "weather"=>weather(p,r,v,&ink,k),
            "music"=> {
                if !settings.logo.is_empty() {
                    surface(p, r, &ink, k);
                    let size = (r.2.min(r.3) - 20.0 * k).max(1.0);
                    cover_or_note(p, cover, r.0 + (r.2-size)/2.0, r.1 + (r.3-size)/2.0,
                        size, 8.0*k, ink.dim, (ink.card, ink.bg));
                } else if widget.mode=="cover" {album(p,r,d,cover,&ink,k)}
                else {
                    surface(p,r,&ink,k);
                    music(p,d,cover,(r.0+16.0*k,r.1+16.0*k,r.2-32.0*k,r.3-32.0*k),18.0*k,&ink,k,false);
                }
            },
            _=>{}
        }
    }
    true
}
