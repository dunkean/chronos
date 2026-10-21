//! Name assignment by stable key. A name is a label, never an identity.

use std::collections::{BTreeMap, BTreeSet};

use crate::keys::key;
use crate::model::NamePool;

pub const CULTURES: [&str; 3] = ["french", "british", "fantasy"];

/// Pool key for a site culture: the site's own culture when it names a pool,
/// the configured culture otherwise.
pub fn resolve_culture<'a>(site_culture: &'a str, default: &'a str) -> &'a str {
    if CULTURES.contains(&site_culture) {
        site_culture
    } else {
        default
    }
}

/// French elision before a capitalised vowel: "Royaume de Ébreuil" →
/// "Royaume d'Ébreuil". Only proper nouns are affected.
pub fn elide(text: &str) -> String {
    const VOWELS: &str = "AEIOUYÀÂÄÉÈÊËÎÏÔÖÛÜŒ";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find("de ") {
        let at_word_start = pos == 0 && out.is_empty() || rest[..pos].ends_with([' ', '(', '«']);
        let next = rest[pos + 3..].chars().next();
        out.push_str(&rest[..pos]);
        if at_word_start && next.is_some_and(|c| VOWELS.contains(c)) {
            out.push_str("d'");
        } else {
            out.push_str("de ");
        }
        rest = &rest[pos + 3..];
    }
    out.push_str(rest);
    out
}

/// Resolved name pools by culture key (fixture lists, or built-in fallbacks).
#[derive(Clone, Debug, Default)]
pub struct Pools {
    pub by_culture: BTreeMap<String, NamePool>,
    pub default: String,
}

impl Pools {
    pub fn get(&self, culture: &str) -> &NamePool {
        let key = resolve_culture(culture, &self.default);
        self.by_culture
            .get(key)
            .or_else(|| self.by_culture.get(&self.default))
            .expect("default culture pool is always resolved")
    }
}

const FR_FORENAMES: &[&str] = &[
    "Agnès",
    "Alix",
    "Amaury",
    "Arnaud",
    "Béatrice",
    "Bertrand",
    "Blanche",
    "Clémence",
    "Enguerrand",
    "Ermengarde",
    "Étienne",
    "Geoffroy",
    "Gilles",
    "Guillaume",
    "Hugues",
    "Isabeau",
    "Jeanne",
    "Mahaut",
    "Marguerite",
    "Perrine",
    "Raoul",
    "Renaud",
    "Thibaut",
    "Yolande",
];
const FR_SURNAMES: &[&str] = &[
    "d'Aubigny",
    "Barbier",
    "Bonnet",
    "de Brienne",
    "Chevalier",
    "de Coucy",
    "Dufour",
    "Fabre",
    "Faucher",
    "Garnier",
    "Lefèvre",
    "Marchand",
    "Mercier",
    "de Montfort",
    "Morel",
    "Pelletier",
    "Roussel",
    "Tessier",
    "Vasseur",
    "de Vergy",
];
const FR_PLACES: &[&str] = &[
    "Aubeterre",
    "Beaulieu",
    "Belcastel",
    "Bellegarde",
    "Boisvert",
    "Castelnau",
    "Châteauroux",
    "Clairvaux",
    "Combelonge",
    "Escalquens",
    "Fontfroide",
    "Hautefort",
    "Lagarde",
    "Malemort",
    "Montcalm",
    "Montfaucon",
    "Montrésor",
    "Noirmoutier",
    "Pierrefonds",
    "Puylaurens",
    "Rochefort",
    "Roquebrune",
    "Saint-Aubin",
    "Sauveterre",
    "Valmont",
    "Villefranche",
    "Villeneuve",
    "Vauclair",
    "Mirebeau",
    "Brassac",
];
const EN_FORENAMES: &[&str] = &[
    "Agnes",
    "Alice",
    "Bartholomew",
    "Cecily",
    "Edmund",
    "Eleanor",
    "Geoffrey",
    "Gilbert",
    "Hawise",
    "Henry",
    "Isabel",
    "Joan",
    "John",
    "Margery",
    "Matilda",
    "Nicholas",
    "Ralph",
    "Richard",
    "Robert",
    "Roger",
    "Sibyl",
    "Thomas",
    "Walter",
    "William",
];
const EN_SURNAMES: &[&str] = &[
    "Ashdown", "Baker", "Bigod", "Carter", "Clare", "Cooper", "de Lacy", "Fletcher", "Fuller",
    "Grey", "Hawkwood", "Marshal", "Mortimer", "Neville", "Percy", "Ridley", "Thatcher", "Tyler",
    "Warren", "Wright",
];
const EN_PLACES: &[&str] = &[
    "Ashby",
    "Barnford",
    "Blackwater",
    "Bramley",
    "Burford",
    "Castleton",
    "Chalkhurst",
    "Eastwick",
    "Elmstead",
    "Fairbourne",
    "Foxley",
    "Greystone",
    "Hartwell",
    "Holloway",
    "Kingsbridge",
    "Langdon",
    "Marlow",
    "Middlewick",
    "Northam",
    "Oakhurst",
    "Redcliffe",
    "Rookwood",
    "Saltmarsh",
    "Stanford",
    "Thornbury",
    "Westbury",
    "Whitby",
    "Wickham",
    "Woodley",
    "Yarmouth",
];
const SYLLABLES: &[&str] = &[
    "ar", "bel", "cor", "dun", "el", "fen", "gal", "hal", "ir", "kar", "lor", "mir", "nor", "or",
    "quel", "ral", "sar", "tal", "ul", "vor", "wen", "yr", "zan", "ith", "eth", "an",
];

