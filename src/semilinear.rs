// Semi-linear sets

use crate::deterministic_map::{HashMap, HashSet};
use std::clone::Clone;
pub use std::hash::Hash;

use crate::kleene::Kleene;

use std::sync::atomic::{AtomicBool, Ordering};

pub static REMOVE_REDUNDANT: AtomicBool = AtomicBool::new(true);

pub fn set_remove_redundant(on: bool) {
    REMOVE_REDUNDANT.store(on, Ordering::SeqCst);
}

pub static GENERATE_LESS: AtomicBool = AtomicBool::new(true);

pub fn set_generate_less(on: bool) {
    GENERATE_LESS.store(on, Ordering::SeqCst);
}

/// A sparse vector in d-dimensional nonnegative integer space.
/// Keys represent dimensions and values represent the value at that dimension.
/// Dimensions not present in the HashMap are assumed to be 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseVector<K: Eq + Hash + Clone + Ord> {
    pub values: HashMap<K, usize>,
}

/// Display a sparse vector as a string of the form "ab^3cde^3"
impl<K: Eq + Hash + Clone + Ord + std::fmt::Display> std::fmt::Display for SparseVector<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut entries: Vec<_> = self.values.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        let formatted_entries: Vec<String> = entries
            .into_iter()
            .map(|(key, value)| {
                if *value == 1 {
                    format!("{}", key)
                } else {
                    format!("{}^{}", key, value)
                }
            })
            .collect();
        write!(f, "{}", formatted_entries.join(" "))?;
        Ok(())
    }
}

// Manual implementation of Hash for SparseVector by converting the HashMap to a sorted Vec
impl<K: Eq + Hash + Clone + Ord> Hash for SparseVector<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Convert the HashMap to a sorted Vec of (key, value) pairs
        let mut entries: Vec<_> = self.values.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        // Hash the sorted entries
        entries.hash(state);
    }
}

impl<K: Eq + Hash + Clone + Ord> SparseVector<K> {
    /// Create a new empty sparse vector (all zeros)
    pub fn new() -> Self {
        SparseVector {
            values: HashMap::default(),
        }
    }

    /// Get the value at a specific dimension
    pub fn get(&self, key: &K) -> usize {
        *self.values.get(key).unwrap_or(&0)
    }

    /// Set the value at a specific dimension
    pub fn set(&mut self, key: K, value: usize) {
        if value == 0 {
            self.values.remove(&key);
        } else {
            self.values.insert(key, value);
        }
    }

    /// Create a unit vector with 1 at the specified dimension
    pub fn unit(key: K) -> Self {
        let mut values = HashMap::default();
        values.insert(key, 1);
        SparseVector { values }
    }

    /// Add another vector to this one element-wise
    pub fn add(&self, other: &Self) -> Self {
        let mut result = self.clone();
        for (key, &value) in &other.values {
            let new_value = self.get(key) + value;
            result.set(key.clone(), new_value);
        }
        result
    }

    /// Run an operation on each key
    pub fn for_each_key(&self, mut f: impl for<'a> FnMut(&'a K)) {
        for key in self.values.keys() {
            f(key);
        }
    }

    /// Rename all the keys
    pub fn rename<L: Eq + Hash + Clone + Ord>(self, mut f: impl FnMut(K) -> L) -> SparseVector<L> {
        let mut new_map = HashMap::default();
        for (k, v) in self.values {
            let k = f(k);
            *new_map.entry(k).or_insert(0) += v;
        }
        SparseVector { values: new_map }
    }

    /// Check if the vector is zero
    pub fn is_zero(&self) -> bool {
        self.values.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinearSet<K: Eq + Hash + Clone + Ord> {
    pub base: SparseVector<K>,         // u0: the base vector
    pub periods: Vec<SparseVector<K>>, // [u1, u2, ..., um]: list of period generator vectors
}

impl<K: Eq + Hash + Clone + Ord> LinearSet<K> {
    /// Run an operation on each key mentioned in the linear set
    pub fn for_each_key(&self, mut f: impl for<'a> FnMut(&'a K)) {
        self.base.for_each_key(&mut f);
        for period in &self.periods {
            period.for_each_key(&mut f);
        }
    }

    /// Rename all the keys
    pub fn rename<L: Eq + Hash + Clone + Ord>(self, mut f: impl FnMut(K) -> L) -> LinearSet<L> {
        LinearSet {
            base: self.base.rename(&mut f),
            periods: self.periods.into_iter().map(|p| p.rename(&mut f)).collect(),
        }
    }

    /// Optimize the linear set by deduplicating period vectors, without changing its semantic
    /// meaning.
    pub fn dedup_periods(&mut self) {
        // iteratively remove periods that are linear combinations of others
        'fixpoint: loop {
            // try to find an index i such that periods[i] is a linear combination of the other periods
            for i in 0..self.periods.len() {
                let mut other_periods = self.periods.clone();
                other_periods.remove(i);
                if is_nonnegative_combination(&self.periods[i], &other_periods) {
                    self.periods.remove(i);
                    continue 'fixpoint;
                }
            }
            break;
        }
    }
}

