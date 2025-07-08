// Kleene algebra trait with operations:
// - star
// - plus
// - times
// - one
// - zero

use crate::deterministic_map::{HashMap, HashSet};

use std::sync::atomic::{AtomicBool, Ordering};

use crate::semilinear::GENERATE_LESS;

pub static SMART_ORDER: AtomicBool = AtomicBool::new(true);

pub fn set_smart_kleene_order(on: bool) {
    SMART_ORDER.store(on, Ordering::SeqCst);
}

pub trait Kleene {
    fn zero() -> Self;
    fn one() -> Self;
    fn plus(self, other: Self) -> Self;
    fn times(self, other: Self) -> Self;
    fn star(self) -> Self;
}

impl Kleene for bool {
    fn zero() -> Self {
        false
    }
    fn one() -> Self {
        true
    }
    fn plus(self, other: Self) -> Self {
        self || other
    }
    fn times(self, other: Self) -> Self {
        self && other
    }
    fn star(self) -> Self {
        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Regex<T> {
    Atom(T),
    Zero,
    One,
    Plus(Box<Regex<T>>, Box<Regex<T>>),
    Times(Box<Regex<T>>, Box<Regex<T>>),
    Star(Box<Regex<T>>),
}

impl<T: std::fmt::Display> std::fmt::Display for Regex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Regex::Atom(c) => write!(f, "{}", c),
            Regex::Zero => write!(f, "0"),
            Regex::One => write!(f, "1"),
            Regex::Plus(a, b) => write!(f, "({} + {})", a, b),
            Regex::Times(a, b) => write!(f, "({} · {})", a, b),
            Regex::Star(a) => write!(f, "({})*", a),
        }
    }
}

impl<T> Kleene for Regex<T> {
    fn zero() -> Self {
        Regex::Zero
    }
    fn one() -> Self {
        Regex::One
    }
    fn plus(self, other: Self) -> Self {
        if GENERATE_LESS.load(Ordering::SeqCst) {
            match (self, other) {
                (Regex::Zero, x) | (x, Regex::Zero) => x,
                (a, b) => Regex::Plus(Box::new(a), Box::new(b)),
            }
        } else {
            // naive: always build a Plus node
            Regex::Plus(Box::new(self), Box::new(other))
        }
    }
    fn times(self, other: Self) -> Self {
        if GENERATE_LESS.load(Ordering::SeqCst) {
            match (self, other) {
                (Regex::Zero, _) | (_, Regex::Zero) => Regex::Zero,
                (Regex::One, x) | (x, Regex::One) => x,
                (a, b) => Regex::Times(Box::new(a), Box::new(b)),
            }
        } else {
            // naive: always build a Times node
            Regex::Times(Box::new(self), Box::new(other))
        }
    }
    fn star(self) -> Self {
        if GENERATE_LESS.load(Ordering::SeqCst) {
            match self {
                Regex::Zero | Regex::One => Regex::One,
                Regex::Star(x) => Regex::Star(x),
                x => Regex::Star(Box::new(x)),
            }
        } else {
            // naive: always build a Star node
            Regex::Star(Box::new(self))
        }
    }
}

