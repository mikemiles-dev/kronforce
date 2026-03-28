use std::fs;
use std::path::Path;

fn main() {
    let web_dir = Path::new("web");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("dashboard.html");

    // Read HTML template
    let mut html =
        fs::read_to_string(web_dir.join("index.html")).expect("failed to read web/index.html");

    // Process INCLUDE markers — replace <!-- INCLUDE:path --> with file contents
    let include_prefix = "<!-- INCLUDE:";
    let include_suffix = " -->";
    let mut includes_found = true;
    while includes_found {
        includes_found = false;
        if let Some(start) = html.find(include_prefix)
            && let Some(end) = html[start..].find(include_suffix)
        {
            let marker_end = start + end + include_suffix.len();
            let path_start = start + include_prefix.len();
            let path_end = start + end;
            let rel_path = html[path_start..path_end].to_string();
            let full_path = web_dir.join(&rel_path);
            let content = fs::read_to_string(&full_path)
                .unwrap_or_else(|_| panic!("failed to read web/{}", rel_path));
            html = format!("{}{}{}", &html[..start], content, &html[marker_end..]);
            includes_found = true;
        }
    }

    // Read CSS
    let css = fs::read_to_string(web_dir.join("css/style.css"))
        .expect("failed to read web/css/style.css");

    // Read and concatenate JS files (app.js first, then alphabetical)
    let js_dir = web_dir.join("js");
    let mut js_files: Vec<String> = fs::read_dir(&js_dir)
        .expect("failed to read web/js/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "js"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| name != "app.js")
        .collect();
    js_files.sort();
    js_files.insert(0, "app.js".to_string());

    let mut js = String::new();
    for file in &js_files {
        let content = fs::read_to_string(js_dir.join(file))
            .unwrap_or_else(|_| panic!("failed to read web/js/{}", file));
        js.push_str(&content);
        js.push('\n');
    }

    // Inject CSS and JS into HTML
    let result = html
        .replace("<!-- INJECT:CSS -->", &format!("<style>\n{}</style>", css))
        .replace("<!-- INJECT:JS -->", &format!("<script>\n{}</script>", js));

    fs::write(&out_path, result).expect("failed to write dashboard.html to OUT_DIR");

    // Rerun if any web file changes
    println!("cargo::rerun-if-changed=web/index.html");
    println!("cargo::rerun-if-changed=web/css/style.css");
    for file in &js_files {
        println!("cargo::rerun-if-changed=web/js/{}", file);
    }

    // Rerun if any partial file changes
    println!("cargo::rerun-if-changed=web/partials");
    fn register_partials(dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() {
                    register_partials(&path);
                } else {
                    println!("cargo::rerun-if-changed={}", path.display());
                }
            }
        }
    }
    register_partials(Path::new("web/partials"));

    // --- Migrations ---
    let migrations_dir = Path::new("migrations");
    let migrations_out = Path::new(&out_dir).join("migrations.rs");

    let mut migration_files: Vec<String> = fs::read_dir(migrations_dir)
        .expect("failed to read migrations/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    migration_files.sort();

    let mut migrations_code = String::from("const MIGRATIONS: &[(i64, &str, &str)] = &[\n");
    for file in &migration_files {
        let path = migrations_dir.join(file);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("failed to read migrations/{}", file));

        let mut version: Option<i64> = None;
        let mut description = String::new();
        for line in content.lines() {
            if let Some(v) = line.strip_prefix("-- version:") {
                version = Some(
                    v.trim()
                        .parse()
                        .expect("invalid version number in migration"),
                );
            }
            if let Some(d) = line.strip_prefix("-- description:") {
                description = d.trim().to_string();
            }
            if version.is_some() && !description.is_empty() {
                break;
            }
        }
        let ver =
            version.unwrap_or_else(|| panic!("migration {} missing -- version: header", file));
        if description.is_empty() {
            panic!("migration {} missing -- description: header", file);
        }

        migrations_code.push_str(&format!(
            "    ({}, \"{}\", include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/migrations/{}\"))),\n",
            ver, description, file
        ));

        println!("cargo::rerun-if-changed=migrations/{}", file);
    }
    migrations_code.push_str("];\n");

    fs::write(&migrations_out, migrations_code).expect("failed to write migrations.rs");
    println!("cargo::rerun-if-changed=migrations");
}
