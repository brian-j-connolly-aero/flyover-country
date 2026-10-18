//! Readers for Weninger and Fukasawa's knapsack interdiction files (Math.
//! Programming 210 (2025) 847-879; github.com/nwoeanhinnogaehr/bkpsolver):
//! `.ki` instances, `.ans` optima, and the per-run records of their
//! experiments. They carry the published optima over to BOBILib's copies
//! of the same instances, which list the items in another order and keep
//! the items the leader blocks for free (zero cost) that their files omit.

use crate::kip::Kip;
use crate::kp;
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

/// (profit, follower weight, leader cost) of one item.
pub type Item = (i64, i64, i64);

/// A KIP instance up to item order: both capacities and the sorted items.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    pub cl: i64,
    pub cu: i64,
    pub items: Vec<Item>,
}

impl Signature {
    fn new(cl: i64, cu: i64, mut items: Vec<Item>) -> Self {
        items.sort_unstable();
        Signature { cl, cu, items }
    }
}

#[derive(Clone, Debug)]
pub struct KiFile {
    /// Path relative to the repository root, as the run records name it.
    pub file: String,
    pub cl: i64,
    pub cu: i64,
    /// Items in file order.
    pub items: Vec<Item>,
    /// Optimal follower profit, from the matching `.ans` file.
    pub ans: Option<i64>,
}

impl KiFile {
    pub fn signature(&self) -> Signature {
        Signature::new(self.cl, self.cu, self.items.clone())
    }
}

/// The signature of `k` without its zero-cost items, as their files store it.
pub fn kip_signature(k: &Kip) -> Signature {
    let items = (0..k.n)
        .filter(|&j| k.wu[j] > 0)
        .map(|j| (k.p[j], k.wl[j], k.wu[j]))
        .collect();
    Signature::new(k.cl, k.cu, items)
}

fn ints(line: Option<&str>, what: &str) -> Result<Vec<i64>, String> {
    line.ok_or_else(|| format!("missing {what}"))?
        .split_whitespace()
        .map(|t| t.parse().map_err(|_| format!("bad {what} {t:?}")))
        .collect()
}

/// Read `root/rel`: n, C^L, C^U, then the follower weights, the leader
/// costs and the profits, one line each.
pub fn read_ki(root: &Path, rel: &str) -> Result<KiFile, String> {
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let parse = || -> Result<KiFile, String> {
        let mut lines = text.lines();
        let mut scalar = |what: &str| -> Result<i64, String> {
            match ints(lines.next(), what)?[..] {
                [v] => Ok(v),
                _ => Err(format!("{what} is not one number")),
            }
        };
        let n = scalar("n")? as usize;
        let cl = scalar("C^L")?;
        let cu = scalar("C^U")?;
        let wl = ints(lines.next(), "w^L")?;
        let wu = ints(lines.next(), "w^U")?;
        let p = ints(lines.next(), "p")?;
        if wl.len() != n || wu.len() != n || p.len() != n {
            return Err(format!("expected {n} items"));
        }
        let items = (0..n).map(|j| (p[j], wl[j], wu[j])).collect();
        let ans = rel
            .strip_suffix(".ki")
            .and_then(|stem| std::fs::read_to_string(root.join(format!("{stem}.ans"))).ok())
            .and_then(|s| s.split_whitespace().next()?.parse().ok());
        Ok(KiFile {
            file: rel.to_string(),
            cl,
            cu,
            items,
            ans,
        })
    };
    parse().map_err(|e| format!("{rel}: {e}"))
}

/// Every `.ki` file under `root/instances`, by signature. Also returns the
/// files that would not parse, with the reason.
pub fn index(root: &Path) -> (HashMap<Signature, Vec<KiFile>>, Vec<String>) {
    let mut rels = Vec::new();
    let mut stack = vec![root.join("instances")];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "ki") {
                if let Ok(rel) = p.strip_prefix(root) {
                    rels.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    rels.sort();
    let mut by_sig: HashMap<Signature, Vec<KiFile>> = HashMap::new();
    let mut bad = Vec::new();
    for rel in rels {
        match read_ki(root, &rel) {
            Ok(f) => by_sig.entry(f.signature()).or_default().push(f),
            Err(e) => bad.push(e),
        }
    }
    (by_sig, bad)
}

/// One solver run from `full_results/all_results.json`.
#[derive(Clone, Debug)]
pub struct Run {
    pub file: String,
    pub algorithm: String,
    pub status: String,
    pub profit: Option<i64>,
    /// Leader solution, one '0' or '1' per item of the file, item 1 first.
    pub upper: Option<String>,
}

/// Read the run records, one Python dict literal per line, grouped by file.
/// Returns the runs and the number of lines that would not parse.
pub fn read_runs(path: &Path) -> Result<(HashMap<String, Vec<Run>>, usize), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut runs: HashMap<String, Vec<Run>> = HashMap::new();
    let mut bad = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Some(mut d) = py_dict(line) else {
            bad += 1;
            continue;
        };
        let Some(file) = d.remove("file") else {
            bad += 1;
            continue;
        };
        let run = Run {
            algorithm: d.remove("algorithm").unwrap_or_default(),
            status: d.remove("status").unwrap_or_default(),
            profit: d.get("profit").and_then(|s| s.trim().parse().ok()),
            upper: d.remove("upper"),
            file: file.clone(),
        };
        runs.entry(file).or_default().push(run);
    }
    Ok((runs, bad))
}

