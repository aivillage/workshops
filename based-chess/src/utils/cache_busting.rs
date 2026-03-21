use regex::Regex;
use std::path::Path;

// Cache busting utility - injects version numbers into HTML
pub fn inject_cache_busting(html: &str, static_dir: &str) -> String {
    // Pattern to match src/href with optional version query params
    let pattern = Regex::new(r#"(src|href)=["'](/static/[^"']+\.(js|css))(?:\?v=\d+)?"'"#)
        .expect("Invalid regex pattern");
    
    pattern.replace_all(html, |caps: &regex::Captures| {
        let attr = caps.get(1).unwrap().as_str();
        let file_path = caps.get(2).unwrap().as_str();
        let relative_path = file_path.replace("/static/", "");
        let full_path = Path::new(static_dir).join(&relative_path);
        
        let version = if full_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&full_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        duration.as_secs().to_string()
                    } else {
                        "0".to_string()
                    }
                } else {
                    "0".to_string()
                }
            } else {
                "0".to_string()
            }
        } else {
            "0".to_string()
        };
        
        format!(r#"{}="{}?v={}""#, attr, file_path, version)
    }).to_string()
}

