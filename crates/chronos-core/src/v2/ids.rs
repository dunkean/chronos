//! Adresses complètes et tirages indexés ; aucun hash ne sert d'identité.
use super::model::{string_enum, ErrorCode, HistoryError};
use serde::{Deserialize, Serialize};

pub const MAX_ID_CHARS: usize = 4096;
string_enum!(AddressKind {
    Entity,
    Person,
    Tenure,
    Event,
    Group,
    Household,
    Relation,
    Transfer,
    Plan,
    Domain
});
impl AddressKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Person => "person",
            Self::Tenure => "tenure",
            Self::Event => "event",
            Self::Group => "group",
            Self::Household => "household",
            Self::Relation => "relation",
            Self::Transfer => "transfer",
            Self::Plan => "plan",
            Self::Domain => "domain",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AddressWire", into = "AddressWire")]
pub struct SemanticAddress {
    kind: AddressKind,
    fields: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AddressWire {
    kind: AddressKind,
    fields: Vec<String>,
}
impl From<SemanticAddress> for AddressWire {
    fn from(a: SemanticAddress) -> Self {
        Self {
            kind: a.kind,
            fields: a.fields,
        }
    }
}
impl TryFrom<AddressWire> for SemanticAddress {
    type Error = HistoryError;
    fn try_from(a: AddressWire) -> Result<Self, Self::Error> {
        Self::from_fields(a.kind, a.fields)
    }
}
fn invalid(message: &str) -> HistoryError {
    HistoryError::new(ErrorCode::InvalidAddress, message)
}
impl SemanticAddress {
    /// Champs déjà normalisés : ASCII imprimable sans espaces ; les positions
    /// numériques sont décimales canoniques. Aucun encodage parent imbriqué.
    pub fn from_fields(kind: AddressKind, fields: Vec<String>) -> Result<Self, HistoryError> {
        let count = match kind {
            AddressKind::Entity | AddressKind::Event => 5,
            AddressKind::Person if fields.len() == 6 => 6,
            AddressKind::Person => 4,
            AddressKind::Tenure if fields.len() == 6 => 6,
            AddressKind::Tenure => 3,
            AddressKind::Group => 6,
            AddressKind::Household => 5,
            AddressKind::Transfer => 7,
            AddressKind::Plan if fields.len() == 4 => 4,
            AddressKind::Plan => 2,
            AddressKind::Domain => 4,
            AddressKind::Relation => fields.len(),
        };
        if fields.len() != count
            || fields.is_empty()
            || fields
                .iter()
                .any(|s| s.is_empty() || !s.bytes().all(|b| (0x21..=0x7e).contains(&b)))
        {
            return Err(invalid("champs d'adresse non normalisés"));
        }
        let numeric: &[usize] = match kind {
            AddressKind::Entity => &[3],
            AddressKind::Event => &[3, 4],
            AddressKind::Tenure if fields.len() == 6 => &[3, 5],
            AddressKind::Tenure => &[2],
            AddressKind::Group => &[4, 5],
            AddressKind::Person if fields.len() == 6 => &[3],
            AddressKind::Plan if fields.len() == 4 => &[3],
            AddressKind::Household => &[4],
            AddressKind::Transfer => &[5, 6],
            AddressKind::Domain => &[3],
            _ => &[],
        };
        for &i in numeric {
            let n: i64 = fields[i]
                .parse()
                .map_err(|_| invalid("entier d'adresse invalide"))?;
            if n.to_string() != fields[i] {
                return Err(invalid("entier d'adresse non canonique"));
            }
        }
        let ordinal = match kind {
            AddressKind::Event => Some(4),
            AddressKind::Tenure if fields.len() == 6 => Some(5),
            AddressKind::Tenure => Some(2),
            AddressKind::Group => Some(5),
            AddressKind::Transfer => Some(6),
            AddressKind::Domain => Some(3),
            AddressKind::Person if fields.len() == 6 => Some(3),
            AddressKind::Plan if fields.len() == 4 => Some(3),
            _ => None,
        };
        if ordinal.is_some_and(|i| fields[i].parse::<u32>().is_err()) {
            return Err(invalid("ordinal hors u32"));
        }
        if fields.iter().any(|field| {
            field.split_once(':').is_some_and(|(prefix, payload)| {
                [
                    "entity",
                    "person",
                    "tenure",
                    "event",
                    "group",
                    "household",
                    "relation",
                    "transfer",
                    "plan",
                    "domain",
                ]
                .contains(&prefix)
                    && payload.starts_with("Y2hyMg")
            })
        }) {
            return Err(invalid(
                "ID parent encodé : utiliser ses champs sémantiques",
            ));
        }
        if kind == AddressKind::Relation {
            validate_relation_fields(&fields)?;
        }
        let address = Self { kind, fields };
        if address.encoded_len() > MAX_ID_CHARS {
            return Err(invalid("ID au-delà de 4 096 caractères"));
        }
        Ok(address)
    }
    pub fn kind(&self) -> AddressKind {
        self.kind
    }
    pub fn fields(&self) -> &[String] {
        &self.fields
    }
    pub fn entity(
        world: &str,
        owner: &str,
        kind: &str,
        block: i32,
        slot: &str,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Entity,
            vec![
                world.into(),
                owner.into(),
                kind.into(),
                block.to_string(),
                slot.into(),
            ],
        )
    }
    pub fn person(
        world: &str,
        domain: &str,
        algorithm: &str,
        local: &str,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Person,
            vec![world.into(), domain.into(), algorithm.into(), local.into()],
        )
    }
    /// Domaine déjà adressé : réutiliser ses champs, jamais son ID encodé.
    pub fn person_in_domain(
        domain: &Self,
        algorithm: &str,
        local: &str,
    ) -> Result<Self, HistoryError> {
        if domain.kind != AddressKind::Domain {
            return Err(invalid("adresse de domaine requise"));
        }
        let mut fields = domain.fields.clone();
        fields.extend([algorithm.into(), local.into()]);
        Self::from_fields(AddressKind::Person, fields)
    }
    pub fn plan_for_domain(domain: &Self) -> Result<Self, HistoryError> {
        if domain.kind != AddressKind::Domain {
            return Err(invalid("adresse de domaine requise"));
        }
        Self::from_fields(AddressKind::Plan, domain.fields.clone())
    }
    pub fn tenure_for_office(office: &Self, ordinal: u32) -> Result<Self, HistoryError> {
        if office.kind != AddressKind::Entity {
            return Err(invalid("adresse d'entité office requise"));
        }
        let mut fields = office.fields.clone();
        fields.push(ordinal.to_string());
        Self::from_fields(AddressKind::Tenure, fields)
    }
    pub fn tenure(world: &str, office: &str, ordinal: u32) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Tenure,
            vec![world.into(), office.into(), ordinal.to_string()],
        )
    }
    pub fn event(
        world: &str,
        owner: &str,
        kind: &str,
        block: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Event,
            vec![
                world.into(),
                owner.into(),
                kind.into(),
                block.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn group(
        world: &str,
        owner: &str,
        slot: &str,
        archetype: &str,
        block: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Group,
            vec![
                world.into(),
                owner.into(),
                slot.into(),
                archetype.into(),
                block.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn household(
        world: &str,
        domain: &str,
        origin: &str,
        year: i32,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Household,
            vec![
                world.into(),
                domain.into(),
                "household".into(),
                origin.into(),
                year.to_string(),
            ],
        )
    }
    pub fn transfer(
        world: &str,
        owner: &str,
        origin: &str,
        destination: &str,
        year: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Transfer,
            vec![
                world.into(),
                owner.into(),
                "transfer".into(),
                origin.into(),
                destination.into(),
                year.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn plan(world: &str, domain: &str) -> Result<Self, HistoryError> {
        Self::from_fields(AddressKind::Plan, vec![world.into(), domain.into()])
    }
    pub fn domain(
        world: &str,
        region: &str,
        society: &str,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::from_fields(
            AddressKind::Domain,
            vec![
                world.into(),
                region.into(),
                society.into(),
                ordinal.to_string(),
            ],
        )
    }
    /// Le préfixe commun des extrémités est écrit une seule fois. Chaque segment
    /// porte son nombre de champs ; les types des extrémités restent explicites.
    pub fn relation(
        owner: &str,
        predicate: &str,
        subject: &Self,
        object: &Self,
        start: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        if subject.fields[0] != object.fields[0] {
            return Err(invalid("relation entre deux canons"));
        }
        let common = subject
            .fields
            .iter()
            .zip(&object.fields)
            .take_while(|(a, b)| a == b)
            .count();
        let mut fields = vec![
            subject.fields[0].clone(),
            owner.into(),
            "relation".into(),
            predicate.into(),
            subject.kind.prefix().into(),
            object.kind.prefix().into(),
            (common - 1).to_string(),
        ];
        fields.extend_from_slice(&subject.fields[1..common]);
        fields.push((subject.fields.len() - common).to_string());
        fields.extend_from_slice(&subject.fields[common..]);
        fields.push((object.fields.len() - common).to_string());
        fields.extend_from_slice(&object.fields[common..]);
        fields.extend([start.to_string(), ordinal.to_string()]);
        Self::from_fields(AddressKind::Relation, fields)
    }
    fn encoded_len(&self) -> usize {
        let n = 5 + self.fields.iter().map(|s| 4 + s.len()).sum::<usize>();
        self.kind.prefix().len() + 1 + (n * 8).div_ceil(6)
    }
    pub fn encode(&self) -> String {
        let mut bytes = b"chr2\0".to_vec();
        for field in &self.fields {
            bytes.extend_from_slice(&(field.len() as u32).to_le_bytes());
            bytes.extend_from_slice(field.as_bytes());
        }
        format!("{}:{}", self.kind.prefix(), base64_encode(&bytes))
    }
    pub fn decode(id: &str) -> Result<Self, HistoryError> {
        if id.len() > MAX_ID_CHARS {
            return Err(invalid("ID au-delà de 4 096 caractères"));
        }
        let (prefix, payload) = id
            .split_once(':')
            .ok_or_else(|| invalid("préfixe absent"))?;
        let kind = [
            AddressKind::Entity,
            AddressKind::Person,
            AddressKind::Tenure,
            AddressKind::Event,
            AddressKind::Group,
            AddressKind::Household,
            AddressKind::Relation,
            AddressKind::Transfer,
            AddressKind::Plan,
            AddressKind::Domain,
        ]
        .into_iter()
        .find(|k| k.prefix() == prefix)
        .ok_or_else(|| invalid("type inconnu"))?;
        let bytes = base64_decode(payload)?;
        if !bytes.starts_with(b"chr2\0") {
            return Err(invalid("signature chr2 absente"));
        }
        let mut fields = Vec::new();
        let mut pos = 5;
        while pos < bytes.len() {
            let length = bytes
                .get(pos..pos + 4)
                .ok_or_else(|| invalid("longueur tronquée"))?;
            let length = u32::from_le_bytes(length.try_into().expect("quatre octets")) as usize;
            pos += 4;
            let end = pos
                .checked_add(length)
                .ok_or_else(|| invalid("longueur excessive"))?;
            let field = std::str::from_utf8(
                bytes
                    .get(pos..end)
                    .ok_or_else(|| invalid("champ tronqué"))?,
            )
            .map_err(|_| invalid("UTF-8 invalide"))?;
            fields.push(field.to_owned());
            pos = end;
        }
        let address = Self::from_fields(kind, fields)?;
        if address.encode() != id {
            return Err(invalid("base64url non canonique"));
        }
        Ok(address)
    }
}
fn validate_relation_fields(fields: &[String]) -> Result<(), HistoryError> {
    fn count(fields: &[String], i: usize) -> Result<usize, HistoryError> {
        let s = fields.get(i).ok_or_else(|| invalid("relation tronquée"))?;
        let n: usize = s
            .parse()
            .map_err(|_| invalid("longueur de segment invalide"))?;
        if n.to_string() != *s || n > fields.len() {
            return Err(invalid("segment non canonique"));
        }
        Ok(n)
    }
    if fields.len() < 12 || fields[2] != "relation" {
        return Err(invalid("structure de relation invalide"));
    }
    let common_end = 7 + count(fields, 6)?;
    let subject_end = common_end + 1 + count(fields, common_end)?;
    let pos = subject_end + 1 + count(fields, subject_end)?;
    if pos + 2 != fields.len() {
        return Err(invalid("segments de relation invalides"));
    }
    let kind = |s: &str| -> Result<AddressKind, HistoryError> {
        [
            AddressKind::Entity,
            AddressKind::Person,
            AddressKind::Tenure,
            AddressKind::Event,
            AddressKind::Group,
            AddressKind::Household,
            AddressKind::Transfer,
            AddressKind::Plan,
            AddressKind::Domain,
        ]
        .into_iter()
        .find(|k| k.prefix() == s)
        .ok_or_else(|| invalid("type d'extrémité invalide"))
    };
    let mut subject = vec![fields[0].clone()];
    subject.extend_from_slice(&fields[7..common_end]);
    let mut object = subject.clone();
    subject.extend_from_slice(&fields[common_end + 1..subject_end]);
    object.extend_from_slice(&fields[subject_end + 1..pos]);
    if subject.get(common_end - 6).is_some()
        && subject.get(common_end - 6) == object.get(common_end - 6)
    {
        return Err(invalid("préfixe commun non maximal"));
    }
    SemanticAddress::from_fields(kind(&fields[4])?, subject)?;
    SemanticAddress::from_fields(kind(&fields[5])?, object)?;
    let start: i32 = fields[pos].parse().map_err(|_| invalid("année invalide"))?;
    let ordinal: u32 = fields[pos + 1]
        .parse()
        .map_err(|_| invalid("ordinal invalide"))?;
    if start.to_string() != fields[pos] || ordinal.to_string() != fields[pos + 1] {
        return Err(invalid("entier non canonique"));
    }
    Ok(())
}
const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
fn base64_encode(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for b in bytes {
        buffer = (buffer << 8) | u32::from(*b);
        bits += 8;
        while bits >= 6 {
            bits -= 6;
            result.push(BASE64[((buffer >> bits) & 63) as usize] as char);
        }
    }
    if bits > 0 {
        result.push(BASE64[((buffer << (6 - bits)) & 63) as usize] as char);
    }
    result
}
fn base64_decode(s: &str) -> Result<Vec<u8>, HistoryError> {
    if s.len() % 4 == 1 {
        return Err(invalid("longueur base64url invalide"));
    }
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    for b in s.bytes() {
        let v = BASE64
            .iter()
            .position(|c| *c == b)
            .ok_or_else(|| invalid("alphabet base64url invalide"))?;
        buffer = (buffer << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    if bits > 0 && buffer & ((1 << bits) - 1) != 0 {
        return Err(invalid("bits de bourrage non nuls"));
    }
    Ok(out)
}

/// Point d'entrée unique des décisions aléatoires v2. Sous-compteurs et essais
/// de rejet sont des champs explicites, jamais un état global consommé.
pub mod random_semantics_v2 {
    use super::{ErrorCode, HistoryError};
    pub fn domain_seed(world_seed: u64, domain_id: &str, provider_algorithm: &str) -> u64 {
        crate::keys::key(
            world_seed,
            &["population/domain-seed/v2", domain_id, provider_algorithm],
        )
    }
    pub fn draw(seed: u64, parts: &[&str], subcounter: u64, rejection: u64) -> u64 {
        let sub = subcounter.to_string();
        let retry = rejection.to_string();
        // ASCII élimine le séparateur 0xff du mélangeur v1. Le nombre de
        // champs empêche toute confusion avec les sous-compteurs suffixés.
        let count = parts.len().to_string();
        let encoded: Vec<_> = parts.iter().map(|s| base64_part(s.as_bytes())).collect();
        let mut key = vec!["random_semantics_v2", count.as_str()];
        key.extend(encoded.iter().map(String::as_str));
        key.extend([sub.as_str(), retry.as_str()]);
        crate::keys::key(seed, &key)
    }
    fn base64_part(bytes: &[u8]) -> String {
        super::base64_encode(bytes)
    }
    /// Uniforme exact sur [0, upper), y compris les bornes non puissances de 2.
    pub fn uniform_below(
        seed: u64,
        parts: &[&str],
        subcounter: u64,
        upper: u64,
    ) -> Result<u64, HistoryError> {
        if upper == 0 {
            return Err(HistoryError::new(
                ErrorCode::InvalidCanonicalValue,
                "borne uniforme nulle",
            ));
        }
        let threshold = upper.wrapping_neg() % upper;
        for retry in 0..=u64::MAX {
            let value = draw(seed, parts, subcounter, retry);
            if value >= threshold {
                return Ok(value % upper);
            }
        }
        Err(HistoryError::new(
            ErrorCode::BudgetExceeded,
            "compteur de rejet épuisé",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_address_has_reference_encoding() {
        let a = SemanticAddress::tenure("w", "o", 0).unwrap();
        assert_eq!(a.encode(), "tenure:Y2hyMgABAAAAdwEAAABvAQAAADA");
        assert_eq!(SemanticAddress::decode(&a.encode()).unwrap(), a);
    }
    #[test]
    fn decimal_address_fields_are_normalized() {
        assert!(SemanticAddress::from_fields(
            AddressKind::Tenure,
            vec!["w".into(), "o".into(), "01".into()]
        )
        .is_err());
        assert!(SemanticAddress::decode("tenure:Y2hyMgABAAAAdwEAAABvAQAAADB").is_err());
    }
    #[test]
    fn domain_seed_preserves_specified_mixer_call() {
        assert_eq!(
            random_semantics_v2::domain_seed(42, "d", "a"),
            crate::keys::key(42, &["population/domain-seed/v2", "d", "a"])
        );
    }
    #[test]
    fn exact_uniform_rejects_the_biased_prefix_and_uses_explicit_retry() {
        use random_semantics_v2::*;
        let upper = (1u64 << 63) + 1;
        let threshold = upper.wrapping_neg() % upper;
        let seed = (0..100)
            .find(|seed| {
                draw(*seed, &["rejection-reference"], 7, 0) < threshold
                    && draw(*seed, &["rejection-reference"], 7, 1) >= threshold
            })
            .unwrap();
        assert_eq!(
            uniform_below(seed, &["rejection-reference"], 7, upper).unwrap(),
            draw(seed, &["rejection-reference"], 7, 1) % upper
        );
        assert_eq!(
            uniform_below(seed, &["rejection-reference"], 7, 1).unwrap(),
            0
        );
        assert!(uniform_below(seed, &[], 0, 0).is_err());
    }
    #[test]
    fn parent_ids_cannot_be_recursively_embedded() {
        let parent = SemanticAddress::entity("w", "r", "office", 0, "mayor").unwrap();
        assert!(SemanticAddress::tenure("w", &parent.encode(), 0).is_err());
        let tenure = SemanticAddress::tenure_for_office(&parent, 0).unwrap();
        assert_eq!(SemanticAddress::decode(&tenure.encode()).unwrap(), tenure);
        let domain = SemanticAddress::domain("w", "region", "society", 0).unwrap();
        let person = SemanticAddress::person_in_domain(&domain, "algo", "17").unwrap();
        assert_eq!(&person.fields()[..4], domain.fields());
        assert_eq!(SemanticAddress::decode(&person.encode()).unwrap(), person);
        let plan = SemanticAddress::plan_for_domain(&domain).unwrap();
        assert_eq!(SemanticAddress::decode(&plan.encode()).unwrap(), plan);
    }
}
