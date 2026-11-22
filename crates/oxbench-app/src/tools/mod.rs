mod bcrypt;
mod cidr;
mod epoch;
mod hash;
mod id_generator;
mod json_viewer;
mod jwt;
mod key_pair;
mod lorem;
mod password;
mod qr;
mod random_string;
mod text_compare;

use crate::registry::ToolId;
use crate::tool::Tool;
use bcrypt::BcryptTool;
use cidr::CidrCalculator;
use epoch::EpochConverter;
use hash::HashGenerator;
use id_generator::IdGenerator;
use json_viewer::JsonViewer;
use jwt::JwtDecoder;
use key_pair::KeyPairGenerator;
use lorem::LoremIpsum;
use password::PasswordGenerator;
use qr::QrGenerator;
use random_string::RandomStringGenerator;
use text_compare::TextCompare;

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
        ToolId::TextCompare => Box::new(TextCompare::default()),
        ToolId::CidrCalculator => Box::new(CidrCalculator::default()),
        ToolId::QrGenerator => Box::new(QrGenerator::default()),
        ToolId::LoremIpsum => Box::new(LoremIpsum::default()),
        ToolId::KeyPair => Box::new(KeyPairGenerator::default()),
    }
}
