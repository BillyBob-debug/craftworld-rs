//! Read-only probe (D238): per Popit page, the category names with item counts and the
//! first titles, to compare with the game's inventory lists. Numbers and names only.
//! Usage: popit_pages_probe SNAPSHOT GAME-DIRS [PAGE...]
use lbp_catalog::{attachment::AttachedSource, LoadControl};
use std::{collections::BTreeMap, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err("popit_pages_probe SNAPSHOT GAME-DIRS [PAGE...]".into());
    }
    let control = LoadControl::default();
    let catalog = lbp_catalog::load(Path::new(&args[1]), &control)?;
    let source = AttachedSource::attach(Path::new(&args[2]), &catalog, &control)?;
    let popit = lbp_preview::popit::load_popit(&source, &catalog, &control)?;
    if let Ok(pat) = std::env::var("FIND") {
        for it in &popit.items {
            if it.title.to_lowercase().contains(&pat.to_lowercase())
                || it.paths.iter().any(|p| p.contains(&pat))
            {
                println!(
                    "{:?} | {:?} | {:?} | {:?} {}",
                    it.title, it.icon, it.icon_resolved, it.paths, it.sub_type
                );
            }
        }
        return Ok(());
    }
    if let Ok(dir) = std::env::var("DUMP") {
        // DUMP=dir: write the icon of every plan whose path contains $MATCH as <name>_<w>x<h>.rgba.
        let pat = std::env::var("MATCH").unwrap_or_else(|_| "palette_tools_".into());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for it in &popit.items {
            let Some(path) = it.paths.iter().find(|p| p.contains(&pat)) else {
                continue;
            };
            let Some(key) = it.icon.filter(|_| it.icon_resolved) else {
                continue;
            };
            let Ok(icon) = lbp_preview::story::decode_icon(&source, &catalog, key, &control) else {
                continue;
            };
            let stem = path
                .rsplit('/')
                .next()
                .unwrap_or("x")
                .trim_end_matches(".plan");
            let name = format!("{dir}/{stem}_{}x{}.rgba", icon.width, icon.height);
            println!("{stem} {key:?}");
            std::fs::write(name, &icon.rgba).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    for (page, list) in &popit.pages {
        if args.len() > 3 && !args[3..].contains(page) {
            continue;
        }
        println!("page {page}: {} items", list.len());
        let mut cats: BTreeMap<String, (usize, Vec<String>, usize)> = BTreeMap::new();
        for &i in list {
            let it = &popit.items[i];
            let e = cats
                .entry(format!(
                    "{} / {}",
                    it.location.as_deref().unwrap_or("-"),
                    it.category.as_deref().unwrap_or("-")
                ))
                .or_default();
            e.0 += 1;
            e.2 += usize::from(it.icon_resolved);
            if e.1.len() < 3 {
                e.1.push(it.title.clone());
            }
        }
        if std::env::var("LIST").is_ok() {
            for &i in list {
                let it = &popit.items[i];
                println!(
                    "    {:?} | {} | {} | sub {} | {}",
                    it.title,
                    it.location.as_deref().unwrap_or("-"),
                    it.category.as_deref().unwrap_or("-"),
                    it.sub_type,
                    it.paths.first().map_or("?", String::as_str)
                );
            }
        }
        for (c, (n, titles, icons)) in cats {
            println!("  {c}: {n} (icons {icons}) e.g. {titles:?}");
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
