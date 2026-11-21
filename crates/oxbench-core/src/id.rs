//! UUID and ULID generation and inspection.

use chrono::{DateTime, Utc};
use rand::Rng;
use ulid::Ulid;
use uuid::{Uuid, Variant};

use crate::{CoreError, CoreResult};

pub const MAX_ID_COUNT: usize = 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IdKind {
    #[default]
    UuidV4,
    UuidV7,
    UuidV1,
    UuidV6,
    UuidV5,
    UuidV3,
    Ulid,
    Nil,
    Max,
}

impl IdKind {
    pub const ALL: [IdKind; 9] = [
        IdKind::UuidV4,
        IdKind::UuidV7,
        IdKind::UuidV1,
        IdKind::UuidV6,
        IdKind::UuidV5,
        IdKind::UuidV3,
        IdKind::Ulid,
        IdKind::Nil,
        IdKind::Max,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            IdKind::UuidV4 => "UUID v4 · random",
            IdKind::UuidV7 => "UUID v7 · time-ordered",
            IdKind::UuidV1 => "UUID v1 · time and node",
            IdKind::UuidV6 => "UUID v6 · reordered time",
            IdKind::UuidV5 => "UUID v5 · name, SHA-1",
            IdKind::UuidV3 => "UUID v3 · name, MD5",
            IdKind::Ulid => "ULID",
            IdKind::Nil => "Nil UUID",
            IdKind::Max => "Max UUID",
        }
    }

    pub const fn is_name_based(self) -> bool {
        matches!(self, IdKind::UuidV3 | IdKind::UuidV5)
    }

    /// Kinds that give the same value every time, so generating more than one is pointless.
    pub const fn is_deterministic(self) -> bool {
        matches!(
            self,
            IdKind::UuidV3 | IdKind::UuidV5 | IdKind::Nil | IdKind::Max
        )
    }

    pub const fn is_ulid(self) -> bool {
        matches!(self, IdKind::Ulid)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Namespace {
    #[default]
    Dns,
    Url,
    Oid,
    X500,
    Custom,
}

impl Namespace {
    pub const ALL: [Namespace; 5] = [
        Namespace::Dns,
        Namespace::Url,
        Namespace::Oid,
        Namespace::X500,
        Namespace::Custom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Namespace::Dns => "DNS",
            Namespace::Url => "URL",
            Namespace::Oid => "OID",
            Namespace::X500 => "X.500",
            Namespace::Custom => "Custom",
        }
    }
}

/// How UUIDs are written. ULIDs only take the letter case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IdFormat {
    pub is_uppercase: bool,
    pub has_hyphens: bool,
    pub has_braces: bool,
}

impl Default for IdFormat {
    fn default() -> Self {
        Self {
            is_uppercase: false,
            has_hyphens: true,
            has_braces: false,
        }
    }
}

/// The namespace and name for v3 and v5 UUIDs.
#[derive(Clone, Copy, Debug, Default)]
pub struct NameInput<'a> {
    pub namespace: Namespace,
    /// Used when `namespace` is `Custom`.
    pub custom_namespace: &'a str,
    pub name: &'a str,
}

/// `count` IDs of `kind`. Deterministic kinds always give exactly one.
pub fn generate_ids(
    kind: IdKind,
    count: usize,
    format: IdFormat,
    name: &NameInput,
) -> CoreResult<Vec<String>> {
    if !(1..=MAX_ID_COUNT).contains(&count) {
        return Err(invalid(&format!(
            "Choose between 1 and {MAX_ID_COUNT} IDs."
        )));
    }
    if kind.is_ulid() {
        return generate_ulids(count, format);
    }
    let count = if kind.is_deterministic() { 1 } else { count };
    let node_id = random_node_id();
    let namespace = resolve_namespace(name)?;
    let ids = (0..count)
        .map(|_| {
            let uuid = match kind {
                IdKind::UuidV4 => Uuid::new_v4(),
                IdKind::UuidV7 => Uuid::now_v7(),
                IdKind::UuidV1 => Uuid::now_v1(&node_id),
                IdKind::UuidV6 => Uuid::now_v6(&node_id),
                IdKind::UuidV5 => Uuid::new_v5(&namespace, name.name.as_bytes()),
                IdKind::UuidV3 => Uuid::new_v3(&namespace, name.name.as_bytes()),
                IdKind::Nil | IdKind::Ulid => Uuid::nil(),
                IdKind::Max => Uuid::max(),
            };
            format_uuid(uuid, format)
        })
        .collect();
    Ok(ids)
}