/// Display a linear set as a string of the form "base(period1 + period2 + ...)*"
impl<K: Eq + Hash + Clone + Ord + std::fmt::Display> std::fmt::Display for LinearSet<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base)?;
        if !self.periods.is_empty() {
            if !self.base.to_string().is_empty() {
                write!(f, " ")?;
            }
            write!(f, "(")?;
            for (i, period) in self.periods.iter().enumerate() {
                if i > 0 {
                    write!(f, " + ")?;
                }
                write!(f, "{}", period)?;
            }
            write!(f, ")*")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SemilinearSet<K: Eq + Hash + Clone + Ord> {
    pub components: Vec<LinearSet<K>>, // finite list of linear sets whose union defines the set
}

impl<K: Eq + Hash + Clone + Ord> PartialEq for SemilinearSet<K> {
    fn eq(&self, other: &Self) -> bool {
        let self_components: HashSet<_> = self.components.iter().cloned().collect();
        let other_components: HashSet<_> = other.components.iter().cloned().collect();
        self_components == other_components
    }
}

/// Display a semilinear set as a string of the form "component1 + component2 + ..."
impl<K: Eq + Hash + Clone + Ord + std::fmt::Display> std::fmt::Display for SemilinearSet<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.components
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" + \n")
        )
    }
}

impl<K: Eq + Hash + Clone + Ord> SemilinearSet<K> {
    /// Create a new semilinear set from a list of LinearSet components.
    pub fn new(mut components: Vec<LinearSet<K>>) -> Self {
        // Filter out duplicate period vectors
        if REMOVE_REDUNDANT.load(Ordering::SeqCst) {
            for lin in &mut components {
                lin.dedup_periods();
            }
        }

        // Try merging any of the new_components into another
        if REMOVE_REDUNDANT.load(Ordering::SeqCst) {
            'fixpoint: loop {
                for i in 0..components.len() {
                    for j in i + 1..components.len() {
                        if let Some(merged) = try_merge_linear_sets(&components[i], &components[j])
                        {
                            components[i] = merged;
                            components.swap_remove(j);
                            continue 'fixpoint;
                        }
                    }
                }
                break;
            }
        }
        SemilinearSet { components }
    }

    /// Check if the semilinear set is empty.
    fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Create a semilinear set containing a single vector (an atomic singleton).
    pub fn singleton(vector: SparseVector<K>) -> Self {
        SemilinearSet {
            components: vec![LinearSet {
                base: vector,
                periods: vec![],
            }],
        }
    }

    pub fn atom(k: K) -> Self {
        Self::singleton(SparseVector::unit(k))
    }

    /// Singleton containing the zero vector.
    fn zero() -> Self {
        SemilinearSet::singleton(SparseVector {
            values: HashMap::default(),
        })
    }

    /// The empty semilinear set (contains no vectors).
    fn empty() -> Self {
        SemilinearSet {
            components: Vec::new(),
        }
    }

    /// The universe (all possible sparse vectors) as a semilinear set.
    /// This requires providing a set of possible dimensions.
    pub fn universe(keys: Vec<K>) -> Self {
        // Universe = linear set with base = empty (all zeros), periods = unit vectors for each key
        let base = SparseVector {
            values: HashMap::default(),
        };
        let periods = keys.into_iter().map(SparseVector::unit).collect();
        SemilinearSet::new(vec![LinearSet { base, periods }])
    }

    /// Run an operation on all keys mentioned in the semilinear set
    pub fn for_each_key(&self, mut f: impl for<'a> FnMut(&'a K)) {
        for c in &self.components {
            c.for_each_key(&mut f);
        }
    }

    /// Rename all the keys
    pub fn rename<L: Eq + Hash + Clone + Ord>(self, mut f: impl FnMut(K) -> L) -> SemilinearSet<L> {
        SemilinearSet {
            components: self
                .components
                .into_iter()
                .map(|l| l.rename(&mut f))
                .collect(),
        }
    }
}

