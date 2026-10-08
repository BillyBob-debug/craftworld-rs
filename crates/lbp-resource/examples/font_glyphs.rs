//! U2 PC check helper: decode every glyph of one FNTb file, write a JSON summary and a
//! greyscale PGM glyph sheet (one per raw channel for uncompressed fonts).
//! `cargo run -q -p lbp-resource --example font_glyphs -- <file.fnt> <local-analysis/new-prefix>`
//! Writes `<prefix>.json` and `<prefix>-c0.pgm` (and `-c1.pgm`); refuses existing files.
//! Sheets contain game glyph pixels: keep them under ignored `local-analysis/`.
use lbp_resource::font::{
    atlas_layout, decode_glyphs, parse_font_face, FontLimits, GlyphDecodeLimits, GlyphPixels,
};
use std::{env, fs, io::Write, process::ExitCode};

fn create_new(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let [input, prefix] = args.as_slice() else {
        eprintln!("usage: font_glyphs <file.fnt> <local-analysis/new-prefix>");
        return ExitCode::from(2);
    };
    let run = || -> Result<String, String> {
        let bytes = fs::read(input).map_err(|e| e.to_string())?;
        let font = parse_font_face(&bytes, FontLimits::default()).map_err(|e| e.to_string())?;
        let (bitmaps, summary) = decode_glyphs(&bytes, &font, GlyphDecodeLimits::default())
            .map_err(|e| e.to_string())?;
        let atlas = atlas_layout(&font.glyphs, 1024, 1).map_err(|e| e.to_string())?;
        let channels = if font.compressed == Some(true) { 1 } else { 2 };
        for channel in 0..channels {
            let (w, h) = (atlas.width as usize, atlas.height as usize);
            let mut sheet = vec![0u8; w * h];
            for (slot, bitmap) in atlas.slots.iter().zip(&bitmaps) {
                let Some(bitmap) = bitmap else { continue };
                let row = usize::from(bitmap.width);
                let value = |i: usize| match &bitmap.pixels {
                    GlyphPixels::Coverage(v) => v[i],
                    GlyphPixels::TwoChannel(v) => v[i][channel],
                };
                for i in 0..row * usize::from(bitmap.height) {
                    let (x, y) = (slot.x as usize + i % row, slot.y as usize + i / row);
                    sheet[y * w + x] = value(i);
                }
            }
            let mut pgm = format!("P5\n{w} {h}\n255\n").into_bytes();
            pgm.extend_from_slice(&sheet);
            create_new(&format!("{prefix}-c{channel}.pgm"), &pgm).map_err(|e| e.to_string())?;
        }
        let json = serde_json::json!({
            "schema_version": 1,
            "scope": "u2_font_glyphs",
            "font": font.summary(),
            "decode": summary,
            "atlas": {"width": atlas.width, "height": atlas.height},
            "sheet_channels": channels,
        });
        let text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
        create_new(&format!("{prefix}.json"), text.as_bytes()).map_err(|e| e.to_string())?;
        Ok(format!(
            "decoded {} empty {} malformed {}",
            summary.decoded, summary.empty, summary.malformed
        ))
    };
    match run() {
        Ok(line) => {
            println!("font_glyphs: {line}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("font_glyphs failed: {error}");
            ExitCode::FAILURE
        }
    }
}
