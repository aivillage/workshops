use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct PieceImages {
    #[serde(flatten)]
    pub images: HashMap<String, String>,
}

pub async fn load_piece_images(images_dir: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mapping = [
        ("K", "white_king.png"),
        ("Q", "white_queen.png"),
        ("R", "white_rook.png"),
        ("B", "white_bishop.png"),
        ("N", "white_knight.png"),
        ("P", "white_pawn.png"),
        ("k", "black_king.png"),
        ("q", "black_queen.png"),
        ("r", "black_rook.png"),
        ("b", "black_bishop.png"),
        ("n", "black_knight.png"),
        ("p", "black_pawn.png"),
    ];

    let mut data_uris = HashMap::new();
    for (key, filename) in mapping.iter() {
        let path = Path::new(images_dir).join(filename);
        if !path.exists() {
            tracing::warn!("Missing piece image at {:?}", path);
            continue;
        }
        
        let bytes = tokio::fs::read(&path).await?;
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        data_uris.insert(key.to_string(), format!("data:image/png;base64,{}", encoded));
    }
    
    Ok(data_uris)
}