/// Returns true if `target` can be expressed as a nonnegative integer combination
/// of the vectors in `periods`.
pub fn is_nonnegative_combination<K: Eq + Hash + Clone + Ord>(
    target: &SparseVector<K>,
    periods: &[SparseVector<K>],
) -> bool {
    // We'll do a DFS with memoization.  The memo stores `(current_vector, index_in_periods)`.
    let mut memo = HashSet::default();
    dfs(target, 0, periods, &mut memo)
}

fn dfs<K: Eq + Hash + Clone + Ord>(
    target: &SparseVector<K>,
    idx: usize,
    periods: &[SparseVector<K>],
    memo: &mut HashSet<(SparseVector<K>, usize)>,
) -> bool {
    // If our target has become the zero vector, we are done.
    if target.values.is_empty() {
        return true;
    }
    // If we've run out of period vectors, and we still haven't zeroed out `target`, fail.
    if idx == periods.len() {
        return false;
    }

    // Check if we already encountered this (vector, index) pair.
    let key = (target.clone(), idx);
    if memo.contains(&key) {
        return false;
    }
    memo.insert(key);

    let p = &periods[idx];

    // We'll compute the maximum times we might subtract p from target in *any* dimension
    // (to keep all coordinates nonnegative).
    let mut max_coeff = usize::MAX;
    for (k, &p_val) in &p.values {
        if p_val > 0 {
            let t_val = target.values.get(k).copied().unwrap_or(0);
            let c = t_val / p_val;
            if c < max_coeff {
                max_coeff = c;
            }
        }
    }

    // We try all coefficients c = 0..=max_coeff.
    // c=0 => skip p entirely, check next.
    for c in 0..=max_coeff {
        if c == 0 {
            // Not using p at all
            if dfs(target, idx + 1, periods, memo) {
                return true;
            }
        } else {
            // Subtract c * p from the target
            let mut new_target = target.clone();
            for (k, &p_val) in &p.values {
                let t_val = new_target.values.get(k).copied().unwrap_or(0);
                let new_val = t_val.saturating_sub(c * p_val);
                if new_val == 0 {
                    new_target.values.remove(k);
                } else {
                    new_target.values.insert(k.clone(), new_val);
                }
            }
            if dfs(&new_target, idx + 1, periods, memo) {
                return true;
            }
        }
    }
    false
}

/// Subtract vector b from vector a, returning a - b, or None if that can't be done nonnegatively.
fn sub_vectors<K: Eq + Hash + Clone + Ord>(
    a: &SparseVector<K>,
    b: &SparseVector<K>,
) -> Option<SparseVector<K>> {
    let mut result = a.clone();
    for (k, &b_val) in &b.values {
        let a_val = result.values.get(k).cloned().unwrap_or(0);
        if b_val > a_val {
            // can't do nonnegative subtraction
            return None;
        }
        let new_val = a_val - b_val;
        if new_val == 0 {
            result.values.remove(k);
        } else {
            result.values.insert(k.clone(), new_val);
        }
    }
    Some(result)
}

/// Check if linear_set1 is contained in linear_set2
/// i.e. L1 ⊆ L2
pub fn linear_set_subset<K: Eq + Hash + Clone + Ord>(l1: &LinearSet<K>, l2: &LinearSet<K>) -> bool {
    // 1. Check if (base1 - base2) is in submonoid(periods2).
    //    We do "base1 - base2" in a nonnegative sense, so if base2 has bigger coords in some dimension,
    //    we can’t do it at all => subset is false.  But sometimes you might want to do "base2 - base1".
    //    Typically for L1 ⊆ L2, we want to see that any vector in L1 can be re-expressed from L2's base + periods,
    //    so we want base1 = base2 + something => base1 - base2 >= 0.
    //
    //    But if base1 < base2 in some dimension, you’d never get base1 from base2 by adding periods.
    //    So the immediate check for L1 ⊆ L2 is: base1 - base2 must be nonnegative in all coords,
    //    then check membership in submonoid generated by l2.periods.
    let Some(diff) = sub_vectors(&l1.base, &l2.base) else {
        return false;
    };
    if !is_nonnegative_combination(&diff, &l2.periods) {
        return false;
    }

    // 2. Check that every period u_i^(1) is in the submonoid of l2.periods as well.
    for p in &l1.periods {
        if !is_nonnegative_combination(p, &l2.periods) {
            return false;
        }
    }

    true
}

