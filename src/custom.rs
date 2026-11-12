//! Shared, bounded custom-canvas format. Coordinates use the 1920 x 462 canvas.
pub const SENSORS: &[&str] = &["cpu_temp", "cpu_load", "cpu_power", "cpu_clock", "cpu_fan", "uptime", "network_down", "network_up", "gpu_temp", "gpu_hotspot", "gpu_load", "gpu_power", "gpu_fan", "gpu_clock", "vram", "fps", "pump_fan", "case_fan", "ram_used", "ram_free", "ssd_temp", "disk_used", "disk_read", "disk_write", "clock", "date", "weather", "music"];

#[derive(Clone, Debug, PartialEq)]
pub struct Widget {
    pub sensor: String, pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub size: f32, pub color: [u8; 3], pub panel: bool,
}
impl Widget {
    pub fn new(sensor: &str) -> Self {
        Self { sensor: sensor.into(), x: 40., y: 40., w: 300., h: 150., size: 56., color: [236,238,242], panel: true }
    }
    pub fn bound(&mut self) {
        self.w = self.w.clamp(120.,1920.); self.h = self.h.clamp(if self.sensor.starts_with("group_"){64.}else{90.},462.);
        self.x = self.x.clamp(0.,1920.-self.w); self.y = self.y.clamp(0.,462.-self.h);
        self.size = self.size.clamp(12.,(self.h-40.).min(160.));
    }
}
pub fn parse(text: &str) -> Result<Vec<Widget>, String> {
    if text.trim().is_empty() { return Ok(Vec::new()); }
    let mut out = Vec::new();
    for row in text.split(';') {
        if out.len() >= 48 { return Err("At most 48 widgets are supported".into()); }
        let f: Vec<_> = row.trim().split(',').collect();
        if f.len()!=9 || !(SENSORS.contains(&f[0]) || ["group_cpu","group_gpu","group_mem","group_time","group_weather","group_music"].contains(&f[0])) { return Err("Invalid custom widget or sensor".into()); }
        let num = |i: usize| -> Result<f32,String> { f[i].parse::<f32>().ok().filter(|n| n.is_finite()).ok_or_else(|| "Widget coordinates must be finite numbers".into()) };
        let hex = f[7];
        if hex.len()!=6 || !hex.is_ascii() { return Err("Invalid widget color".into()); }
        let mut color=[0;3];
        for i in 0..3 { color[i]=u8::from_str_radix(&hex[i*2..i*2+2],16).map_err(|_| "Invalid widget color")?; }
        let mut w=Widget {sensor:f[0].into(), x:num(1)?,y:num(2)?,w:num(3)?,h:num(4)?,size:num(5)?,panel:match f[6] {"1"=>true,"0"=>false,_=>return Err("Invalid panel flag".into())},color};
        if f[8]!="v1" {return Err("Unsupported custom widget version".into());}
        w.bound(); out.push(w);
    }
    Ok(out)
}
pub fn encode(widgets: &[Widget]) -> String {
    widgets.iter().map(|w|format!("{},{:.0},{:.0},{:.0},{:.0},{:.0},{},{:02x}{:02x}{:02x},v1",w.sensor,w.x,w.y,w.w,w.h,w.size,u8::from(w.panel),w.color[0],w.color[1],w.color[2])).collect::<Vec<_>>().join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn built_in_theme_copies_keep_all_original_panels() {
        for key in ["axis","ledger","duet","orbit","atelier","panorama","cinema"] {
            let panels=theme_template(key);
            assert_eq!(panels.len(),6);
            assert_eq!(parse(&encode(&panels)).unwrap(),panels);
            for panel in panels {assert!(panel.x+panel.w<=1920. && panel.y+panel.h<=462.);}
        }
        assert!(theme_template("unknown").is_empty());
    }
    #[test] fn rejects_unbounded_and_malformed_inputs() {
        assert!(parse("cpu_temp,NaN,0,300,150,40,1,ffffff,v1").is_err());
        assert!(parse("fake,0,0,300,150,40,1,ffffff,v1").is_err());
        assert!(parse("cpu_temp,0,0,300,150,40,1,éffff,v1").is_err());
        let w=parse("cpu_temp,9999,-5,300,150,999,1,ffffff,v1").unwrap();
        assert_eq!((w[0].x,w[0].y,w[0].size),(1620.,0.,110.));
    }
    #[test] fn preset_roundtrip_preserves_sensor_and_style() {
        let w=vec![Widget::new("gpu_hotspot"),Widget::new("music")];
        assert_eq!(parse(&encode(&w)).unwrap(),w);
    }
}

include!("dashboard/templates.rs");
