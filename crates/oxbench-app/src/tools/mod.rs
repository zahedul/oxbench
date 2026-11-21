mod bcrypt;
mod coming_soon;
mod epoch;
mod hash;
mod id_generator;
mod json_viewer;
mod jwt;
mod password;
mod random_string;

use crate::registry::ToolId;
use crate::tool::Tool;
use bcrypt::BcryptTool;
use coming_soon::ComingSoon;
use epoch::EpochConverter;
use hash::HashGenerator;
use id_generator::IdGenerator;
use json_viewer::JsonViewer;
use jwt::JwtDecoder;
use password::PasswordGenerator;
use random_string::RandomStringGenerator;

pub fn create_tool(id: ToolId) -> Box<dyn Tool> {
    match id {
        ToolId::JsonViewer => Box::new(JsonViewer::default()),
        ToolId::EpochConverter => Box::new(EpochConverter::default()),
        ToolId::JwtDecoder => Box::new(JwtDecoder::default()),
        ToolId::HashGenerator => Box::new(HashGenerator::default()),
        ToolId::IdGenerator => Box::new(IdGenerator::default()),
        ToolId::RandomString => Box::new(RandomStringGenerator::default()),
        ToolId::PasswordGenerator => Box::new(PasswordGenerator::default()),
        ToolId::Bcrypt => Box::new(BcryptTool::default()),
        _ => Box::new(ComingSoon::new(id)),
    }
}
