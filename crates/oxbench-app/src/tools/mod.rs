mod coming_soon;
mod epoch;
mod json_viewer;

use crate::registry::ToolId;
use crate::tool::Tool;
use coming_soon::ComingSoon;
use epoch::EpochConverter;
use json_viewer::JsonViewer;

pub fn create_tool(id: ToolId) -> Box<dyn Tool> {
    match id {
        ToolId::JsonViewer => Box::new(JsonViewer::default()),
        ToolId::EpochConverter => Box::new(EpochConverter::default()),
        _ => Box::new(ComingSoon::new(id)),
    }
}