/// Parse a flat Python dict literal whose keys are strings and whose values
/// are strings or bare literals (numbers, None), as `repr` writes them.
/// Values come back as text; `None` and malformed input give `None`.
pub fn py_dict(line: &str) -> Option<HashMap<String, String>> {
    let mut it = line.trim().chars().peekable();
    let skip_ws = |it: &mut std::iter::Peekable<std::str::Chars>| {
        while it.peek().is_some_and(|c| c.is_whitespace()) {
            it.next();
        }
    };
    let string = |it: &mut std::iter::Peekable<std::str::Chars>| -> Option<String> {
        let q = it.next().filter(|&c| c == '\'' || c == '"')?;
        let mut s = String::new();
        loop {
            match it.next()? {
                c if c == q => return Some(s),
                '\\' => s.push(match it.next()? {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    c => c,
                }),
                c => s.push(c),
            }
        }
    };
    let mut out = HashMap::new();
    if it.next()? != '{' {
        return None;
    }
    loop {
        skip_ws(&mut it);
        if it.peek() == Some(&'}') {
            it.next();
            break;
        }
        let key = string(&mut it)?;
        skip_ws(&mut it);
        if it.next()? != ':' {
            return None;
        }
        skip_ws(&mut it);
        let value = if matches!(it.peek(), Some('\'' | '"')) {
            Some(string(&mut it)?)
        } else {
            let mut s = String::new();
            while it.peek().is_some_and(|&c| c != ',' && c != '}') {
                s.push(it.next()?);
            }
            let s = s.trim().to_string();
            (s != "None").then_some(s)
        };
        if let Some(v) = value {
            out.insert(key, v);
        }
        skip_ws(&mut it);
        match it.next()? {
            ',' => {}
            '}' => break,
            _ => return None,
        }
    }
    skip_ws(&mut it);
    it.next().is_none().then_some(out)
}

/// The orders in which their run records list a solution's bits: the
/// file's order (item 1 first), or for some runs the solver's internal
/// order, without mapping back (the record's `up_util` is then summed in
/// file order and can exceed C^U). Their solver skipped the mapping back on
/// its early exit until commit 1e413da (April 2023), and the records are
/// older than that fix.
///
/// The internal order is the one their solver used when it wrote the
/// records (until commit bc1d985): `std::sort` of the item indices by
/// decreasing efficiency p / w^L, then decreasing profit. Items with equal
/// efficiency and profit also share w^L but can differ in leader cost, and
/// their order is whatever libstdc++'s unstable sort leaves, so
/// [`libstdcxx_sort`] reproduces that sort step by step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitOrder {
    File,
    Sorted,
}

impl BitOrder {
    /// `order[k]` is the file item that bit k describes.
    pub fn positions(self, ki: &KiFile) -> Vec<usize> {
        let mut order: Vec<usize> = (0..ki.items.len()).collect();
        if self == BitOrder::Sorted {
            let it = &ki.items;
            // Their comparator; exact products, which fit their `int`s on
            // these files.
            libstdcxx_sort(&mut order, |a, b| {
                let ((pa, la, _), (pb, lb, _)) = (it[a], it[b]);
                match kp::eff_cmp(pa, la, pb, lb) {
                    Ordering::Equal => pa > pb,
                    o => o == Ordering::Less,
                }
            });
        }
        order
    }
}

/// Runs this short or shorter are left to the final insertion sort.
const SORT_THRESHOLD: usize = 16;

/// `std::sort` as GCC's libstdc++ implements it: introsort (quicksort with
/// the median of the second, middle and last elements as pivot, heapsort
/// past a depth of 2 floor(log2 n)) down to runs of [`SORT_THRESHOLD`], then
/// one insertion sort. It is not stable, so only an exact replica puts
/// elements that compare equal where their solver put them.
pub fn libstdcxx_sort(v: &mut [usize], less: impl Fn(usize, usize) -> bool) {
    let n = v.len();
    if n < 2 {
        return;
    }
    introsort(v, 0, n, 2 * n.ilog2() as usize, &less);
    if n > SORT_THRESHOLD {
        insertion_sort(v, SORT_THRESHOLD, &less);
        // The first run holds the minimum, so the rest needs no bound check.
        for i in SORT_THRESHOLD..n {
            linear_insert(v, i, &less);
        }
    } else {
        insertion_sort(v, n, &less);
    }
}

