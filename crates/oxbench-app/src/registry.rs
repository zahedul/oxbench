//! The single list of tools. The sidebar, tabs, home screen and command palette all read from it.
//!
//! To add a tool: add a `ToolId` variant, list it in `ToolId::ALL`, describe it in `ToolId::meta`,
//! and create its UI in `tools::create_tool`. The compiler flags every match that needs the new arm.

use serde::{Deserialize, Serialize};

use crate::style::icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolId {
    JsonViewer,
    EpochConverter,
    JwtDecoder,
    HashGenerator,
    IdGenerator,
    RandomString,
    PasswordGenerator,
    Bcrypt,
    TextCompare,
    CidrCalculator,
    QrGenerator,
    LoremIpsum,
    KeyPair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Data,
    Time,
    Security,
    Generators,
    Text,
    Network,
}

#[derive(Clone, Copy, Debug)]
pub struct ToolMeta {
    pub id: ToolId,
    pub name: &'static str,
    pub icon: &'static str,
    pub category: Category,
    pub summary: &'static str,
    /// Extra lowercase search terms, so "sha" finds the Hash Generator.
    pub keywords: &'static [&'static str],
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Data,
        Category::Time,
        Category::Security,
        Category::Generators,
        Category::Text,
        Category::Network,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Category::Data => "Data & JSON",
            Category::Time => "Time",
            Category::Security => "Security",
            Category::Generators => "Generators",
            Category::Text => "Text",
            Category::Network => "Network",
        }
    }
}

impl ToolId {
    pub const ALL: [ToolId; 13] = [
        ToolId::JsonViewer,
        ToolId::EpochConverter,
        ToolId::JwtDecoder,
        ToolId::HashGenerator,
        ToolId::IdGenerator,
        ToolId::RandomString,
        ToolId::PasswordGenerator,
        ToolId::Bcrypt,
        ToolId::TextCompare,
        ToolId::CidrCalculator,
        ToolId::QrGenerator,
        ToolId::LoremIpsum,
        ToolId::KeyPair,
    ];

    #[allow(
        clippy::too_many_lines,
        reason = "one data entry per tool; the exhaustive match makes the compiler flag a missing tool"
    )]
    pub const fn meta(self) -> ToolMeta {
        let (name, category, summary, keywords): (_, _, _, &'static [&'static str]) = match self {
            ToolId::JsonViewer => (
                "JSON Viewer",
                Category::Data,
                "Format, validate, query and explore JSON and API responses",
                &[
                    "json", "format", "pretty", "minify", "validate", "schema", "jsonpath", "tree",
                    "api", "response", "graphql",
                ],
            ),
            ToolId::EpochConverter => (
                "Epoch Converter",
                Category::Time,
                "Convert Unix timestamps to dates and back",
                &[
                    "epoch",
                    "unix",
                    "timestamp",
                    "time",
                    "date",
                    "utc",
                    "timezone",
                    "milliseconds",
                ],
            ),
            ToolId::JwtDecoder => (
                "JWT Decoder",
                Category::Security,
                "Decode and verify JSON Web Tokens",
                &["jwt", "token", "bearer", "jws", "claims", "verify"],
            ),
            ToolId::HashGenerator => (
                "Hash Generator",
                Category::Security,
                "MD5, SHA-1, SHA-2, SHA-3, BLAKE3 and HMAC",
                &[
                    "hash", "md5", "sha", "sha1", "sha256", "sha512", "sha3", "blake3", "hmac",
                    "checksum", "digest",
                ],
            ),
            ToolId::IdGenerator => (
                "UUID / ULID",
                Category::Generators,
                "Generate and inspect UUIDs and ULIDs",
                &["uuid", "guid", "ulid", "v4", "v7", "id"],
            ),
            ToolId::RandomString => (
                "Random String",
                Category::Generators,
                "Nano IDs, hex and Base64URL tokens, custom alphabets",
                &["random", "nanoid", "token", "string", "hex", "secret"],
            ),
            ToolId::PasswordGenerator => (
                "Password Generator",
                Category::Security,
                "Passwords and passphrases with a strength check",
                &["password", "passphrase", "secure", "strength"],
            ),
            ToolId::Bcrypt => (
                "Bcrypt",
                Category::Security,
                "Hash and verify passwords with bcrypt",
                &["bcrypt", "hash", "password", "verify"],
            ),
            ToolId::TextCompare => (
                "Text Compare",
                Category::Text,
                "Line, word and character diff",
                &["diff", "compare", "difference", "text"],
            ),
            ToolId::CidrCalculator => (
                "IP / CIDR Calculator",
                Category::Network,
                "Subnets, masks, host ranges, split and merge",
                &[
                    "ip", "ipv4", "ipv6", "cidr", "subnet", "netmask", "network", "mask",
                ],
            ),
            ToolId::QrGenerator => (
                "QR Generator",
                Category::Generators,
                "Make QR codes and save them as PNG or SVG",
                &["qr", "qrcode", "barcode"],
            ),
            ToolId::LoremIpsum => (
                "Lorem Ipsum",
                Category::Generators,
                "Placeholder text and fake data",
                &["lorem", "ipsum", "placeholder", "dummy", "fake", "text"],
            ),
            ToolId::KeyPair => (
                "Key Pair Generator",
                Category::Security,
                "RSA, ECDSA and Ed25519 keys as PEM and OpenSSH",
                &[
                    "rsa",
                    "ecdsa",
                    "ed25519",
                    "ssh",
                    "pem",
                    "key",
                    "keypair",
                    "public key",
                    "private key",
                ],
            ),
        };
        ToolMeta {
            id: self,
            name,
            icon: self.icon(),
            category,
            summary,
            keywords,
        }
    }
}

