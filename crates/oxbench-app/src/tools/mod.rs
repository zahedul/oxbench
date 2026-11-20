mod coming_soon;

use crate::registry::ToolId;
use crate::tool::Tool;
use coming_soon::ComingSoon;

pub fn create_tool(id: ToolId) -> Box<dyn Tool> {
    Box::new(ComingSoon::new(id))
}