/// Attempt to merge two linear sets L1 and L2 into a single linear set L
/// with L1 ∪ L2 = L. Returns Some(merged_set) if successful, else None.
pub fn try_merge_linear_sets<K: Eq + Hash + Clone + Ord>(
    l1: &LinearSet<K>,
    l2: &LinearSet<K>,
) -> Option<LinearSet<K>> {
    if l1 == l2 {
        return Some(l1.clone());
    }
    // Check if l1 is a subset of l2
    if linear_set_subset(l1, l2) {
        return Some(l2.clone());
    }
    // Check if it's aP* and ab(P+b)*
    // Merge into a(P+b)*
    // i.e., if (l2.base - l1.base) \cup periods1 is periods2
    match sub_vectors(&l2.base, &l1.base) {
        Some(diff) => {
            let mut periods1_set: HashSet<SparseVector<K>> = l1.periods.iter().cloned().collect();
            periods1_set.insert(diff);
            let periods2_set: HashSet<SparseVector<K>> = l2.periods.iter().cloned().collect();
            if periods1_set == periods2_set {
                Some(LinearSet {
                    base: l1.base.clone(),
                    periods: l2.periods.clone(),
                })
            } else {
                None
            }
        }
        None => None,
    }
}

/// A very naive membership check:
///    does `vec` ∈ { l.base + Σ α_i l.periods[i] } for some α_i ≥ 0 } ?
fn vector_in_linear_set<K: Eq + Hash + Clone + Ord>(
    vec: &SparseVector<K>,
    lin: &LinearSet<K>,
) -> bool {
    // We want to see if vec - lin.base is in submonoid(lin.periods).
    // If (vec < lin.base) in some dimension => false immediately.
    if let Some(diff) = sub_vectors(vec, &lin.base) {
        is_nonnegative_combination(&diff, &lin.periods)
    } else {
        false
    }
}

impl<K: Eq + Hash + Clone + Ord> Kleene for SemilinearSet<K> {
    fn zero() -> Self {
        SemilinearSet::empty()
    }

    fn one() -> Self {
        SemilinearSet::zero()
    }

    // Union of two semilinear sets.
    fn plus(mut self, mut other: Self) -> Self {
        // Clone components of both and combine
        if GENERATE_LESS.load(Ordering::SeqCst) {
            self.components.append(&mut other.components);
            SemilinearSet::new(self.components)
        } else {
            // naive: tack on without any dedupe/merge
            self.components.extend(other.components);
            SemilinearSet {
                components: self.components,
            }
        }
    }

    // Sequential composition (a.k.a. Minkowski sum) of two semilinear sets.
    fn times(self, other: Self) -> Self {
        let mut comps = Vec::new();
        for a in &self.components {
            for b in &other.components {
                let mut lin = a.clone();
                lin.base = lin.base.add(&b.base);
                lin.periods.extend(b.periods.clone());
                comps.push(lin);
            }
        }
        if GENERATE_LESS.load(Ordering::SeqCst) {
            SemilinearSet::new(comps)
        } else {
            SemilinearSet { components: comps }
        }
    }