impl ToolId {
    const fn icon(self) -> &'static str {
        match self {
            ToolId::JsonViewer => icons::BRACKETS_CURLY,
            ToolId::EpochConverter => icons::CLOCK,
            ToolId::JwtDecoder => icons::SEAL_CHECK,
            ToolId::HashGenerator => icons::HASH,
            ToolId::IdGenerator => icons::FINGERPRINT,
            ToolId::RandomString => icons::DICE_FIVE,
            ToolId::PasswordGenerator => icons::PASSWORD,
            ToolId::Bcrypt => icons::LOCK_KEY,
            ToolId::TextCompare => icons::GIT_DIFF,
            ToolId::CidrCalculator => icons::NETWORK,
            ToolId::QrGenerator => icons::QR_CODE,
            ToolId::LoremIpsum => icons::PARAGRAPH,
            ToolId::KeyPair => icons::KEY,
        }
    }
}

pub fn list_tools_in(category: Category) -> impl Iterator<Item = ToolMeta> {
    ToolId::ALL
        .into_iter()
        .map(ToolId::meta)
        .filter(move |tool| tool.category == category)
}

/// Every query word must match the tool. Name matches rank above keyword matches;
/// ties keep registry order.
pub fn search_tools(query: &str) -> Vec<ToolMeta> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut scored: Vec<(u32, ToolMeta)> = ToolId::ALL
        .into_iter()
        .map(ToolId::meta)
        .filter_map(|tool| score_tool(&tool, &terms).map(|score| (score, tool)))
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(_, tool)| tool).collect()
}

fn score_tool(tool: &ToolMeta, terms: &[String]) -> Option<u32> {
    let name = tool.name.to_lowercase();
    terms
        .iter()
        .map(|term| score_term(&name, tool.keywords, term))
        .sum()
}

fn score_term(name: &str, keywords: &[&str], term: &str) -> Option<u32> {
    if name.starts_with(term) {
        return Some(3);
    }
    if name.split_whitespace().any(|word| word.starts_with(term)) {
        return Some(2);
    }
    if name.contains(term) || keywords.iter().any(|keyword| keyword.contains(term)) {
        return Some(1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn lists_every_tool_once() {
        let unique: HashSet<ToolId> = ToolId::ALL.into_iter().collect();
        assert_eq!(unique.len(), ToolId::ALL.len());
    }

    #[test]
    fn every_tool_belongs_to_a_listed_category() {
        let listed: usize = Category::ALL
            .into_iter()
            .map(|c| list_tools_in(c).count())
            .sum();
        assert_eq!(listed, ToolId::ALL.len());
    }

    #[test]
    fn keywords_are_lowercase() {
        for tool in ToolId::ALL.map(ToolId::meta) {
            for keyword in tool.keywords {
                assert_eq!(*keyword, keyword.to_lowercase(), "{}", tool.name);
            }
        }
    }

    #[test]
    fn empty_query_returns_all_tools_in_order() {
        let ids: Vec<ToolId> = search_tools("  ").iter().map(|t| t.id).collect();
        assert_eq!(ids, ToolId::ALL);
    }

    #[test]
    fn name_match_ranks_above_keyword_match() {
        let results = search_tools("hash");
        assert_eq!(results[0].id, ToolId::HashGenerator);
        assert!(results.iter().any(|t| t.id == ToolId::Bcrypt));
    }

    #[test]
    fn finds_tools_by_keyword_and_ignores_case() {
        assert_eq!(search_tools("SHA256")[0].id, ToolId::HashGenerator);
        assert_eq!(search_tools("subnet")[0].id, ToolId::CidrCalculator);
        assert_eq!(search_tools("jwt")[0].id, ToolId::JwtDecoder);
    }

    #[test]
    fn requires_every_word_to_match() {
        let ids: Vec<ToolId> = search_tools("key rsa").iter().map(|t| t.id).collect();
        assert_eq!(ids, [ToolId::KeyPair]);
        assert!(search_tools("json zebra").is_empty());
    }
}
