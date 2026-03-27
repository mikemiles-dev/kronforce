use std::fs;
use std::path::Path;

fn main() {
    let web_dir = Path::new("web");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("dashboard.html");

    // Read HTML template
    let html = fs::read_to_string(web_dir.join("index.html"))
        .expect("failed to read web/index.html");

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
}
