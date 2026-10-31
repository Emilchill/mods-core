use std::{borrow::Cow, error::Error, path::Path};

use wasm_encoder::{ComponentNameSection, ComponentSection, CustomSection};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: cinnabar-mod-template-pack CORE.wasm COMPONENT.wasm".into());
    }

    let core = std::fs::read(&args[0])?;
    let mut component = wit_component::ComponentEncoder::default()
        .module(&core)?
        .validate(true)
        .encode()?;

    let mut name = ComponentNameSection::new();
    name.component("Mods Core");
    name.append_to_component(&mut component);
    append_section(
        &mut component,
        "description",
        b"One list of your mods: press F10, then click a mod to open its settings.",
    );
    append_section(&mut component, "version", env!("CARGO_PKG_VERSION").as_bytes());
    append_section(&mut component, "licenses", b"MIT");
    append_section(
        &mut component,
        "cinnabar:grants",
        include_bytes!("../grants.json"),
    );

    append_section(&mut component, mod_api::ICON_SECTION, include_bytes!("../../../../assets/icon.png"));

    // An optional title font (TTF/OTF), for a personal build: MODSCORE_FONT=path. Do not
    // publish a build with a font you may not share.
    if let Some(path) = std::env::var_os("MODSCORE_FONT") {
        let font = std::fs::read(&path)?;
        append_section(&mut component, mod_api::PANEL_FONT_SECTION, &font);
        println!("Embedded the title font ({} bytes)", font.len());
    }

    // The menu, so the mod never sends its document at run time; it only patches values.
    let panel: serde_json::Value = serde_json::from_str(&modscore::menu_panel())?;
    let mut surfaces = serde_json::Map::new();
    surfaces.insert(modscore::MENU_ROUTE.to_owned(), panel["surface"].clone());
    let bundle = serde_json::json!({"panel": panel, "surfaces": surfaces});
    append_section(&mut component, mod_api::PANEL_BUNDLE_SECTION, &serde_json::to_vec(&bundle)?);

    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(&component)?;
    for payload in wasmparser::Parser::new(0).parse_all(&component) {
        match payload? {
            wasmparser::Payload::ImportSection(imports) => {
                for import in imports.into_imports() {
                    if import?.module.starts_with("wasi") {
                        return Err("Cinnabar mods must not import WASI".into());
                    }
                }
            }
            wasmparser::Payload::ComponentImportSection(imports) => {
                for import in imports {
                    if import?.name.0.starts_with("wasi") {
                        return Err("Cinnabar mods must not import WASI".into());
                    }
                }
            }
            _ => {}
        }
    }

    let output = Path::new(&args[1]);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, &component)?;
    println!("Built Cinnabar component: {} bytes", component.len());
    Ok(())
}

fn append_section(component: &mut Vec<u8>, name: &str, data: &[u8]) {
    CustomSection {
        name: Cow::Borrowed(name),
        data: Cow::Borrowed(data),
    }
    .append_to_component(component);
}
