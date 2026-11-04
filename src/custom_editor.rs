use super::*;
use aperture::custom::{self,Widget};

impl App {
    pub(super) fn edit_current_theme(&mut self) {
        let key=self.get_or("pro_layout","axis");
        let widgets=custom::theme_template(&key);
        if widgets.is_empty() {self.save_error=Some("Choose an Aperture layout (Axis, Ledger, Duet, Orbit, Atelier, Panorama or Cinema) first.".into());return;}
        if self.get_or("pro_edit_layout","")!=key {self.set("pro_edit_widgets",&custom::encode(&widgets));self.unset("pro_edit_hidden");}
        self.set("pro_edit_layout",&key);
        self.set("pro_custom_enabled","false");self.page=Page::Canvas;
    }
    pub(super) fn page_canvas(&mut self, ui: &mut egui::Ui) {
        let builtin=!self.get_or("pro_edit_layout", "").is_empty() && self.get("pro_custom_enabled").as_deref()!=Some("true");
        section_title(ui,"Personalize your display","Start with a theme, then select a panel to move, resize or change its color.");
        ui.horizontal(|ui| {
            if ui.button("Edit current theme").clicked(){self.edit_current_theme();}
            if ui.button("Free canvas").clicked(){self.set("pro_custom_enabled","true");self.set("layout","pro");}
            if ui.button("Restore original theme").clicked(){self.unset("pro_edit_layout");self.unset("pro_edit_widgets");self.unset("pro_edit_hidden");self.unset("pro_edit_text_scale");self.set("pro_custom_enabled","false");}
        });
        let storage=if builtin {"pro_edit_widgets"}else{"pro_custom_widgets"};
        let mut widgets=match custom::parse(&self.get(storage).unwrap_or_default()) {
            Ok(w)=>w, Err(e)=>{ui.colored_label(BAD,e);return;}
        };
        let mut changed=false;
        let mut enabled=self.get("pro_custom_enabled").as_deref()==Some("true");
        if !builtin && ui.checkbox(&mut enabled,"Apply free canvas to the display").changed() {
            self.set("pro_custom_enabled",if enabled{"true"}else{"false"});
            if enabled { self.set("layout","pro");self.set("pro_layout","axis");self.doc.unset("pro_game_layout"); }
        }
        ui.horizontal(|ui| {
            if !builtin && ui.button("Add sensor widget").clicked() && widgets.len()<48 {
                let mut w=Widget::new("cpu_temp");w.x=40.+(widgets.len()%5) as f32*320.;w.y=40.+(widgets.len()/5%2) as f32*180.;w.bound();widgets.push(w);changed=true;
            }
            ui.label("Click a widget to select it. Drag to move; use controls below to resize.");
        });
        let width=ui.available_width(); let scale=width/1920.;
        let (canvas,_) = ui.allocate_exact_size(Vec2::new(width,462.*scale),Sense::hover());
        ui.painter().rect_filled(canvas,Rounding::same(8.),BG);
        if let Some(texture)=&self.preview {ui.painter().image(texture.id(),canvas,Rect::from_min_max(Pos2::ZERO,Pos2::new(1.,1.)),Color32::WHITE);}
        for x in (0..1920).step_by(40) {
            ui.painter().line_segment([canvas.min+Vec2::new(x as f32*scale,0.),canvas.min+Vec2::new(x as f32*scale,462.*scale)],Stroke::new(0.5,LINE));
        }
        for y in (0..462).step_by(40) {
            ui.painter().line_segment([canvas.min+Vec2::new(0.,y as f32*scale),canvas.min+Vec2::new(1920.*scale,y as f32*scale)],Stroke::new(0.5,LINE));
        }
        let mut selected=ui.ctx().data_mut(|d|d.get_temp::<usize>(egui::Id::new("canvas_selected"))).unwrap_or(0).min(widgets.len().saturating_sub(1));
        let snap=ui.ctx().data_mut(|d|d.get_temp::<bool>(egui::Id::new("canvas_snap"))).unwrap_or(true);
        for (i,w) in widgets.iter_mut().enumerate() {
            let r=Rect::from_min_size(canvas.min+Vec2::new(w.x,w.y)*scale,Vec2::new(w.w,w.h)*scale);
            let handle=Rect::from_min_size(r.right_bottom()-Vec2::splat(18.),Vec2::splat(18.));
            let resize=ui.interact(handle,ui.id().with(("resize",i)),Sense::drag());
            if resize.dragged(){selected=i;let delta=ui.input(|input|input.pointer.delta())/scale;w.w+=delta.x;w.h+=delta.y;w.bound();changed=true;}
            ui.painter().rect_filled(handle,Rounding::same(3.),if i==selected{ACCENT}else{LINE});
            let response=ui.interact(r,ui.id().with(("widget",i)),Sense::click_and_drag());
            if response.clicked() || response.drag_started() {selected=i;}
            if response.dragged() && !resize.dragged() {
                let delta=ui.input(|input|input.pointer.delta())/scale;
                w.x+=delta.x;w.y+=delta.y;w.bound();changed=true;
            }
            if response.drag_stopped() && snap {w.x=(w.x/8.).round()*8.;w.y=(w.y/8.).round()*8.;w.bound();changed=true;}
            ui.painter().rect_stroke(r,Rounding::same(4.),Stroke::new(if i==selected {2.}else{1.},if i==selected{ACCENT}else{MUTED}));
        }
        ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("canvas_selected"),selected));
        let mut snap_value=snap;
        if ui.checkbox(&mut snap_value,"Snap to 8 px grid").changed(){ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("canvas_snap"),snap_value));}
        ui.horizontal_wrapped(|ui| {
            for (i,w) in widgets.iter().enumerate(){if ui.selectable_label(i==selected,w.sensor.trim_start_matches("group_").replace('_'," ")).clicked(){selected=i;}}
        });
        ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("canvas_selected"),selected));
        let mut delete=false;let mut duplicate=false;
        if let Some(w)=widgets.get_mut(selected) {
            ui.horizontal_wrapped(|ui| {
                if !builtin { ui.label("Show this reading");
                egui::ComboBox::from_id_salt("canvas_sensor").selected_text(&w.sensor).show_ui(ui,|ui| {
                    for &key in custom::SENSORS {changed|=ui.selectable_value(&mut w.sensor,key.into(),key.replace('_'," ")).changed();}
                });
                }
                ui.collapsing("Position and size",|ui| { for (label,value) in [("X",&mut w.x),("Y",&mut w.y),("Width",&mut w.w),("Height",&mut w.h), ] {
                    ui.label(label);changed|=ui.add(egui::DragValue::new(value).speed(1.)).changed();
                }});
                if !builtin {ui.label("Text size");changed|=ui.add(egui::Slider::new(&mut w.size,12.0..=120.0)).changed();}
                ui.label("Text color");changed|=ui.color_edit_button_srgb(&mut w.color).changed();
                changed|=ui.checkbox(&mut w.panel,if builtin {"Show panel"}else{"Panel background"}).changed();
                if !builtin {duplicate=ui.button("Duplicate").clicked();delete=ui.button("Delete widget").clicked();}
            });
            if builtin {
                let group=w.sensor.trim_start_matches("group_");
                let names:&[&str]=match group {"cpu"=>&["load","power","clock","fan","uptime","network"],"gpu"=>&["load","power","fan","clock","vram","fps"],"mem"=>&["used","free","ssd","disk","read","write"],_=>&[]};
                let mut hidden=self.get_or("pro_edit_hidden","").split(',').filter(|v|!v.is_empty()).map(str::to_string).collect::<Vec<_>>();
                let mut edited=false;
                ui.horizontal_wrapped(|ui|{ui.label("Readings to show:");for &name in names {let key=format!("{group}:{name}");let mut show=!hidden.contains(&key);if ui.checkbox(&mut show,name.to_uppercase()).changed(){hidden.retain(|v|v!=&key);if !show{hidden.push(key);}edited=true;}}});
                if edited {self.set("pro_edit_hidden",&hidden.join(","));}
                let mut size=self.get("pro_edit_text_scale").and_then(|v|v.parse::<f32>().ok()).unwrap_or(1.);
                ui.horizontal(|ui|{ui.label("Reading text size");if ui.add(egui::Slider::new(&mut size,0.85..=1.15).custom_formatter(|n,_|format!("{:.0}%",n*100.))).changed(){self.set("pro_edit_text_scale",&size.to_string());}});
            }
            if builtin && ["group_cpu","group_gpu","group_mem"].contains(&w.sensor.as_str()) {
                let source=custom::theme_template(&self.get_or("pro_edit_layout","axis"));
                if let Some(original)=source.iter().find(|p|p.sensor==w.sensor){w.w=w.w.max(original.w*0.8);w.h=w.h.max(original.h);}
            }
            w.bound();
        }
        if duplicate && widgets.len()<48 {let mut w=widgets[selected].clone();w.x+=24.;w.y+=24.;w.bound();widgets.push(w);changed=true;}
        if delete {widgets.remove(selected);changed=true;}
        if changed {self.set(storage,&custom::encode(&widgets));}
        divider(ui);
        if builtin {
            ui.horizontal(|ui| {
                if ui.button("Use logo instead of music").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Logo image", &["png", "jpg", "jpeg", "bmp"]).pick_file() {
                        if let Some(path) = path.to_str().filter(|p| !p.contains(['#', '\n', '\r'])) {
                            self.set("pro_logo", path);
                        } else {
                            self.save_error = Some("Choose an image path without # or line breaks.".into());
                        }
                    }
                }
                if ui.button("Show music again").clicked() { self.unset("pro_logo"); }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Choose image / GIF").clicked() {
                if let Some(path)=rfd::FileDialog::new().add_filter("Image / GIF", &["png","jpg","jpeg","bmp","gif"]).pick_file() {
                    let result=(|| -> std::io::Result<String> {
                        let dir=self.dir.join("assets/backgrounds");std::fs::create_dir_all(&dir)?;
                        let suffix=path.extension().and_then(|s|s.to_str()).unwrap_or("png");
                        let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
                        let name=format!("background-{stamp}.{suffix}");std::fs::copy(&path,dir.join(&name))?;
                        Ok(format!("assets/backgrounds/{name}"))
                    })();
                    match result {Ok(path)=>{self.set("background",&path);self.set("background_effect","none");},Err(e)=>self.save_error=Some(e.to_string())}
                }
            }
            if ui.button("Remove background").clicked(){self.set("background","");}
            let mut dim=self.get("background_dim").and_then(|v|v.parse::<u32>().ok()).unwrap_or(30);
            ui.label("Background dim");if ui.add(egui::Slider::new(&mut dim,0..=100).suffix(" %")).changed(){self.set("background_dim",&dim.to_string());}
        });
        ui.label("GIFs animate on the display. The settings preview shows a single sample frame.");
        ui.horizontal(|ui| {
            if ui.button("Export preset").clicked() {
                if let Some(path)=rfd::FileDialog::new().set_file_name("my-aperture.conf").add_filter("Aperture preset", &["conf"]).save_file() {
                    let keys=["pro_edit_layout","pro_edit_widgets","pro_edit_hidden","pro_edit_text_scale","pro_logo","pro_custom_enabled","pro_custom_widgets","background","background_dim","bg_fit"];
                    let text=keys.iter().map(|key|format!("{key} = {}\n",self.get(key).unwrap_or_default())).collect::<String>();
                    if let Err(e)=std::fs::write(path,text){self.save_error=Some(e.to_string());}
                }
            }
            if ui.button("Import preset").clicked() {
                if let Some(path)=rfd::FileDialog::new().add_filter("Aperture preset", &["conf"]).pick_file() {
                    let result=std::fs::read_to_string(&path).map_err(|e|e.to_string()).and_then(|text|{
                        let doc=ConfDoc::parse(&text);custom::parse(&doc.get("pro_custom_widgets").unwrap_or_default())?;custom::parse(&doc.get("pro_edit_widgets").unwrap_or_default())?;if let Some(key)=doc.get("pro_edit_layout").filter(|v|!v.is_empty()){if custom::theme_template(&key).is_empty(){return Err("This preset uses an unsupported theme".into());}}Ok(doc)
                    });
                    match result {Ok(doc)=>{
                        for key in ["pro_edit_layout","pro_edit_widgets","pro_edit_hidden","pro_edit_text_scale","pro_logo","pro_custom_enabled","pro_custom_widgets","background_dim","bg_fit"] {if let Some(v)=doc.get(key){self.set(key,&v);}}
                        // Presets never modify personal sensors, weather, startup or device orientation.
                        self.set("layout","pro");let key=doc.get("pro_edit_layout").filter(|v|!v.is_empty()).unwrap_or("axis".into());self.set("pro_layout",&key);
                    },Err(e)=>self.save_error=Some(e)}
                }
            }
        });
        ui.label("Presets contain canvas settings only. Background files remain in assets/backgrounds; choose a background after importing on another PC.");
    }
}
