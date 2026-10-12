//! The data files a game is made from: the missions (data/missions.json),
//! the uniforms and their match against each ground (data/uniforms.json),
//! and the names Graceland gives its recruits (data/recruits.json).

use crate::fx::{deg, int};
use crate::geom::{v3, V3};
use crate::world::{Surface, World, SURFACES};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Rescue,
    Eliminate,
    Recover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    None,
    Target,
    Elites,
    Place(V3),
}

#[derive(Debug, Clone)]
pub struct Intel {
    pub id: String,
    pub text: String,
    pub reveals: Reveal,
}

#[derive(Debug, Clone)]
pub struct SoldierSpec {
    pub route: Vec<(V3, i32)>,
    pub yaw: i32,
    pub perch: bool,
    pub elite: bool,
    pub officer: bool,
    pub target: bool,
    pub knows: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Mission {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub brief: String,
    pub objective: String,
    pub objective_unknown: String,
    pub target_name: String,
    pub start: V3,
    pub start_yaw: i32,
    pub lz: V3,
    pub target_place: Option<V3>,
    pub seed: u64,
    pub intel: Vec<Intel>,
    pub soldiers: Vec<SoldierSpec>,
}

#[derive(Deserialize)]
struct RawFile {
    missions: Vec<RawMission>,
}

#[derive(Deserialize)]
struct RawMission {
    id: String,
    kind: String,
    name: String,
    brief: String,
    objective: String,
    objective_unknown: String,
    target_name: String,
    start: String,
    start_facing: i32,
    lz: String,
    #[serde(default)]
    target_place: Option<String>,
    seed: u64,
    intel: Vec<RawIntel>,
    soldiers: Vec<RawSoldier>,
}

#[derive(Deserialize)]
struct RawIntel {
    id: String,
    text: String,
    reveals: String,
}

#[derive(Deserialize)]
struct RawSoldier {
    #[serde(default)]
    route: Vec<[i32; 3]>,
    #[serde(default)]
    facing: i32,
    #[serde(default)]
    perch: Option<String>,
    #[serde(default)]
    elite: bool,
    #[serde(default)]
    officer: bool,
    #[serde(default)]
    target: bool,
    #[serde(default)]
    knows: Vec<String>,
}

fn place(world: &World, name: &str, what: &str) -> Result<V3, String> {
    world
        .place_at(name)
        .ok_or_else(|| format!("missions.json: {what} names an unknown place '{name}'"))
}

pub fn parse_missions(json: &str, world: &World) -> Result<Vec<Mission>, String> {
    let raw: RawFile = serde_json::from_str(json).map_err(|e| format!("missions.json: {e}"))?;
    let mut out = Vec::new();
    for m in raw.missions {
        let kind = match m.kind.as_str() {
            "rescue" => Kind::Rescue,
            "eliminate" => Kind::Eliminate,
            "recover" => Kind::Recover,
            k => return Err(format!("missions.json {}: unknown kind '{k}'", m.id)),
        };
        let mut intel = Vec::new();
        for i in &m.intel {
            let reveals = match i.reveals.as_str() {
                "none" => Reveal::None,
                "target" => Reveal::Target,
                "elites" => Reveal::Elites,
                p => Reveal::Place(place(world, p, &i.id)?),
            };
            intel.push(Intel {
                id: i.id.clone(),
                text: i.text.clone(),
                reveals,
            });
        }
        let mut soldiers = Vec::new();
        for (k, s) in m.soldiers.iter().enumerate() {
            let knows =
                s.knows
                    .iter()
                    .map(|id| {
                        intel.iter().position(|i| &i.id == id).ok_or_else(|| {
                            format!("missions.json {} soldier {k}: unknown intel '{id}'", m.id)
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
            let route = match &s.perch {
                Some(p) => vec![(place(world, p, "a perch")?, 0)],
                None => s
                    .route
                    .iter()
                    .map(|r| (v3(int(r[0]), 0, int(r[1])), int(r[2])))
                    .collect(),
            };
            if route.is_empty() {
                return Err(format!(
                    "missions.json {} soldier {k}: no route and no perch",
                    m.id
                ));
            }
            soldiers.push(SoldierSpec {
                route,
                yaw: deg(s.facing),
                perch: s.perch.is_some(),
                elite: s.elite,
                officer: s.officer,
                target: s.target,
                knows,
            });
        }
        if kind == Kind::Eliminate && soldiers.iter().filter(|s| s.target).count() != 1 {
            return Err(format!(
                "missions.json {}: an elimination needs exactly one target",
                m.id
            ));
        }
        let target_place = match &m.target_place {
            Some(p) => Some(place(world, p, "target_place")?),
            None => None,
        };
        if kind != Kind::Eliminate && target_place.is_none() {
            return Err(format!("missions.json {}: needs a target_place", m.id));
        }
        out.push(Mission {
            start: place(world, &m.start, "start")?,
            lz: place(world, &m.lz, "lz")?,
            id: m.id,
            kind,
            name: m.name,
            brief: m.brief,
            objective: m.objective,
            objective_unknown: m.objective_unknown,
            target_name: m.target_name,
            start_yaw: deg(m.start_facing),
            target_place,
            seed: m.seed,
            intel,
            soldiers,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct Uniform {
    pub id: String,
    pub name: String,
    pub base: String,
    pub blotch: String,
    /// percent added to the camo index on each ground, in `Surface` order
    pub matches: [i32; SURFACES],
}

#[derive(Deserialize)]
struct RawUniforms {
    uniforms: Vec<RawUniform>,
}

#[derive(Deserialize)]
struct RawUniform {
    id: String,
    name: String,
    base: String,
    blotch: String,
    grass: i32,
    dirt: i32,
    concrete: i32,
    floor: i32,
    sand: i32,
}

pub fn parse_uniforms(json: &str) -> Result<Vec<Uniform>, String> {
    let raw: RawUniforms = serde_json::from_str(json).map_err(|e| format!("uniforms.json: {e}"))?;
    if raw.uniforms.is_empty() {
        return Err("uniforms.json: no uniforms".into());
    }
    Ok(raw
        .uniforms
        .into_iter()
        .map(|u| {
            let mut matches = [0; SURFACES];
            matches[Surface::Grass as usize] = u.grass;
            matches[Surface::Dirt as usize] = u.dirt;
            matches[Surface::Concrete as usize] = u.concrete;
            matches[Surface::Floor as usize] = u.floor;
            matches[Surface::Sand as usize] = u.sand;
            Uniform {
                id: u.id,
                name: u.name,
                base: u.base,
                blotch: u.blotch,
                matches,
            }
        })
        .collect())
}

#[derive(Debug, Clone, Deserialize)]
pub struct Recruits {
    pub first: Vec<String>,
    pub second: Vec<String>,
    pub units: Vec<String>,
    pub grades: Vec<String>,
    /// what a soldier radios on landing at Graceland; `{}` is his codename
    pub thanks: Vec<String>,
}

pub fn parse_recruits(json: &str) -> Result<Recruits, String> {
    let r: Recruits = serde_json::from_str(json).map_err(|e| format!("recruits.json: {e}"))?;
    if r.first.is_empty()
        || r.second.is_empty()
        || r.units.len() != 3
        || r.grades.is_empty()
        || r.thanks.is_empty()
    {
        return Err("recruits.json: needs names, three units, grades and thanks".into());
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Palette;
    use crate::world;

    pub fn missions() -> Vec<Mission> {
        let w = world::build(&Palette::parse(include_str!("../../../data/palette.json")).unwrap());
        parse_missions(include_str!("../../../data/missions.json"), &w).unwrap()
    }

    #[test]
    fn three_missions_with_different_objectives_targets_and_landing_zones() {
        let m = missions();
        assert_eq!(m.len(), 3);
        let kinds: Vec<Kind> = m.iter().map(|m| m.kind).collect();
        assert_eq!(kinds, vec![Kind::Rescue, Kind::Eliminate, Kind::Recover]);
        assert_ne!(m[0].lz, m[1].lz);
        assert_ne!(m[1].lz, m[2].lz);
        assert_ne!(m[0].start, m[2].start);
        for mission in &m {
            assert!(!mission.intel.is_empty());
            // someone knows where the objective is
            let target_intel = mission
                .intel
                .iter()
                .position(|i| i.reveals == Reveal::Target)
                .unwrap();
            assert!(
                mission.soldiers.iter().any(|s| s.knows.contains(&target_intel)),
                "{}",
                mission.id
            );
        }
    }

    #[test]
    fn the_layouts_differ_between_missions() {
        let m = missions();
        let firsts = |k: usize| m[k].soldiers.iter().map(|s| s.route[0].0).collect::<Vec<_>>();
        assert_ne!(firsts(0), firsts(1));
        assert_ne!(firsts(1), firsts(2));
        let elites = |k: usize| m[k].soldiers.iter().filter(|s| s.elite).count();
        assert!(elites(2) > elites(0), "the lens mission has the most elites");
    }

    #[test]
    fn a_bad_place_name_is_an_error_not_a_panic() {
        let w = World::default();
        let e = parse_missions(include_str!("../../../data/missions.json"), &w).unwrap_err();
        assert!(e.contains("unknown place"), "{e}");
    }

    #[test]
    fn the_data_files_parse() {
        assert!(
            parse_uniforms(include_str!("../../../data/uniforms.json"))
                .unwrap()
                .len()
                >= 4
        );
        parse_recruits(include_str!("../../../data/recruits.json")).unwrap();
    }
}