// Kleene's algorithm for converting a NFA to a Kleene algebra
// Takes a start state and computes the Kleene element for going from the start state to any other state
pub fn nfa_to_kleene<S: Clone + Eq + std::hash::Hash, K: Kleene + Clone>(
    nfa_vec: &[(S, K, S)],
    start: S,
) -> K {
    // We add an extra state `None` and eliminate all states except that one

    let mut nfa: HashMap<(Option<&S>, Option<&S>), K> = HashMap::default();
    for (from, k, to) in nfa_vec.iter() {
        nfa.entry((Some(from), Some(to)))
            .and_modify(|e| *e = e.clone().plus(k.clone()))
            .or_insert(k.clone());
    }

    // Add epsilon edge from None to start
    nfa.entry((None, Some(&start)))
        .and_modify(|e| *e = e.clone().plus(K::one()))
        .or_insert(K::one());

    let mut states_todo = nfa_vec
        .iter()
        .flat_map(|(from, _, to)| vec![from, to])
        .collect::<HashSet<_>>();

    states_todo.insert(&start);

    // Insert epsilon edges from all states_todo to None
    for state in states_todo.iter() {
        nfa.entry((Some(state), None))
            .and_modify(|e| *e = e.clone().plus(K::one()))
            .or_insert(K::one());
    }

    while !states_todo.is_empty() {
        let state = *states_todo
            .iter()
            .min_by_key(|s| {
                // Optionally, disable the heuristics for picking the next state
                if !SMART_ORDER.load(Ordering::SeqCst) {
                    return 0;
                }
                let mut count = 0;
                for ((_, to), _) in nfa.iter() {
                    if to == &Some(**s) && !nfa.contains_key(&(Some(s), *to)) {
                        count += 1;
                    }
                }
                for ((from, _), _) in nfa.iter() {
                    if from == &Some(**s) && !nfa.contains_key(&(*from, Some(s))) {
                        count += 1;
                    }
                }
                count
            })
            .unwrap();
        states_todo.remove(&state);
        let mut new_nfa: Vec<(Option<&S>, Option<&S>, K)> = vec![];
        let mut incoming: Vec<(Option<&S>, Option<&S>, K)> = vec![];
        let mut outgoing: Vec<(Option<&S>, Option<&S>, K)> = vec![];
        let mut self_loops: Vec<(Option<&S>, Option<&S>, K)> = vec![];

        for ((from, to), k) in nfa.iter() {
            let edge: (Option<&S>, Option<&S>, K) = (*from, *to, k.clone());
            if from == &Some(state) && to == &Some(state) {
                self_loops.push(edge);
            } else if from == &Some(state) {
                outgoing.push(edge);
            } else if to == &Some(state) {
                incoming.push(edge);
            } else {
                new_nfa.push(edge);
            }
        }
        // Add up the self loops
        let self_loop = self_loops
            .iter()
            .map(|(_, _, k)| k)
            .fold(K::zero(), |acc, k| acc.plus(k.clone()))
            .star();
        // Insert all the shortcut edges into the new NFA
        for (from, _, k1) in incoming.iter() {
            for (_, to, k2) in outgoing.iter() {
                new_nfa.push((
                    *from,
                    *to,
                    k1.clone().times(self_loop.clone().times(k2.clone())),
                ));
            }
        }
        let mut new_nfa_map: HashMap<(Option<&S>, Option<&S>), K> = HashMap::default();
        for (from, to, k) in new_nfa.iter() {
            new_nfa_map
                .entry((*from, *to))
                .and_modify(|e| *e = e.clone().plus(k.clone()))
                .or_insert(k.clone());
        }
        nfa = new_nfa_map;
    }
    let mut answer = K::zero();
    for ((from, to), k) in nfa.iter() {
        assert!(from.is_none());
        assert!(to.is_none());
        answer = answer.plus(k.clone());
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nfa_to_kleene() {
        // Create a simple NFA with 3 states
        let nfa = vec![
            (0, Regex::Atom('a'), 1), // State 0 to 1 with label 'a'
            (1, Regex::Atom('b'), 2), // State 1 to 2 with label 'b'
            (2, Regex::Atom('c'), 0), // State 2 back to 0 with label 'c'
            (1, Regex::Atom('d'), 1), // Self loop on state 1 with label 'd'
        ];

        let result = nfa_to_kleene(&nfa, 0);

        // Check if characters 'a', 'b', 'c', and 'd' are in the result exactly once
        let mut chars = HashSet::default();
        for c in result.to_string().chars() {
            if c.is_ascii_alphabetic() {
                chars.insert(c);
            }
        }
        assert_eq!(chars.len(), 4);
        assert!(chars.contains(&'a'));
        assert!(chars.contains(&'b'));
        assert!(chars.contains(&'c'));
        assert!(chars.contains(&'d'));
    }
}