/// What a pasted UUID or ULID contains. Any 128-bit ID can be written either way, so both
/// forms are given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdDetails {
    pub description: String,
    pub uuid: String,
    pub ulid: String,
    pub time: Option<DateTime<Utc>>,
    /// The node of a v1 or v6 UUID, written like a MAC address.
    pub node: Option<String>,
    pub variant: Option<&'static str>,
}

pub fn inspect_id(input: &str) -> CoreResult<IdDetails> {
    let text = input.trim();
    if text.len() == ulid::ULID_LEN {
        let parsed = Ulid::from_string(text)
            .map_err(|error| invalid(&format!("This is not a valid ULID: {error}.")))?;
        return Ok(describe_ulid(parsed));
    }
    let uuid = Uuid::try_parse(text).map_err(|_| {
        invalid(
            "Paste a UUID (such as 0190a5d2-ac96-774b-bcce-b302099a8057) or a ULID (26 characters).",
        )
    })?;
    Ok(describe_uuid(uuid))
}

fn generate_ulids(count: usize, format: IdFormat) -> CoreResult<Vec<String>> {
    // One generator, so IDs made in the same millisecond still sort in creation order.
    let mut generator = ulid::Generator::new();
    (0..count)
        .map(|_| {
            let id = generator.generate().map_err(|_| {
                invalid("Too many ULIDs in one millisecond; generate fewer at a time.")
            })?;
            let text = id.to_string();
            Ok(if format.is_uppercase {
                text
            } else {
                text.to_lowercase()
            })
        })
        .collect()
}

fn resolve_namespace(name: &NameInput) -> CoreResult<Uuid> {
    match name.namespace {
        Namespace::Dns => Ok(Uuid::NAMESPACE_DNS),
        Namespace::Url => Ok(Uuid::NAMESPACE_URL),
        Namespace::Oid => Ok(Uuid::NAMESPACE_OID),
        Namespace::X500 => Ok(Uuid::NAMESPACE_X500),
        Namespace::Custom => Uuid::try_parse(name.custom_namespace.trim())
            .map_err(|_| invalid("The custom namespace must be a UUID.")),
    }
}

/// A random node with the multicast bit set, as RFC 9562 asks for when no MAC address is used.
fn random_node_id() -> [u8; 6] {
    let mut node = [0u8; 6];
    rand::rng().fill_bytes(&mut node);
    node[0] |= 0x01;
    node
}

fn format_uuid(uuid: Uuid, format: IdFormat) -> String {
    let text = if format.has_hyphens {
        uuid.hyphenated().to_string()
    } else {
        uuid.simple().to_string()
    };
    let text = if format.is_uppercase {
        text.to_uppercase()
    } else {
        text
    };
    if format.has_braces {
        return format!("{{{text}}}");
    }
    text
}

fn describe_ulid(id: Ulid) -> IdDetails {
    let time = i64::try_from(id.timestamp_ms())
        .ok()
        .and_then(DateTime::from_timestamp_millis);
    IdDetails {
        description: "ULID · 48-bit millisecond time and 80 random bits".to_owned(),
        uuid: Uuid::from_bytes(id.to_bytes()).hyphenated().to_string(),
        ulid: id.to_string(),
        time,
        node: None,
        variant: None,
    }
}