fn introsort(
    v: &mut [usize],
    first: usize,
    mut last: usize,
    mut depth: usize,
    less: &impl Fn(usize, usize) -> bool,
) {
    while last - first > SORT_THRESHOLD {
        if depth == 0 {
            heap_sort(&mut v[first..last], less);
            return;
        }
        depth -= 1;
        // Median of three moved to `first`, which then serves as pivot.
        let (a, b, c) = (first + 1, first + (last - first) / 2, last - 1);
        let median = if less(v[a], v[b]) {
            if less(v[b], v[c]) {
                b
            } else if less(v[a], v[c]) {
                c
            } else {
                a
            }
        } else if less(v[a], v[c]) {
            a
        } else if less(v[b], v[c]) {
            c
        } else {
            b
        };
        v.swap(first, median);
        let (mut lo, mut hi) = (first + 1, last);
        let cut = loop {
            while less(v[lo], v[first]) {
                lo += 1;
            }
            hi -= 1;
            while less(v[first], v[hi]) {
                hi -= 1;
            }
            if lo >= hi {
                break lo;
            }
            v.swap(lo, hi);
            lo += 1;
        };
        introsort(v, cut, last, depth, less);
        last = cut;
    }
}

/// `std::make_heap` then `std::sort_heap`.
fn heap_sort(h: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    let len = h.len();
    for parent in (0..len / 2).rev() {
        adjust_heap(h, parent, len, h[parent], less);
    }
    for last in (1..len).rev() {
        let value = h[last];
        h[last] = h[0];
        adjust_heap(h, 0, last, value, less);
    }
}

/// libstdc++'s `__adjust_heap`: sift the hole down to a leaf, then push
/// `value` back up.
fn adjust_heap(
    h: &mut [usize],
    mut hole: usize,
    len: usize,
    value: usize,
    less: &impl Fn(usize, usize) -> bool,
) {
    let top = hole;
    let mut child = hole;
    while child < (len - 1) / 2 {
        child = 2 * (child + 1);
        if less(h[child], h[child - 1]) {
            child -= 1;
        }
        h[hole] = h[child];
        hole = child;
    }
    if len.is_multiple_of(2) && child == (len - 2) / 2 {
        child = 2 * (child + 1);
        h[hole] = h[child - 1];
        hole = child - 1;
    }
    while hole > top {
        let parent = (hole - 1) / 2;
        if !less(h[parent], value) {
            break;
        }
        h[hole] = h[parent];
        hole = parent;
    }
    h[hole] = value;
}

/// Insertion sort of `v[..last]`.
fn insertion_sort(v: &mut [usize], last: usize, less: &impl Fn(usize, usize) -> bool) {
    for i in 1..last {
        if less(v[i], v[0]) {
            v[..=i].rotate_right(1);
        } else {
            linear_insert(v, i, less);
        }
    }
}

/// Move `v[i]` left past every element it is less than.
fn linear_insert(v: &mut [usize], mut i: usize, less: &impl Fn(usize, usize) -> bool) {
    let value = v[i];
    while i > 0 && less(value, v[i - 1]) {
        v[i] = v[i - 1];
        i -= 1;
    }
    v[i] = value;
}

