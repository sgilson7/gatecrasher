//! Where soldiers and the mule can walk: a grid of nodes every 2 m over the
//! zone, open where a soldier-sized cylinder fits on the ground, joined to
//! their eight neighbours. A* finds a path; the path is then pulled tight
//! wherever a straight line stays on open nodes.

use crate::fx::{self, int, ONE};
use crate::geom::{v3, V3};
use crate::world::{dm, World};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Node spacing: 2 m. Doors in the layout are centred on even metres and at
/// least 2.4 m wide, so a node or an edge always passes through each.
pub const CELL: i32 = int(2);
/// Nodes per side: −100 m to +100 m.
pub const N: i32 = 101;
const ORIGIN: i32 = -int(100);
const CLEARANCE: i32 = fx::ratio(45, 100);

#[derive(Debug, Clone, Default)]
pub struct Nav {
    open: Vec<bool>,
    /// whether the step east (+x) and south (+z) from each node is clear:
    /// thin bars between two open nodes close the edge, not the nodes
    east: Vec<bool>,
    south: Vec<bool>,
}

fn index(i: i32, j: i32) -> usize {
    (j * N + i) as usize
}

pub fn node_at(i: i32, j: i32) -> V3 {
    v3(ORIGIN + i * CELL, 0, ORIGIN + j * CELL)
}

/// The node nearest a point (clamped to the grid).
pub fn cell_of(p: V3) -> (i32, i32) {
    let f = |x: i32| ((x - ORIGIN + CELL / 2).div_euclid(CELL)).clamp(0, N - 1);
    (f(p.x), f(p.z))
}

impl Nav {
    pub fn build(w: &World) -> Nav {
        let mut open = vec![false; (N * N) as usize];
        let mut east = vec![false; (N * N) as usize];
        let mut south = vec![false; (N * N) as usize];
        for j in 0..N {
            for i in 0..N {
                let p = node_at(i, j);
                open[index(i, j)] = !w.blocked(p, CLEARANCE, dm(1), dm(18));
                east[index(i, j)] = !w.blocked(p.add(v3(CELL / 2, 0, 0)), dm(2), dm(1), dm(18));
                south[index(i, j)] = !w.blocked(p.add(v3(0, 0, CELL / 2)), dm(2), dm(1), dm(18));
            }
        }
        Nav { open, east, south }
    }

    pub fn is_open(&self, i: i32, j: i32) -> bool {
        i >= 0 && j >= 0 && i < N && j < N && self.open[index(i, j)]
    }

