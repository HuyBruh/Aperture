// The panels of the Aperture layouts on the 1920 x 462 canvas: 12 px outer margin, 12 px
// gaps, equal columns. Shared by the display (`collection.rs`) and the canvas editor.

/// (panel, [x, y, w, h], mode) of a layout; empty for layouts without panels.
pub fn layout_geometry(key: &str) -> Vec<(&'static str, [f32; 4], &'static str)> {
    match key {
        "panorama" => vec![
            ("cpu", [12., 12., 624., 326.], "grid"),
            ("gpu", [648., 12., 624., 326.], "grid"),
            ("mem", [1284., 12., 624., 326.], "grid"),
            ("time", [12., 350., 400., 100.], "compact"),
            ("weather", [424., 350., 640., 100.], "compact"),
            ("music", [1076., 350., 832., 100.], "compact"),
        ],
        "axis" => vec![
            ("time", [12., 12., 400., 102.], "compact"),
            ("weather", [424., 12., 640., 102.], "compact"),
            ("music", [1076., 12., 832., 102.], "compact"),
            ("cpu", [12., 126., 624., 324.], "grid"),
            ("gpu", [648., 126., 624., 324.], "grid"),
            ("mem", [1284., 126., 624., 324.], "grid"),
        ],
        "ledger" => vec![
            ("time", [12., 12., 392., 150.], "large"),
            ("weather", [12., 174., 392., 160.], "compact"),
            ("music", [12., 346., 392., 104.], "compact"),
            ("cpu", [416., 12., 1492., 138.], "row"),
            ("gpu", [416., 162., 1492., 138.], "row"),
            ("mem", [416., 312., 1492., 138.], "row"),
        ],
        "duet" => vec![
            ("cpu", [12., 12., 1184., 213.], "wide"),
            ("gpu", [12., 237., 1184., 213.], "wide"),
            ("mem", [1208., 12., 348., 438.], "column"),
            ("time", [1568., 12., 340., 140.], "large"),
            ("weather", [1568., 164., 340., 170.], "compact"),
            ("music", [1568., 346., 340., 104.], "compact"),
        ],
        "orbit" => vec![
            ("cpu", [12., 12., 380., 438.], "ring"),
            ("gpu", [404., 12., 380., 438.], "ring"),
            ("time", [796., 12., 328., 150.], "large"),
            ("music", [796., 174., 328., 276.], "cover"),
            ("mem", [1136., 12., 380., 438.], "ring"),
            ("weather", [1528., 12., 380., 438.], "large"),
        ],
        "atelier" => vec![
            ("music", [12., 12., 438., 438.], "cover"),
            ("time", [462., 12., 392., 124.], "large"),
            ("weather", [866., 12., 1042., 124.], "compact"),
            ("cpu", [462., 148., 474., 302.], "grid"),
            ("gpu", [948., 148., 474., 302.], "grid"),
            ("mem", [1434., 148., 474., 302.], "grid"),
        ],
        "cinema" => vec![
            ("music", [12., 12., 438., 438.], "cover"),
            ("cpu", [462., 12., 1446., 107.], "row"),
            ("gpu", [462., 131., 1446., 107.], "row"),
            ("mem", [462., 250., 1446., 107.], "row"),
            ("time", [462., 369., 400., 81.], "compact"),
            ("weather", [874., 369., 1034., 81.], "compact"),
        ],
        _ => Vec::new(),
    }
}

/// The panels of a layout as editable canvas widgets.
pub fn theme_template(key: &str) -> Vec<Widget> {
    layout_geometry(key)
        .into_iter()
        .map(|(kind, r, _)| Widget {
            sensor: format!("group_{kind}"),
            x: r[0],
            y: r[1],
            w: r[2],
            h: r[3],
            size: 24.,
            color: [242, 243, 239],
            panel: true,
        })
        .collect()
}