fn describe_uuid(uuid: Uuid) -> IdDetails {
    let ulid_text = Ulid::from_bytes(*uuid.as_bytes()).to_string();
    let hyphenated = uuid.hyphenated().to_string();
    if uuid.is_nil() || uuid.is_max() {
        let description = if uuid.is_nil() {
            "Nil UUID · all bits zero"
        } else {
            "Max UUID · all bits one"
        };
        return IdDetails {
            description: description.to_owned(),
            uuid: hyphenated,
            ulid: ulid_text,
            time: None,
            node: None,
            variant: None,
        };
    }
    let variant = uuid.get_variant();
    let description = if variant == Variant::RFC4122 {
        describe_version(uuid.get_version_num())
    } else {
        "UUID from outside RFC 9562; its version bits carry no meaning".to_owned()
    };
    let time = uuid.get_timestamp().and_then(|timestamp| {
        let (seconds, nanos) = timestamp.to_unix();
        DateTime::from_timestamp(i64::try_from(seconds).ok()?, nanos)
    });
    IdDetails {
        description,
        uuid: hyphenated,
        ulid: ulid_text,
        time,
        node: uuid.get_node_id().map(describe_node),
        variant: Some(describe_variant(variant)),
    }
}

fn describe_version(version: usize) -> String {
    let meaning = match version {
        1 => "Gregorian time and node",
        2 => "DCE security",
        3 => "name-based, MD5",
        4 => "random",
        5 => "name-based, SHA-1",
        6 => "reordered Gregorian time",
        7 => "Unix time, sortable",
        8 => "custom layout",
        _ => "unknown version",
    };
    format!("UUID version {version} · {meaning}")
}

fn describe_variant(variant: Variant) -> &'static str {
    match variant {
        Variant::RFC4122 => "RFC 9562",
        Variant::NCS => "NCS (reserved)",
        Variant::Microsoft => "Microsoft (reserved)",
        _ => "Future (reserved)",
    }
}