    fn star(self) -> Self {
        // Lots of heuristic optimizations to prevent blow up.
        // As a reminder, a linear set looks like:  b(p1+...+pN)*  and a SLS is a union of these.
        //
        // 1. Pull out linear sets with zero base.
        //      (p* + ...)* = p*(...)*
        //
        // 2. Remove redundant periods.
        //      p*(b(p+q)* + ...)* = p*(bq* + ...)*
        //
        // 3. Pull out bases with no periods.
        //      (b + ...)* = b*(...)*
        //
        // 4. Lastly, for any combinations that are left, we have to do the slow thing of
        //      (bp* + ...)* = bb*p*(...)* + (...)*
        //    recursively.
        let mut extra_periods = HashSet::default();

        // 1. Pull out linear sets with zero base.
        let mut components = self.components;
        if GENERATE_LESS.load(Ordering::SeqCst) {
            components.retain(|comp| {
                if comp.base.is_zero() {
                    for p in &comp.periods {
                        extra_periods.insert(p.clone());
                    }
                    false
                } else {
                    true
                }
            });

            // 2+3. Remove redundant periods, and pull out bases with no periods.
            loop {
                let mut modified = false;
                components.retain_mut(|comp| {
                    // Remove redundant periods.
                    if REMOVE_REDUNDANT.load(Ordering::SeqCst) {
                        comp.periods.retain(|p| !extra_periods.contains(p));
                    }
                    // If the component has no periods, we add its base to extra_periods
                    if comp.periods.is_empty() {
                        extra_periods.insert(comp.base.clone());
                        modified = true;
                        false
                    } else {
                        true
                    }
                });
                if !modified {
                    break;
                }
            }
        }

        // 4. Slow, recursive thing (but not actually implemented recursively)
        let mut result_components = Vec::new();
        // We use bit masks to iterate over all non-empty subsets of components
        let n = components.len();
        if n > 30 {
            // Log this as a timeout before panicking
            crate::stats::set_analysis_result("timeout");
            crate::stats::finalize_stats();
            panic!("Number of components in semilinear set is too large");
        }
        for mask in 0..(1 << n) {
            // Determine subset X for this mask
            let mut subset_base = SparseVector {
                values: HashMap::default(),
            };
            let mut subset_periods: Vec<SparseVector<K>> = Vec::new();

            for (i, comp) in components.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    // add this component's base to subset_base
                    subset_base = subset_base.add(&comp.base);
                    // include this component's base and periods in subset_periods
                    subset_periods.push(comp.base.clone());
                    for p in &comp.periods {
                        subset_periods.push(p.clone());
                    }
                }
            }
            // Create the linear set for this subset
            result_components.push(LinearSet {
                base: subset_base,
                periods: subset_periods,
            });
        }

        // Add the extra periods to all the components
        for comp in &mut result_components {
            for p in &extra_periods {
                comp.periods.push(p.clone());
            }
        }
        
        if GENERATE_LESS.load(Ordering::SeqCst) {
            SemilinearSet::new(result_components)
        } else {
            SemilinearSet {
                components: result_components,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_vector_operations() {
        let mut v1 = SparseVector {
            values: HashMap::default(),
        };
        v1.set("x".to_string(), 1);
        v1.set("y".to_string(), 2);

        let mut v2 = SparseVector {
            values: HashMap::default(),
        };
        v2.set("y".to_string(), 3);
        v2.set("z".to_string(), 4);

        let sum = v1.add(&v2);
        assert_eq!(sum.get(&"x".to_string()), 1);
        assert_eq!(sum.get(&"y".to_string()), 5);
        assert_eq!(sum.get(&"z".to_string()), 4);
        assert_eq!(sum.get(&"w".to_string()), 0); // Non-existent key
    }

    #[test]
    fn test_semilinear_set_union() {
        let mut v1 = SparseVector {
            values: HashMap::default(),
        };
        v1.set("x".to_string(), 1);
        v1.set("y".to_string(), 2);

        let mut v2 = SparseVector {
            values: HashMap::default(),
        };
        v2.set("y".to_string(), 3);
        v2.set("z".to_string(), 4);

        let set1 = SemilinearSet::singleton(v1.clone());
        let set2: SemilinearSet<String> = SemilinearSet::singleton(v2.clone());
        let union = set1.plus(set2);

        assert_eq!(union.components.len(), 2);
        // Check that the components contain our original vectors
        assert!(union.components.iter().any(|c| c.base == v1));
        assert!(union.components.iter().any(|c| c.base == v2));
    }

    #[test]
    fn test_semilinear_set_add() {
        let mut v1 = SparseVector {
            values: HashMap::default(),
        };
        v1.set("x".to_string(), 1);

        let mut v2 = SparseVector {
            values: HashMap::default(),
        };
        v2.set("y".to_string(), 2);

        let set1 = SemilinearSet::singleton(v1);
        let set2 = SemilinearSet::singleton(v2);
        let sum = set1.times(set2);

        assert_eq!(sum.components.len(), 1);
        let result_vector = &sum.components[0].base;
        assert_eq!(result_vector.get(&"x".to_string()), 1);
        assert_eq!(result_vector.get(&"y".to_string()), 2);
    }

    #[test]
    fn test_a_star_proper() {
        // Use the Kleene operations to compute a*
        let a = SemilinearSet::singleton(SparseVector::unit("a"));
        let a_star = a.star();

        // Define the ground truth using the semilinear set constructors
        let ground_truth_a_star = SemilinearSet::new(vec![LinearSet {
            base: SparseVector {
                values: HashMap::default(),
            },
            periods: vec![SparseVector::unit("a")],
        }]);

        assert_eq!(a_star, ground_truth_a_star);
    }

    #[test]
    fn test_b_times_c_proper() {
        // Use the Kleene operations to compute b;c
        let b = SemilinearSet::singleton(SparseVector::unit("b".to_string()));
        let c = SemilinearSet::singleton(SparseVector::unit("c".to_string()));
        let b_times_c = b.clone().times(c.clone());
        let c_times_b = c.times(b);
        // check symetry
        assert_eq!(b_times_c, c_times_b);

        let mut b_time_c_sparse_vector = SparseVector {
            values: HashMap::default(),
        };
        b_time_c_sparse_vector.set("b".to_string(), 1);
        b_time_c_sparse_vector.set("c".to_string(), 1);

        // Define the ground truth using the semilinear set constructors
        let ground_truth_b_times_c = SemilinearSet::new(vec![LinearSet {
            base: b_time_c_sparse_vector,
            periods: vec![],
        }]);

        assert_eq!(b_times_c, ground_truth_b_times_c);
        // println!("{:?}", b_times_c);
        // println!("done!!!");
    }

    #[test]
    fn test_a_star_times_b_proper() {
        // Use the Kleene operations to compute a*
        let a = SemilinearSet::singleton(SparseVector::unit("a".to_string()));
        let a_star = a.star();

        let b = SemilinearSet::singleton(SparseVector::unit("b".to_string()));
        // Use the Kleene operations to compute (a*);b
        let a_star_times_b = a_star.times(b);

        let mut a_b = SparseVector {
            values: HashMap::default(),
        };
        a_b.set("a".to_string(), 1);
        a_b.set("b".to_string(), 1);

        // Define the ground truth using the semilinear set constructors
        let ground_truth_a_star_times_b = SemilinearSet::new(vec![
            LinearSet {
                base: SparseVector::unit("b".to_string()), // Ensure consistency
                periods: vec![],
            },
            LinearSet {
                base: a_b,
                periods: vec![SparseVector::unit("a".to_string())], // Ensure consistency
            },
        ]);

        assert_eq!(a_star_times_b, ground_truth_a_star_times_b);
    }

    #[test]
    fn test_a_star_times_b_plus_b_times_c_proper() {
        // Use the Kleene operations to compute a*
        let a = SemilinearSet::singleton(SparseVector::unit("a".to_string()));
        let a_star = a.star();

        let b = SemilinearSet::singleton(SparseVector::unit("b".to_string()));
        // Use the Kleene operations to compute (a*);b
        let a_star_times_b = a_star.times(b.clone());

        let mut a_b = SparseVector {
            values: HashMap::default(),
        };
        a_b.set("a".to_string(), 1);
        a_b.set("b".to_string(), 1);

        let c = SemilinearSet::singleton(SparseVector::unit("c".to_string()));
        let b_times_c = b.times(c);
        let b_times_c_clone = b_times_c.clone(); // Clone before moving

        let a_star_times_b_plus_b_times_c = a_star_times_b.plus(b_times_c);

        // Define the ground truth using the semilinear set constructors
        let ground_truth_a_star_times_b_plus_b_times_c = SemilinearSet::new(vec![
            LinearSet {
                base: SparseVector::unit("b".to_string()), // Ensure consistency
                periods: vec![],
            },
            LinearSet {
                base: a_b,
                periods: vec![SparseVector::unit("a".to_string())], // Ensure consistency
            },
            LinearSet {
                base: b_times_c_clone.components[0].base.clone(),
                periods: vec![], // Ensure consistency
            },
        ]);

        assert_eq!(
            a_star_times_b_plus_b_times_c,
            ground_truth_a_star_times_b_plus_b_times_c
        );
    }
}



