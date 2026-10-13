use std::fs;
use std::path::Path;
use utoipa::OpenApi;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openapi = let_timer_http::ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&openapi)?;

    let web_dir = Path::new("web");
    if !web_dir.exists() {
        fs::create_dir_all(web_dir)?;
    }

    let out_path = web_dir.join("openapi.json");
    fs::write(&out_path, &json)?;
    println!("Generated OpenAPI specification at {}", out_path.display());
    Ok(())
}