/// Built-in synthetic pool used when the fixture provides none.
pub fn builtin_pool(culture: &str, seed: u64) -> NamePool {
    let owned = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    match culture {
        "british" => NamePool {
            forenames: owned(EN_FORENAMES),
            surnames: owned(EN_SURNAMES),
            places: owned(EN_PLACES),
            ..NamePool::default()
        },
        "fantasy" => {
            let make = |kind: &str, count: usize| -> Vec<String> {
                let set: BTreeSet<String> = (0..count)
                    .map(|i| {
                        let parts =
                            2 + (key(seed, &["syl-count", kind, &i.to_string()]) % 2) as usize;
                        let mut s = String::new();
                        for p in 0..parts {
                            let k = key(seed, &["syl", kind, &i.to_string(), &p.to_string()]);
                            s.push_str(SYLLABLES[(k % SYLLABLES.len() as u64) as usize]);
                        }
                        capitalize(&s)
                    })
                    .collect();
                set.into_iter().collect()
            };
            NamePool {
                forenames: make("fore", 40),
                surnames: make("sur", 40),
                places: make("place", 60),
                ..NamePool::default()
            }
        }
        _ => NamePool {
            forenames: owned(FR_FORENAMES),
            surnames: owned(FR_SURNAMES),
            places: owned(FR_PLACES),
            ..NamePool::default()
        },
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn pick<'a>(list: &'a [String], seed: u64, parts: &[&str]) -> &'a str {
    if list.is_empty() {
        return "?";
    }
    &list[(key(seed, parts) % list.len() as u64) as usize]
}

/// Assigns unique place names in site order. Collisions probe further keys,
/// then fall back to a qualified form.
pub fn place_names(pool: &NamePool, culture: &str, seed: u64, site_ids: &[String]) -> Vec<String> {
    let qualifiers: &[&str] = match culture {
        "british" => &["Upper", "Lower", "Little", "Great", "North", "South"],
        "fantasy" => &["Haut-", "Bas-", "Vieux-", "Nouvel-"],
        _ => &[
            "-le-Haut",
            "-le-Bas",
            "-la-Neuve",
            "-le-Vieux",
            "-sur-Mer",
            "-les-Bois",
        ],
    };
    let mut used = BTreeSet::new();
    let mut out = Vec::with_capacity(site_ids.len());
    for id in site_ids {
        let mut chosen = None;
        for attempt in 0..24 {
            let name = pick(&pool.places, seed, &["place", id, &attempt.to_string()]);
            if !used.contains(name) {
                chosen = Some(name.to_string());
                break;
            }
        }
        let name = chosen.unwrap_or_else(|| {
            let base = pick(&pool.places, seed, &["place", id, "base"]);
            let mut q = 0;
            loop {
                let qual = qualifiers[q % qualifiers.len()];
                let candidate = match (culture, q / qualifiers.len()) {
                    ("british", 0) => format!("{qual} {base}"),
                    ("fantasy", 0) => format!("{qual}{base}"),
                    (_, 0) => format!("{base}{qual}"),
                    (_, round) => format!("{base} {}", round + 1),
                };
                if !used.contains(&candidate) {
                    break candidate;
                }
                q += 1;
            }
        });
        used.insert(name.clone());
        out.push(name);
    }
    out
}