    /// The open node nearest a point, searching outward a few rings.
    pub fn nearest_open(&self, p: V3) -> Option<(i32, i32)> {
        let (ci, cj) = cell_of(p);
        let mut best: Option<((i32, i32), i64)> = None;
        for r in 0..6 {
            for j in cj - r..=cj + r {
                for i in ci - r..=ci + r {
                    if (i - ci).abs() != r && (j - cj).abs() != r {
                        continue;
                    }
                    if !self.is_open(i, j) {
                        continue;
                    }
                    let d = node_at(i, j).sub(v3(p.x, 0, p.z));
                    let d2 = d.x as i64 * d.x as i64 + d.z as i64 * d.z as i64;
                    if best.is_none_or(|b| d2 < b.1) {
                        best = Some(((i, j), d2));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|b| b.0)
    }

    fn edge(&self, i: i32, j: i32, di: i32, dj: i32) -> bool {
        match (di, dj) {
            (1, 0) => self.east[index(i, j)],
            (-1, 0) => self.east[index(i - 1, j)],
            (0, 1) => self.south[index(i, j)],
            _ => self.south[index(i, j - 1)],
        }
    }

    fn step_ok(&self, i: i32, j: i32, di: i32, dj: i32) -> bool {
        if !self.is_open(i + di, j + dj) {
            return false;
        }
        if di == 0 || dj == 0 {
            return self.edge(i, j, di, dj);
        }
        self.is_open(i + di, j)
            && self.is_open(i, j + dj)
            && self.edge(i, j, di, 0)
            && self.edge(i + di, j, 0, dj)
            && self.edge(i, j, 0, dj)
            && self.edge(i, j + dj, di, 0)
    }

    /// Whether a straight walk between two points stays on open nodes,
    /// sampled every metre, and meets nothing solid at knee or chest height.
    pub fn straight(&self, w: &World, a: V3, b: V3) -> bool {
        if !self.on_open(a, b) {
            return false;
        }
        let d = v3(b.x - a.x, 0, b.z - a.z);
        let len = d.len();
        if len == 0 {
            return true;
        }
        let dir = d.norm();
        [dm(3), dm(12)].iter().all(|&y| {
            let o = v3(a.x, y, a.z);
            w.ray(o, dir, len, |_| true)
                .is_none_or(|h| h.solid.is_none() || h.t >= len)
        })
    }

    fn on_open(&self, a: V3, b: V3) -> bool {
        let d = v3(b.x - a.x, 0, b.z - a.z);
        let len = d.len();
        let steps = (len / ONE).max(1);
        for k in 0..=steps {
            let p = a.add(v3(d.x * k / steps, 0, d.z * k / steps));
            let (i, j) = cell_of(p);
            if !self.is_open(i, j) {
                return false;
            }
        }
        true
    }

    /// A path from `a` to `b` as waypoints (not including `a`), or `None`
    /// when no open route joins them.
    pub fn path(&self, w: &World, a: V3, b: V3) -> Option<Vec<V3>> {
        let start = self.nearest_open(a)?;
        let goal = self.nearest_open(b)?;
        let total = (N * N) as usize;
        let mut cost = vec![i32::MAX; total];
        let mut from = vec![usize::MAX; total];
        let mut heap = BinaryHeap::new();
        let h = |i: i32, j: i32| {
            let (dx, dz) = ((i - goal.0).abs(), (j - goal.1).abs());
            10 * (dx + dz) - 6 * dx.min(dz)
        };
        cost[index(start.0, start.1)] = 0;
        heap.push(Reverse((h(start.0, start.1), start.0, start.1)));
        let mut found = false;
        let mut expanded = 0;
        while let Some(Reverse((_, i, j))) = heap.pop() {
            if (i, j) == goal {
                found = true;
                break;
            }
            expanded += 1;
            if expanded > 12_000 {
                break;
            }
            let c = cost[index(i, j)];
            for (di, dj) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                if !self.step_ok(i, j, di, dj) {
                    continue;
                }
                let (ni, nj) = (i + di, j + dj);
                let nc = c + if di != 0 && dj != 0 { 14 } else { 10 };
                let k = index(ni, nj);
                if nc < cost[k] {
                    cost[k] = nc;
                    from[k] = index(i, j);
                    heap.push(Reverse((nc + h(ni, nj), ni, nj)));
                }
            }
        }
        if !found {
            return None;
        }
        let mut nodes = Vec::new();
        let mut k = index(goal.0, goal.1);
        let s = index(start.0, start.1);
        while k != s {
            nodes.push(node_at(k as i32 % N, k as i32 / N));
            k = from[k];
        }
        nodes.reverse();
        // the goal itself where it is reachable in a straight line from the last node
        let last = nodes.last().copied().unwrap_or(node_at(start.0, start.1));
        if self.straight(w, last, b) {
            nodes.push(v3(b.x, 0, b.z));
        }
        // pull the string: from each kept point, skip ahead as far as a straight walk goes
        let mut out = Vec::new();
        let mut at = v3(a.x, 0, a.z);
        let mut k = 0;
        while k < nodes.len() {
            let mut far = k;
            for m in (k + 1..nodes.len()).rev().take(24) {
                if self.straight(w, at, nodes[m]) {
                    far = m;
                    break;
                }
            }
            at = nodes[far];
            out.push(at);
            k = far + 1;
        }
        Some(out)
    }
}

/// Steps a walker along waypoints: returns the new position and heading
/// toward the next point, dropping points it reaches.
pub fn follow(pos: V3, path: &mut Vec<V3>, speed: i32, dt: i32) -> (V3, Option<i32>) {
    let mut step = fx::mul(speed, dt);
    let mut p = pos;
    let mut heading = None;
    while step > 0 && !path.is_empty() {
        let target = path[0];
        let d = v3(target.x - p.x, 0, target.z - p.z);
        let len = d.len();
        if len > 0 {
            heading = Some(fx::atan2(-d.x, -d.z));
        }
        if len <= step {
            p = v3(target.x, p.y, target.z);
            step -= len;
            path.remove(0);
        } else {
            let n = d.norm();
            p = p.add(v3(fx::mul(n.x, step), 0, fx::mul(n.z, step)));
            step = 0;
        }
    }
    (p, heading)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Palette;
    use crate::world;

    fn setup() -> (World, Nav) {
        let w = world::build(&Palette::parse(include_str!("../../../data/palette.json")).unwrap());
        let n = Nav::build(&w);
        (w, n)
    }

    #[test]
    fn a_path_goes_round_a_wall_through_a_door_not_through_it() {
        let (w, nav) = setup();
        // from the yard into the barracks (door on its south side at x = −58)
        let a = v3(-int(58), 0, int(0));
        let b = v3(-int(60), 0, -int(26));
        let path = nav.path(&w, a, b).expect("a route exists");
        let mut at = a;
        for p in &path {
            assert!(
                w.clear(at.add(v3(0, ONE, 0)), p.add(v3(0, ONE, 0))),
                "leg {at:?} -> {p:?} goes through a wall"
            );
            at = *p;
        }
        assert!(at.dist(b) < int(2));
    }

    #[test]
    fn walls_and_buildings_are_closed_and_the_yard_is_open() {
        let (_, nav) = setup();
        let (i, j) = cell_of(v3(0, 0, 0));
        assert!(nav.is_open(i, j));
        // the comms tower's shaft
        let (i, j) = cell_of(v3(int(33), 0, -int(57)));
        assert!(!nav.is_open(i, j));
    }

    #[test]
    fn nobody_walks_out_through_the_drain() {
        let (_, nav) = setup();
        let (i, j) = cell_of(v3(-int(80), 0, -dm(290)));
        assert!(!nav.is_open(i, j));
    }

    #[test]
    fn following_a_path_reaches_its_end_at_the_given_speed() {
        let mut path = vec![v3(int(3), 0, 0), v3(int(3), 0, int(4))];
        let mut p = V3::ZERO;
        let mut ticks = 0;
        while !path.is_empty() && ticks < 1000 {
            p = follow(p, &mut path, int(7), fx::ratio(1, 60)).0;
            ticks += 1;
        }
        // 7 m at 7 m/s: one second
        assert!((59..=61).contains(&ticks), "{ticks}");
        assert_eq!(p, v3(int(3), 0, int(4)));
    }
}
