//! Adresses complètes et tirages indexés ; aucun hash ne sert d'identité.
//!
//! Chaque objet a une seule forme d'adresse : `from_fields` fixe, par type, le
//! nombre et le rôle de ses champs. Un propriétaire ou un domaine entre toujours
//! à plat dans l'adresse, par `Owner` ou `DomainKey`, jamais par une chaîne jointe.
use super::model::{string_enum, ErrorCode, HistoryError, Predicate, MAX_ABS_YEAR};
use serde::{Deserialize, Serialize};

pub const MAX_ID_CHARS: usize = 4096;
/// Début base64url de la signature `chr2\0` : sa présence n'importe où dans un
/// champ trahit un ID ou un payload encodé, avec ou sans préfixe de type.
const ENCODED_SIGNATURE: &str = "Y2hyMg";
const OWNER_CELL: &str = "cell";
const OWNER_DOMAIN: &str = "domain";
const TARGET_LABEL: &str = "label";

string_enum!(AddressKind {
    Entity,
    Person,
    Tenure,
    Event,
    Effect,
    Group,
    Household,
    Relation,
    Transfer,
    Plan,
    Domain
});
impl AddressKind {
    const ALL: [Self; 11] = [
        Self::Entity,
        Self::Person,
        Self::Tenure,
        Self::Event,
        Self::Effect,
        Self::Group,
        Self::Household,
        Self::Relation,
        Self::Transfer,
        Self::Plan,
        Self::Domain,
    ];
    fn prefix(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Person => "person",
            Self::Tenure => "tenure",
            Self::Event => "event",
            Self::Effect => "effect",
            Self::Group => "group",
            Self::Household => "household",
            Self::Relation => "relation",
            Self::Transfer => "transfer",
            Self::Plan => "plan",
            Self::Domain => "domain",
        }
    }
    fn from_prefix(prefix: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.prefix() == prefix)
    }
    /// Une relation ou un effet ne désigne jamais une autre relation ou un
    /// autre effet : leurs champs ne s'emboîtent pas.
    fn from_endpoint_prefix(prefix: &str) -> Result<Self, HistoryError> {
        Self::from_prefix(prefix)
            .filter(|k| !matches!(k, Self::Relation | Self::Effect))
            .ok_or_else(|| invalid("type d'extrémité invalide"))
    }
}

/// Bloc décennal de début (§3.2) : division euclidienne, y compris avant l'an
/// zéro. `decade_block(-1) == -10`.
pub fn decade_block(year: i32) -> i32 {
    year.div_euclid(10) * 10
}

/// Domaine démographique : monde, région, profil social et ordinal (§3.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DomainKey {
    world: String,
    region: String,
    society: String,
    ordinal: u32,
}
impl DomainKey {
    pub fn new(
        world: &str,
        region: &str,
        society: &str,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        let key = Self {
            world: world.into(),
            region: region.into(),
            society: society.into(),
            ordinal,
        };
        SemanticAddress::domain(&key)?;
        Ok(key)
    }
    /// Relit un `domain_id` encodé. L'adresse d'un autre type est refusée.
    pub fn from_id(id: &str) -> Result<Self, HistoryError> {
        let address = SemanticAddress::decode(id)?;
        if address.kind != AddressKind::Domain {
            return Err(invalid("adresse de domaine requise"));
        }
        let ordinal = address.fields[3]
            .parse()
            .map_err(|_| invalid("ordinal hors u32"))?;
        Self::new(
            &address.fields[0],
            &address.fields[1],
            &address.fields[2],
            ordinal,
        )
    }
    pub fn world(&self) -> &str {
        &self.world
    }
    pub fn region(&self) -> &str {
        &self.region
    }
    pub fn society(&self) -> &str {
        &self.society
    }
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }
    fn flat_fields(&self) -> Vec<String> {
        vec![
            self.world.clone(),
            self.region.clone(),
            self.society.clone(),
            self.ordinal.to_string(),
        ]
    }
}

