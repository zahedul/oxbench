mod coming_soon;
mod epoch;
mod json_viewer;
mod jwt;

use crate::registry::ToolId;
use crate::tool::Tool;
use coming_soon::ComingSoon;
use epoch::EpochConverter;
use json_viewer::JsonViewer;
use jwt::JwtDecoder;

pub fn create_tool(id: ToolId) -> Box<dyn Tool> {
    match id {
        ToolId::JsonViewer => Box::new(JsonViewer::default()),
        ToolId::EpochConverter => Box::new(EpochConverter::default()),
        ToolId::JwtDecoder => Box::new(JwtDecoder::default()),
        _ => Box::new(ComingSoon::new(id)),
    }
}