fn describe_node(node: [u8; 6]) -> String {
    let address = node
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":");
    if node[0] & 0x01 == 0 {
        return address;
    }
    format!("{address} · random, not a MAC address")
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn generate(kind: IdKind, count: usize) -> Vec<String> {
        generate_ids(kind, count, IdFormat::default(), &NameInput::default()).unwrap()
    }

    #[test]
    fn generates_unique_ids_of_the_chosen_version() {
        for (kind, version) in [
            (IdKind::UuidV4, 4),
            (IdKind::UuidV7, 7),
            (IdKind::UuidV1, 1),
            (IdKind::UuidV6, 6),
        ] {
            let ids = generate(kind, 50);
            assert_eq!(ids.len(), 50);
            assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 50);
            for id in &ids {
                assert_eq!(Uuid::parse_str(id).unwrap().get_version_num(), version);
            }
        }
    }

    #[test]
    fn time_ordered_ids_sort_in_creation_order() {
        for kind in [IdKind::UuidV7, IdKind::UuidV6, IdKind::Ulid] {
            let ids = generate(kind, 200);
            let mut sorted = ids.clone();
            sorted.sort();
            assert_eq!(ids, sorted, "{}", kind.label());
        }
    }

    #[test]
    fn name_based_ids_match_known_values() {
        let python = NameInput {
            name: "python.org",
            ..NameInput::default()
        };
        let v5 = generate_ids(IdKind::UuidV5, 10, IdFormat::default(), &python).unwrap();
        assert_eq!(v5, ["886313e1-3b8a-5372-9b90-0c9aee199e5d"]);
        let v3 = generate_ids(IdKind::UuidV3, 1, IdFormat::default(), &python).unwrap();
        assert_eq!(v3, ["6fa459ea-ee8a-3ca4-894e-db77e160355e"]);
        let custom = NameInput {
            namespace: Namespace::Custom,
            custom_namespace: " 6ba7b810-9dad-11d1-80b4-00c04fd430c8 ",
            name: "python.org",
        };
        let same = generate_ids(IdKind::UuidV5, 1, IdFormat::default(), &custom).unwrap();
        assert_eq!(same, v5);
    }

    #[test]
    fn rejects_a_bad_namespace_and_count() {
        let bad = NameInput {
            namespace: Namespace::Custom,
            custom_namespace: "dns",
            name: "x",
        };
        let error = generate_ids(IdKind::UuidV5, 1, IdFormat::default(), &bad).unwrap_err();
        assert!(error.to_string().contains("namespace must be a UUID"));
        assert!(
            generate_ids(
                IdKind::UuidV4,
                0,
                IdFormat::default(),
                &NameInput::default()
            )
            .is_err()
        );
        assert!(
            generate_ids(
                IdKind::UuidV4,
                MAX_ID_COUNT + 1,
                IdFormat::default(),
                &NameInput::default()
            )
            .is_err()
        );
    }

    #[test]
    fn formats_case_hyphens_and_braces() {
        let format = IdFormat {
            is_uppercase: true,
            has_hyphens: false,
            has_braces: true,
        };
        let max = generate_ids(IdKind::Max, 3, format, &NameInput::default()).unwrap();
        assert_eq!(max, ["{FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF}"]);
        assert_eq!(
            generate(IdKind::Nil, 1),
            ["00000000-0000-0000-0000-000000000000"]
        );
        let ulid = &generate(IdKind::Ulid, 1)[0];
        assert_eq!(ulid.len(), 26);
        assert_eq!(ulid, &ulid.to_lowercase());
    }

    #[test]
    fn inspects_a_v7_uuid() {
        let details = inspect_id(" 0190a5d2-ac96-774b-bcce-b302099a8057 ").unwrap();
        assert_eq!(details.description, "UUID version 7 · Unix time, sortable");
        assert_eq!(details.variant, Some("RFC 9562"));
        assert_eq!(
            details.time.map(|time| time.timestamp_millis()),
            Some(0x0190_a5d2_ac96)
        );
        assert_eq!(details.ulid.len(), 26);
        assert_eq!(inspect_id(&details.ulid).unwrap().uuid, details.uuid);
    }

    #[test]
    fn inspects_a_v1_uuid_with_its_node() {
        // From RFC 9562, appendix A.1.
        let details = inspect_id("C232AB00-9414-11EC-B3C8-9F6BDECED846").unwrap();
        assert_eq!(
            details.description,
            "UUID version 1 · Gregorian time and node"
        );
        assert_eq!(
            details.time.map(|time| time.to_rfc3339()),
            Some("2022-02-22T19:22:22+00:00".to_owned())
        );
        assert_eq!(
            details.node.as_deref(),
            Some("9f:6b:de:ce:d8:46 · random, not a MAC address")
        );
    }

    #[test]
    fn inspects_ulids_braces_and_special_values() {
        let ulid = inspect_id("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap();
        assert!(ulid.description.starts_with("ULID"));
        assert_eq!(
            ulid.time.map(|time| time.timestamp_millis()),
            Some(1_469_922_850_259)
        );
        let braced = inspect_id("{886313e1-3b8a-5372-9b90-0c9aee199e5d}").unwrap();
        assert_eq!(braced.description, "UUID version 5 · name-based, SHA-1");
        assert_eq!(braced.time, None);
        let nil = inspect_id("00000000000000000000000000000000").unwrap();
        assert_eq!(nil.description, "Nil UUID · all bits zero");
    }

    #[test]
    fn explains_unreadable_ids() {
        assert!(
            inspect_id("hello")
                .unwrap_err()
                .to_string()
                .starts_with("Paste a UUID")
        );
        assert!(
            inspect_id("01ARZ3NDEKTSV4RRFFQ69G5FA!")
                .unwrap_err()
                .to_string()
                .contains("not a valid ULID")
        );
    }
}