/// Carry a leader solution over the file's items (`upper`, bits in
/// `order`) to `k`'s items. Items with equal triples are interchangeable,
/// so each file item takes the next unused item of `k` with its triple.
/// The leader also blocks every zero-cost item of `k`, which the file
/// leaves out.
pub fn leader_from_bits(
    k: &Kip,
    ki: &KiFile,
    upper: &str,
    order: BitOrder,
) -> Result<Vec<bool>, String> {
    if upper.len() != ki.items.len() || upper.bytes().any(|b| b != b'0' && b != b'1') {
        return Err(format!(
            "{}: leader solution is not {} bits",
            ki.file,
            ki.items.len()
        ));
    }
    let mut free: HashMap<Item, VecDeque<usize>> = HashMap::new();
    for j in (0..k.n).filter(|&j| k.wu[j] > 0) {
        free.entry((k.p[j], k.wl[j], k.wu[j]))
            .or_default()
            .push_back(j);
    }
    let mut blocked = vec![false; ki.items.len()];
    for (&i, bit) in order.positions(ki).iter().zip(upper.bytes()) {
        blocked[i] = bit == b'1';
    }
    let mut x: Vec<bool> = (0..k.n).map(|j| k.wu[j] == 0).collect();
    for (item, &b) in ki.items.iter().zip(&blocked) {
        let j = free
            .get_mut(item)
            .and_then(VecDeque::pop_front)
            .ok_or_else(|| format!("{}: item {item:?} has no counterpart", ki.file))?;
        x[j] = b;
    }
    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_dict_literals() {
        let d = py_dict(
            r#"{'n': '35', 'ins': 1, 'profit': 'None', 'err': "it's", 'x': None, 'e': '279\n'}"#,
        )
        .unwrap();
        assert_eq!(d["n"], "35");
        assert_eq!(d["ins"], "1");
        assert_eq!(d["profit"], "None");
        assert_eq!(d["err"], "it's");
        assert_eq!(d["e"], "279\n");
        assert!(!d.contains_key("x"));
        assert!(py_dict("{'a': '1'").is_none());
        assert!(py_dict("{'a': '1'} trailing").is_none());
        assert_eq!(py_dict("{}").unwrap().len(), 0);
    }

    #[test]
    fn leader_bits_map_by_triple_and_block_free_items() {
        // BOBILib order: a free item, then two equal items and a third one.
        let k = Kip {
            n: 4,
            p: vec![9, 5, 5, 3],
            wl: vec![1, 2, 2, 1],
            wu: vec![0, 1, 1, 2],
            cl: 3,
            cu: 2,
        };
        // Their order: the distinct item first, the two equal ones after.
        let ki = KiFile {
            file: "t.ki".into(),
            cl: 3,
            cu: 2,
            items: vec![(3, 1, 2), (5, 2, 1), (5, 2, 1)],
            ans: Some(5),
        };
        assert_eq!(kip_signature(&k), ki.signature());
        let file = |ki: &KiFile, bits| leader_from_bits(&k, ki, bits, BitOrder::File);
        assert_eq!(file(&ki, "011").unwrap(), vec![true, true, true, false]);
        assert_eq!(file(&ki, "100").unwrap(), vec![true, false, false, true]);
        assert!(file(&ki, "10").is_err());
        // Listed in sorted order instead, the 3/1 item comes before the 5/2 ones.
        let shuffled = KiFile {
            items: vec![(5, 2, 1), (5, 2, 1), (3, 1, 2)],
            ..ki.clone()
        };
        assert_eq!(BitOrder::Sorted.positions(&shuffled), vec![2, 0, 1]);
        let x = leader_from_bits(&k, &shuffled, "100", BitOrder::Sorted).unwrap();
        assert_eq!(x, vec![true, false, false, true]);
        let other = KiFile {
            items: vec![(3, 1, 2), (5, 2, 1), (4, 2, 1)],
            ..ki
        };
        assert!(file(&other, "000").is_err());
    }

    /// Keys 0..7 from a linear congruential generator, so most elements tie.
    fn lcg_keys(n: usize, seed: u32) -> Vec<u32> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(1103515245).wrapping_add(12345);
                (s >> 16) % 7
            })
            .collect()
    }

    #[test]
    fn libstdcxx_sort_matches_gcc() {
        // std::sort(pi, [&](int i, int j) { return key[i] > key[j]; }) on
        // iota(n), compiled with g++ 13.3: the order among equal keys.
        let gcc: [(u32, &[usize]); 2] = [
            (
                2,
                &[
                    0, 2, 13, 18, 5, 14, 7, 1, 19, 11, 9, 8, 16, 6, 4, 10, 12, 15, 17, 3,
                ],
            ),
            (
                3,
                &[
                    14, 28, 29, 20, 15, 13, 11, 8, 9, 36, 31, 2, 22, 16, 19, 18, 26, 30, 3, 17, 5,
                    7, 23, 24, 37, 27, 0, 38, 1, 21, 10, 12, 33, 35, 6, 4, 39, 25, 32, 34,
                ],
            ),
        ];
        for (seed, want) in gcc {
            let key = lcg_keys(want.len(), seed);
            let mut v: Vec<usize> = (0..want.len()).collect();
            libstdcxx_sort(&mut v, |i, j| key[i] > key[j]);
            assert_eq!(v, want, "seed {seed}");
        }
        // Whatever the ties it sorts, and so does its heapsort fallback.
        for n in [0, 1, 2, 17, 100, 1000] {
            let key = lcg_keys(n, n as u32);
            let mut v: Vec<usize> = (0..n).collect();
            libstdcxx_sort(&mut v, |i, j| key[i] < key[j]);
            assert!(v.windows(2).all(|w| key[w[0]] <= key[w[1]]));
            let mut h: Vec<usize> = (0..n).collect();
            heap_sort(&mut h, &|i, j| key[i] < key[j]);
            assert!(h.windows(2).all(|w| key[w[0]] <= key[w[1]]));
            h.sort_unstable();
            assert!(h.iter().copied().eq(0..n));
        }
    }
}