/// Propriétaire d'un objet. Son type fait partie de l'adresse : une cellule
/// d'atlas et un domaine restent deux propriétaires distincts, même si le nom
/// de la cellule imitait la forme d'un domaine.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Owner {
    Cell { world: String, cell: String },
    Domain(DomainKey),
}
impl Owner {
    pub fn cell(world: &str, cell: &str) -> Self {
        Self::Cell {
            world: world.into(),
            cell: cell.into(),
        }
    }
    pub fn world(&self) -> &str {
        match self {
            Self::Cell { world, .. } => world,
            Self::Domain(key) => key.world(),
        }
    }
    fn flat_fields(&self) -> Vec<String> {
        match self {
            Self::Cell { world, cell } => vec![world.clone(), OWNER_CELL.into(), cell.clone()],
            Self::Domain(key) => vec![
                key.world.clone(),
                OWNER_DOMAIN.into(),
                key.region.clone(),
                key.society.clone(),
                key.ordinal.to_string(),
            ],
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

/// Rôle d'un champ dans la forme unique d'une adresse.
#[derive(Clone, Copy)]
enum Slot {
    Text,
    Literal(&'static str),
    /// Année de début d'un bloc décennal.
    Block,
    Year,
    Ordinal,
}
use Slot::{Block, Literal, Ordinal, Text, Year};
const DOMAIN_SLOTS: &[Slot] = &[Text, Text, Text, Ordinal];
fn owner_slots(fields: &[String]) -> Result<&'static [Slot], HistoryError> {
    match fields.get(1).map(String::as_str) {
        Some(OWNER_CELL) => Ok(&[Text, Literal(OWNER_CELL), Text]),
        Some(OWNER_DOMAIN) => Ok(&[Text, Literal(OWNER_DOMAIN), Text, Text, Ordinal]),
        _ => Err(invalid("type de propriétaire inconnu")),
    }
}
fn bounded_year(field: &str) -> Result<i64, HistoryError> {
    let n: i64 = field
        .parse()
        .map_err(|_| invalid("entier d'adresse invalide"))?;
    if n.to_string() != field {
        return Err(invalid("entier d'adresse non canonique"));
    }
    if n.abs() > i64::from(MAX_ABS_YEAR) {
        return Err(invalid("année ou bloc hors de ±100 000"));
    }
    Ok(n)
}
fn check_slot(field: &str, slot: Slot) -> Result<(), HistoryError> {
    match slot {
        Text => {}
        Literal(value) if field == value => {}
        Literal(_) => return Err(invalid("champ littéral inattendu")),
        Year => {
            bounded_year(field)?;
        }
        Block => {
            if bounded_year(field)?.rem_euclid(10) != 0 {
                return Err(invalid("origin_block non aligné sur la décennie"));
            }
        }
        Ordinal => {
            let n: u32 = field.parse().map_err(|_| invalid("ordinal hors u32"))?;
            if n.to_string() != field {
                return Err(invalid("entier d'adresse non canonique"));
            }
        }
    }
    Ok(())
}
fn check_slots(fields: &[String], slots: &[Slot]) -> Result<(), HistoryError> {
    if fields.len() != slots.len() {
        return Err(invalid("nombre de champs d'adresse invalide"));
    }
    fields
        .iter()
        .zip(slots)
        .try_for_each(|(field, slot)| check_slot(field, *slot))
}
/// Forme des types à nombre de champs fixe, propriétaire compris.
fn fixed_slots(kind: AddressKind, fields: &[String]) -> Result<Vec<Slot>, HistoryError> {
    let (head, tail): (&[Slot], &[Slot]) = match kind {
        AddressKind::Entity => (owner_slots(fields)?, &[Text, Block, Text]),
        AddressKind::Event => (owner_slots(fields)?, &[Text, Block, Ordinal]),
        AddressKind::Group => (
            owner_slots(fields)?,
            &[Literal("group_slot"), Text, Block, Ordinal],
        ),
        AddressKind::Transfer => (
            owner_slots(fields)?,
            &[Literal("transfer"), Text, Text, Year, Ordinal],
        ),
        // L'office d'un mandat est une entité de type `office`, à plat.
        AddressKind::Tenure => (
            owner_slots(fields)?,
            &[Literal("office"), Block, Text, Ordinal],
        ),
        AddressKind::Person => (DOMAIN_SLOTS, &[Text, Text]),
        AddressKind::Household => (DOMAIN_SLOTS, &[Literal("household"), Text, Year]),
        AddressKind::Plan | AddressKind::Domain => (DOMAIN_SLOTS, &[]),
        AddressKind::Relation | AddressKind::Effect => {
            return Err(invalid("forme à segments, sans liste fixe"))
        }
    };
    Ok([head, tail].concat())
}
fn predicate_name(predicate: Predicate) -> Result<String, HistoryError> {
    serde_json::to_value(predicate)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| invalid("prédicat non sérialisable"))
}

impl SemanticAddress {
    /// Seule porte d'entrée, y compris depuis `decode` et serde. Champs ASCII
    /// imprimables sans espaces ; aucun ID ou payload encodé, à aucune position.
    pub fn from_fields(kind: AddressKind, fields: Vec<String>) -> Result<Self, HistoryError> {
        if fields.is_empty()
            || fields
                .iter()
                .any(|s| s.is_empty() || !s.bytes().all(|b| (0x21..=0x7e).contains(&b)))
        {
            return Err(invalid("champs d'adresse non normalisés"));
        }
        if fields.iter().any(|f| f.contains(ENCODED_SIGNATURE)) {
            return Err(invalid(
                "ID parent encodé : utiliser ses champs sémantiques",
            ));
        }
        match kind {
            AddressKind::Relation => validate_relation_fields(&fields)?,
            AddressKind::Effect => validate_effect_fields(&fields)?,
            _ => check_slots(&fields, &fixed_slots(kind, &fields)?)?,
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
    fn owned(
        kind: AddressKind,
        owner: &Owner,
        tail: impl IntoIterator<Item = String>,
    ) -> Result<Self, HistoryError> {
        let mut fields = owner.flat_fields();
        fields.extend(tail);
        Self::from_fields(kind, fields)
    }
    fn in_domain(
        kind: AddressKind,
        domain: &DomainKey,
        tail: impl IntoIterator<Item = String>,
    ) -> Result<Self, HistoryError> {
        let mut fields = domain.flat_fields();
        fields.extend(tail);
        Self::from_fields(kind, fields)
    }
    pub fn entity(owner: &Owner, kind: &str, block: i32, slot: &str) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Entity,
            owner,
            [kind.into(), block.to_string(), slot.into()],
        )
    }
    pub fn person(domain: &DomainKey, algorithm: &str, local: &str) -> Result<Self, HistoryError> {
        Self::in_domain(
            AddressKind::Person,
            domain,
            [algorithm.into(), local.into()],
        )
    }
    /// `office` est l'adresse d'une entité de type `office` : ses champs sont
    /// repris à plat, jamais son ID encodé.
    pub fn tenure(office: &Self, ordinal: u32) -> Result<Self, HistoryError> {
        if office.kind != AddressKind::Entity {
            return Err(invalid("adresse d'entité office requise"));
        }
        let mut fields = office.fields.clone();
        fields.push(ordinal.to_string());
        Self::from_fields(AddressKind::Tenure, fields)
    }
    pub fn event(
        owner: &Owner,
        kind: &str,
        block: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Event,
            owner,
            [kind.into(), block.to_string(), ordinal.to_string()],
        )
    }
    /// Effet d'un contrat d'impact sur une cible, fils de l'événement parent.
    /// Une cible elle-même adressée entre à plat, sans son monde ; une cible
    /// de catalogue lisible entre comme libellé.
    pub fn effect(event: &Self, contract: &str, target: &str) -> Result<Self, HistoryError> {
        if event.kind != AddressKind::Event {
            return Err(invalid("adresse d'événement requise"));
        }
        let mut fields = event.fields.clone();
        fields.push(contract.into());
        match Self::decode(target) {
            Ok(t) if t.fields[0] == event.fields[0] => {
                fields.push(t.kind.prefix().into());
                fields.extend_from_slice(&t.fields[1..]);
            }
            // Un ID d'un autre monde garde sa signature et sera refusé.
            _ => fields.extend([TARGET_LABEL.into(), target.into()]),
        }
        Self::from_fields(AddressKind::Effect, fields)
    }
    pub fn group(
        owner: &Owner,
        archetype: &str,
        block: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Group,
            owner,
            [
                "group_slot".into(),
                archetype.into(),
                block.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn household(domain: &DomainKey, origin: &str, year: i32) -> Result<Self, HistoryError> {
        Self::in_domain(
            AddressKind::Household,
            domain,
            ["household".into(), origin.into(), year.to_string()],
        )
    }
    pub fn transfer(
        owner: &Owner,
        origin: &str,
        destination: &str,
        year: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        Self::owned(
            AddressKind::Transfer,
            owner,
            [
                "transfer".into(),
                origin.into(),
                destination.into(),
                year.to_string(),
                ordinal.to_string(),
            ],
        )
    }
    pub fn plan(domain: &DomainKey) -> Result<Self, HistoryError> {
        Self::in_domain(AddressKind::Plan, domain, [])
    }
    pub fn domain(domain: &DomainKey) -> Result<Self, HistoryError> {
        Self::in_domain(AddressKind::Domain, domain, [])
    }
    /// Le préfixe commun des extrémités est écrit une seule fois. Chaque segment
    /// porte son nombre de champs ; les types des extrémités restent explicites.
    pub fn relation(
        owner: &Owner,
        predicate: Predicate,
        subject: &Self,
        object: &Self,
        start: i32,
        ordinal: u32,
    ) -> Result<Self, HistoryError> {
        if subject.fields[0] != owner.world() || object.fields[0] != owner.world() {
            return Err(invalid("relation entre deux canons"));
        }
        let common = subject
            .fields
            .iter()
            .zip(&object.fields)
            .take_while(|(a, b)| a == b)
            .count();
        let mut fields = owner.flat_fields();
        fields.extend([
            "relation".into(),
            predicate_name(predicate)?,
            subject.kind.prefix().into(),
            object.kind.prefix().into(),
            (common - 1).to_string(),
        ]);
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
        format!("{}.{}", self.kind.prefix(), base64_encode(&bytes))
    }
    pub fn decode(id: &str) -> Result<Self, HistoryError> {
        if id.len() > MAX_ID_CHARS {
            return Err(invalid("ID au-delà de 4 096 caractères"));
        }
        let (prefix, payload) = id
            .split_once('.')
            .ok_or_else(|| invalid("préfixe absent"))?;
        let kind = AddressKind::from_prefix(prefix).ok_or_else(|| invalid("type inconnu"))?;
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
/// Propriétaire, littéral, prédicat fermé, types des extrémités, puis trois
/// segments comptés : préfixe commun, reste du sujet, reste de l'objet.
fn validate_relation_fields(fields: &[String]) -> Result<(), HistoryError> {
    let owner = owner_slots(fields)?;
    let head = owner.len() + 4;
    let header = fields
        .get(..head)
        .ok_or_else(|| invalid("relation tronquée"))?;
    check_slots(&header[..owner.len()], owner)?;
    let [literal, predicate, subject_kind, object_kind] = &header[owner.len()..] else {
        return Err(invalid("relation tronquée"));
    };
    check_slot(literal, Literal("relation"))?;
    serde_json::from_value::<Predicate>(predicate.as_str().into())
        .map_err(|_| invalid("prédicat inconnu"))?;
    let subject_kind = AddressKind::from_endpoint_prefix(subject_kind)?;
    let object_kind = AddressKind::from_endpoint_prefix(object_kind)?;
    let mut pos = head;
    let mut segment = || -> Result<&[String], HistoryError> {
        let count = fields
            .get(pos)
            .ok_or_else(|| invalid("relation tronquée"))?;
        let n: usize = count
            .parse()
            .map_err(|_| invalid("longueur de segment invalide"))?;
        if n.to_string() != *count {
            return Err(invalid("segment non canonique"));
        }
        let end = pos
            .checked_add(1 + n)
            .filter(|end| *end <= fields.len())
            .ok_or_else(|| invalid("segments de relation invalides"))?;
        let values = &fields[pos + 1..end];
        pos = end;
        Ok(values)
    };
    let common = segment()?;
    let subject_rest = segment()?;
    let object_rest = segment()?;
    if pos + 2 != fields.len() {
        return Err(invalid("segments de relation invalides"));
    }
    if !subject_rest.is_empty() && subject_rest.first() == object_rest.first() {
        return Err(invalid("préfixe commun non maximal"));
    }
    let endpoint = |rest: &[String]| [std::slice::from_ref(&fields[0]), common, rest].concat();
    SemanticAddress::from_fields(subject_kind, endpoint(subject_rest))?;
    SemanticAddress::from_fields(object_kind, endpoint(object_rest))?;
    check_slot(&fields[pos], Year)?;
    check_slot(&fields[pos + 1], Ordinal)
}
/// Champs de l'événement parent, contrat, puis cible typée.
fn validate_effect_fields(fields: &[String]) -> Result<(), HistoryError> {
    let event_end = owner_slots(fields)?.len() + 3;
    let event = fields
        .get(..event_end)
        .ok_or_else(|| invalid("effet tronqué"))?;
    SemanticAddress::from_fields(AddressKind::Event, event.to_vec())?;
    let [_contract, target_kind, target @ ..] = &fields[event_end..] else {
        return Err(invalid("effet tronqué"));
    };
    if target_kind == TARGET_LABEL {
        if target.len() != 1 {
            return Err(invalid("cible d'effet invalide"));
        }
        return Ok(());
    }
    let kind = AddressKind::from_endpoint_prefix(target_kind)?;
    let endpoint = [std::slice::from_ref(&fields[0]), target].concat();
    SemanticAddress::from_fields(kind, endpoint).map(|_| ())
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
    fn domain() -> DomainKey {
        DomainKey::new("w", "region", "society", 0).unwrap()
    }
    fn fields(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).into()).collect()
    }
    fn office() -> SemanticAddress {
        SemanticAddress::entity(&Owner::cell("w", "r"), "office", 0, "mayor").unwrap()
    }
    #[test]
    fn binary_address_has_reference_encoding() {
        let a = SemanticAddress::domain(&domain()).unwrap();
        assert_eq!(
            a.encode(),
            "domain.Y2hyMgABAAAAdwYAAAByZWdpb24HAAAAc29jaWV0eQEAAAAw"
        );
        assert_eq!(SemanticAddress::decode(&a.encode()).unwrap(), a);
    }
    #[test]
    fn address_prefix_uses_a_dot_and_refuses_the_former_colon() {
        let id = SemanticAddress::domain(&domain()).unwrap().encode();
        assert!(id.starts_with("domain.Y2hyMg"));
        assert!(SemanticAddress::decode(&id.replacen('.', ":", 1)).is_err());
    }
    #[test]
    fn decimal_address_fields_are_normalized() {
        for value in ["01", "+1", "-0", "1.0", "x"] {
            assert!(SemanticAddress::from_fields(
                AddressKind::Domain,
                fields(&["w", "r", "s", value])
            )
            .is_err());
        }
        assert!(SemanticAddress::decode("domain.Y2hyMgABAAAAdwEAAAByAQAAAHMBAAAAMB").is_err());
    }
    #[test]
    fn every_object_has_a_single_address_form() {
        // Anciennes formes aplaties ou jointes, toutes refusées.
        for (kind, old) in [
            (
                AddressKind::Person,
                vec!["w", "region/society/0", "a", "17"],
            ),
            (AddressKind::Plan, vec!["w", "region/society/0"]),
            (AddressKind::Tenure, vec!["w", "office", "0"]),
            (
                AddressKind::Household,
                vec!["w", "d", "household", "o", "1354"],
            ),
            (AddressKind::Event, vec!["w", "scope", "k", "1350", "0"]),
            (
                AddressKind::Group,
                vec!["w", "s", "group_slot", "a", "1350", "0"],
            ),
            (
                AddressKind::Transfer,
                vec!["w", "s", "transfer", "a", "b", "1354", "0"],
            ),
            (AddressKind::Entity, vec!["w", "s", "k", "1350", "x"]),
        ] {
            assert!(
                SemanticAddress::from_fields(kind, fields(&old)).is_err(),
                "{kind:?}"
            );
        }
    }
    #[test]
    fn domain_owner_is_flat_in_every_owned_address() {
        let d = domain();
        let owner = Owner::Domain(d.clone());
        let flat = ["w", "domain", "region", "society", "0"];
        for address in [
            SemanticAddress::event(&owner, "plague", 1350, 2).unwrap(),
            SemanticAddress::group(&owner, "farm", 1350, 2).unwrap(),
            SemanticAddress::transfer(&owner, "a", "b", 1354, 2).unwrap(),
            SemanticAddress::entity(&owner, "office", 1350, "mayor").unwrap(),
        ] {
            assert_eq!(&address.fields()[..5], flat);
            assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        }
        for address in [
            SemanticAddress::person(&d, "algo", "17").unwrap(),
            SemanticAddress::household(&d, "singleton/17", 1354).unwrap(),
            SemanticAddress::plan(&d).unwrap(),
        ] {
            assert_eq!(&address.fields()[..4], ["w", "region", "society", "0"]);
            assert_eq!(SemanticAddress::decode(&address.encode()).unwrap(), address);
        }
    }
    #[test]
    fn cell_and_domain_owners_never_share_an_address() {
        let cell = Owner::cell("w", "region/society/0");
        let domain = Owner::Domain(domain());
        assert_ne!(
            SemanticAddress::event(&cell, "k", 0, 0).unwrap().encode(),
            SemanticAddress::event(&domain, "k", 0, 0).unwrap().encode()
        );
    }
    #[test]
    fn domain_key_reads_back_only_from_a_domain_id() {
        let d = domain();
        let id = SemanticAddress::domain(&d).unwrap().encode();
        assert_eq!(DomainKey::from_id(&id).unwrap(), d);
        let plan = SemanticAddress::plan(&d).unwrap().encode();
        assert!(DomainKey::from_id(&plan).is_err());
    }
    #[test]
    fn tenure_requires_an_office_entity() {
        let tenure = SemanticAddress::tenure(&office(), 3).unwrap();
        assert_eq!(SemanticAddress::decode(&tenure.encode()).unwrap(), tenure);
        let other = SemanticAddress::entity(&Owner::cell("w", "r"), "guild", 0, "x").unwrap();
        assert!(SemanticAddress::tenure(&other, 0).is_err());
        let person = SemanticAddress::person(&domain(), "a", "1").unwrap();
        assert!(SemanticAddress::tenure(&person, 0).is_err());
    }
    #[test]
    fn literal_fields_are_checked_when_decoding() {
        // Encodage d'adresses forgées, sans passer par les constructeurs.
        let forge = |kind: AddressKind, values: &[&str]| {
            let mut bytes = b"chr2\0".to_vec();
            for v in values {
                bytes.extend_from_slice(&(v.len() as u32).to_le_bytes());
                bytes.extend_from_slice(v.as_bytes());
            }
            format!("{}.{}", kind.prefix(), base64_encode(&bytes))
        };
        let good = forge(
            AddressKind::Group,
            &["w", "cell", "r", "group_slot", "a", "0", "0"],
        );
        assert!(SemanticAddress::decode(&good).is_ok());
        for (kind, forged) in [
            (
                AddressKind::Group,
                vec!["w", "cell", "r", "x", "a", "0", "0"],
            ),
            (
                AddressKind::Household,
                vec!["w", "r", "s", "0", "x", "o", "1354"],
            ),
            (
                AddressKind::Transfer,
                vec!["w", "cell", "r", "x", "a", "b", "1354", "0"],
            ),
        ] {
            assert!(SemanticAddress::decode(&forge(kind, &forged)).is_err());
        }
        let a = SemanticAddress::person(&domain(), "x", "1").unwrap();
        let b = SemanticAddress::person(&domain(), "x", "2").unwrap();
        let owner = Owner::cell("w", "r");
        let relation =
            SemanticAddress::relation(&owner, Predicate::Trusts, &a, &b, 1300, 0).unwrap();
        let mut forged = relation.fields().to_vec();
        forged[3] = "x".into();
        let forged: Vec<_> = forged.iter().map(String::as_str).collect();
        assert!(SemanticAddress::decode(&forge(AddressKind::Relation, &forged)).is_err());
    }
    #[test]
    fn relation_refuses_an_unknown_predicate_when_decoding() {
        let a = SemanticAddress::person(&domain(), "x", "1").unwrap();
        let b = SemanticAddress::person(&domain(), "x", "2").unwrap();
        let owner = Owner::cell("w", "r");
        let r = SemanticAddress::relation(&owner, Predicate::ParentOf, &a, &b, -5, 0).unwrap();
        assert_eq!(SemanticAddress::decode(&r.encode()).unwrap(), r);
        assert_eq!(r.fields()[4], "parent_of");
        let mut fields = r.fields().to_vec();
        fields[4] = "not_a_predicate".into();
        assert!(SemanticAddress::from_fields(AddressKind::Relation, fields).is_err());
    }
    #[test]
    fn relation_segments_are_exact_and_maximal() {
        let owner = Owner::cell("w", "r");
        let a = SemanticAddress::person(&domain(), "x", "1").unwrap();
        let b = SemanticAddress::person(&domain(), "x", "2").unwrap();
        let self_loop = SemanticAddress::relation(&owner, Predicate::Trusts, &a, &a, 0, 0).unwrap();
        assert_eq!(
            SemanticAddress::decode(&self_loop.encode()).unwrap(),
            self_loop
        );
        let other = Owner::cell("v", "r");
        assert!(SemanticAddress::relation(&other, Predicate::Trusts, &a, &b, 0, 0).is_err());
        let relation = SemanticAddress::relation(&owner, Predicate::Trusts, &a, &b, 0, 0).unwrap();
        assert!(SemanticAddress::relation(&owner, Predicate::Trusts, &relation, &b, 0, 0).is_err());
        // Même relation écrite à la main : préfixe maximal accepté, préfixe
        // raccourci refusé pour le même contenu.
        let head = ["w", "cell", "r", "relation", "trusts", "person", "person"];
        let maximal = [
            &head[..],
            &[
                "4", "region", "society", "0", "x", "1", "1", "1", "2", "0", "0",
            ],
        ]
        .concat();
        assert_eq!(
            SemanticAddress::from_fields(AddressKind::Relation, fields(&maximal)).unwrap(),
            relation
        );
        let unshortened = [
            &head[..],
            &[
                "3", "region", "society", "0", "2", "x", "1", "2", "x", "2", "0", "0",
            ],
        ]
        .concat();
        assert!(SemanticAddress::from_fields(AddressKind::Relation, fields(&unshortened)).is_err());
    }
    #[test]
    fn effect_address_is_a_flat_child_of_its_event() {
        let owner = Owner::cell("w", "r");
        let event = SemanticAddress::event(&owner, "war", 1350, 0).unwrap();
        let site = SemanticAddress::effect(&event, "impact/0", "site/a").unwrap();
        assert_eq!(&site.fields()[..event.fields().len()], event.fields());
        assert_eq!(SemanticAddress::decode(&site.encode()).unwrap(), site);
        assert_ne!(
            site,
            SemanticAddress::effect(&event, "impact/0", "site/b").unwrap()
        );
        let group = SemanticAddress::group(&owner, "farm", 1350, 1).unwrap();
        let flat = SemanticAddress::effect(&event, "impact/0", &group.encode()).unwrap();
        assert!(!flat.fields().contains(&group.encode()));
        assert_eq!(SemanticAddress::decode(&flat.encode()).unwrap(), flat);
        let foreign = SemanticAddress::group(&Owner::cell("v", "r"), "farm", 1350, 1).unwrap();
        assert!(SemanticAddress::effect(&event, "impact/0", &foreign.encode()).is_err());
        assert!(SemanticAddress::effect(&group, "impact/0", "site/a").is_err());
    }
    #[test]
    fn decade_block_uses_euclidean_division_before_year_zero() {
        for (year, block) in [
            (0, 0),
            (9, 0),
            (10, 10),
            (1354, 1350),
            (-1, -10),
            (-10, -10),
            (-11, -20),
            (-100_000, -100_000),
            (100_000, 100_000),
        ] {
            assert_eq!(decade_block(year), block, "{year}");
        }
    }
    #[test]
    fn origin_block_must_be_the_aligned_start_year() {
        let owner = Owner::cell("w", "r");
        assert!(SemanticAddress::event(&owner, "k", -10, 0).is_ok());
        for block in [1351, 1, -1, -15] {
            assert!(SemanticAddress::event(&owner, "k", block, 0).is_err());
            assert!(SemanticAddress::group(&owner, "a", block, 0).is_err());
            assert!(SemanticAddress::entity(&owner, "k", block, "s").is_err());
        }
    }
    #[test]
    fn years_and_blocks_are_bounded() {
        let owner = Owner::cell("w", "r");
        assert!(SemanticAddress::event(&owner, "k", 100_000, 0).is_ok());
        assert!(SemanticAddress::event(&owner, "k", -100_000, 0).is_ok());
        assert!(SemanticAddress::event(&owner, "k", 100_010, 0).is_err());
        assert!(SemanticAddress::event(&owner, "k", -100_010, 0).is_err());
        assert!(SemanticAddress::household(&domain(), "o", 100_000).is_ok());
        assert!(SemanticAddress::household(&domain(), "o", 100_001).is_err());
        assert!(SemanticAddress::transfer(&owner, "a", "b", -100_001, 0).is_err());
        let a = SemanticAddress::person(&domain(), "x", "1").unwrap();
        assert!(SemanticAddress::relation(&owner, Predicate::Helps, &a, &a, 100_001, 0).is_err());
    }
    #[test]
    fn domain_seed_is_pinned_by_a_literal_value() {
        // Oracle : port Python indépendant de keys.rs, seed 42.
        assert_eq!(
            random_semantics_v2::domain_seed(42, "d", "a"),
            816947332546134861
        );
    }
    #[test]
    fn draws_are_pinned_by_literal_values() {
        use random_semantics_v2::*;
        assert_eq!(draw(42, &["a"], 0, 0), 11994162603066947378);
        assert_eq!(draw(42, &["a"], 1, 0), 5619045440094019664);
        assert_eq!(draw(42, &["a"], 0, 1), 10113000962104654725);
        assert_eq!(draw(42, &["a", "b"], 0, 0), 11532361124723730494);
        assert_eq!(draw(43, &["a"], 0, 0), 76182163578657930);
        assert_eq!(uniform_below(42, &["a"], 0, 10_000).unwrap(), 7378);
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
    fn encoded_ids_are_refused_in_every_field_position() {
        let parent = office();
        let id = parent.encode();
        let payload = id.split_once('.').unwrap().1;
        let owner = Owner::cell("w", "r");
        for field in [
            id.clone(),
            payload.to_string(),
            format!("x-{id}-y"),
            format!("x-{payload}-y"),
            format!("{id}/suffixe"),
        ] {
            assert!(
                SemanticAddress::event(&owner, &field, 0, 0).is_err(),
                "{field}"
            );
            assert!(SemanticAddress::entity(&owner, "k", 0, &field).is_err());
            assert!(SemanticAddress::person(&domain(), "a", &field).is_err());
            assert!(DomainKey::new("w", &field, "s", 0).is_err());
            assert!(SemanticAddress::event(&Owner::cell("w", &field), "k", 0, 0).is_err());
        }
    }
}
