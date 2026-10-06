pub mod catbox;
pub mod fileio;
pub mod gofile;
pub mod litterbox;
pub mod pixeldrain;
pub mod tempsh;
pub mod tmpfiles;
pub mod transfersh;
pub mod zerox;

use crate::provider::Provider;

pub fn all_providers() -> Vec<Box<dyn Provider>> {
    vec![
        Box::new(catbox::CatboxProvider),
        Box::new(litterbox::LitterboxProvider),
        Box::new(tempsh::TempshProvider),
        Box::new(gofile::GofileProvider),
        Box::new(tmpfiles::TmpfilesProvider),
        Box::new(zerox::ZeroxProvider),
        Box::new(fileio::FileioProvider),
        Box::new(transfersh::TransfershProvider),
        Box::new(pixeldrain::PixeldrainProvider),
    ]
}

pub fn get_provider(name: &str) -> Option<Box<dyn Provider>> {
    match name.to_ascii_lowercase().as_str() {
        "catbox" => Some(Box::new(catbox::CatboxProvider)),
        "litterbox" => Some(Box::new(litterbox::LitterboxProvider)),
        "tempsh" | "temp.sh" => Some(Box::new(tempsh::TempshProvider)),
        "gofile" => Some(Box::new(gofile::GofileProvider)),
        "tmpfiles" | "tmpfiles.org" => Some(Box::new(tmpfiles::TmpfilesProvider)),
        "zerox" | "0x0" | "0x0.st" => Some(Box::new(zerox::ZeroxProvider)),
        "fileio" | "file.io" => Some(Box::new(fileio::FileioProvider)),
        "transfersh" | "transfer.sh" => Some(Box::new(transfersh::TransfershProvider)),
        "pixeldrain" | "pd" => Some(Box::new(pixeldrain::PixeldrainProvider)),
        _ => None,
    }
}
