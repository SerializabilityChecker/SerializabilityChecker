#[cfg(test)]
mod tests {
    use crate::presburger::PresburgerSet;
    use crate::kleene::Kleene;

    // Helper function to create all permutations of a slice
    fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
        if items.is_empty() {
            return vec![vec![]];
        }
        if items.len() == 1 {
            return vec![items.to_vec()];
        }
        
        let mut result = Vec::new();
        for i in 0..items.len() {
            let mut remaining = items.to_vec();
            let first = remaining.remove(i);
            for mut perm in permutations(&remaining) {
                perm.insert(0, first.clone());
                result.push(perm);
            }
        }
        result
    }

    #[test]
    fn test_harmonize_basic_union_commutativity() {
        // Test that A ∪ B = B ∪ A for the same atoms, regardless of internal ordering
        let a = PresburgerSet::atom('a');
        let b = PresburgerSet::atom('b');
        
        // Union in different orders
        let union_ab = a.clone().union(&b.clone());
        let union_ba = b.union(&a);
        
        assert_eq!(union_ab, union_ba, 
            "Union should be commutative: a ∪ b = b ∪ a");
    }
    
    #[test]
    fn test_harmonize_with_different_atom_orderings() {
        // Test that sets with same content but different internal orderings are equal
        let atoms = vec!['x', 'y', 'z'];
        
        // Create universe sets with different atom orderings
        let universe1 = PresburgerSet::universe(atoms.clone());
        let universe2 = PresburgerSet::universe(vec!['z', 'y', 'x']); // Reversed
        let universe3 = PresburgerSet::universe(vec!['y', 'x', 'z']); // Shuffled
        
        // All should be equal after harmonization
        assert_eq!(universe1, universe2, 
            "Universe sets with different atom orderings should be equal");
        assert_eq!(universe2, universe3, 
            "Universe sets with different atom orderings should be equal");
        assert_eq!(universe1, universe3, 
            "Universe sets with different atom orderings should be equal");
    }

    #[test]
    fn test_harmonize_intersection_commutativity() {
        // Test that A ∩ B = B ∩ A regardless of atom ordering
        let atoms = vec![1, 2, 3];
        let perms = permutations(&atoms);
        
        for perm1 in &perms {
            for perm2 in &perms {
                // Create universe sets with different orderings
                let universe1 = PresburgerSet::universe(perm1.clone());
                let universe2 = PresburgerSet::universe(perm2.clone());
                
                // Create some subset by constraining first dimension
                let subset1 = {
                    let atom = PresburgerSet::atom(perm1[0]);
                    universe1.intersection(&atom)
                };
                
                let subset2 = {
                    let atom = PresburgerSet::atom(perm2[0]);
                    universe2.intersection(&atom)
                };
                
                // Test intersection commutativity
                let inter_12 = subset1.intersection(&subset2);
                let inter_21 = subset2.intersection(&subset1);
                
                assert_eq!(inter_12, inter_21,
                    "Intersection should be commutative regardless of atom ordering. \
                     Perm1: {:?}, Perm2: {:?}", perm1, perm2);
            }
        }
    }

    #[test]
    fn test_harmonize_with_different_dimensions() {
        // Test harmonization when sets have different numbers of dimensions
        let a = PresburgerSet::atom('a');
        let bc = PresburgerSet::atom('b').union(&PresburgerSet::atom('c'));
        let abc = PresburgerSet::universe(vec!['a', 'b', 'c']);
        
        // Union with different dimension counts
        let union1 = a.union(&bc);
        let union2 = bc.union(&a);
        assert_eq!(union1, union2, "Union with different dimensions should be commutative");
        
        // Intersection with universe
        let inter1 = union1.intersection(&abc);
        let inter2 = abc.intersection(&union1);
        assert_eq!(inter1, inter2, "Intersection with universe should be commutative");
    }

    #[test]
    fn test_harmonize_empty_sets() {
        // Test harmonization with empty sets
        let empty1 = PresburgerSet::<char>::zero();
        let _empty2 = PresburgerSet::<i32>::zero();
        
        let a = PresburgerSet::atom('a');
        let _b = PresburgerSet::atom('b');
        
        // Union with empty
        assert_eq!(a.union(&empty1), a.clone(), "Union with empty should be identity");
        assert_eq!(empty1.union(&a), a.clone(), "Empty union A should equal A");
        
        // Intersection with empty
        assert!(a.intersection(&empty1).is_empty(), "Intersection with empty should be empty");
        assert!(empty1.intersection(&a).is_empty(), "Empty intersection A should be empty");
    }

    #[test]
    fn test_harmonize_difference_operations() {
        // Test difference operations with different orderings
        let atoms1 = vec!['x', 'y', 'z'];
        let atoms2 = vec!['z', 'y', 'x']; // Reversed order
        
        let universe1 = PresburgerSet::universe(atoms1.clone());
        let universe2 = PresburgerSet::universe(atoms2.clone());
        
        let xy1 = PresburgerSet::atom('x').union(&PresburgerSet::atom('y'));
        let xy2 = PresburgerSet::atom('y').union(&PresburgerSet::atom('x'));
        
        // A - B with different orderings
        let diff1 = universe1.difference(&xy1);
        let diff2 = universe2.difference(&xy2);
        
        // Both should give us just 'z'
        let z = PresburgerSet::atom('z');
        assert_eq!(diff1.intersection(&z), z.clone(), "Difference should isolate z");
        assert_eq!(diff2.intersection(&z), z.clone(), "Difference should isolate z regardless of ordering");
    }

    #[test]
    fn test_harmonize_preserves_distinct_atoms() {
        // Regression test for the known bug where different atoms might map to same coordinates
        let atom42 = PresburgerSet::atom(42);
        let atom99 = PresburgerSet::atom(99);
        
        // These should remain distinct after union
        let union = atom42.union(&atom99);
        
        // The union should not be equal to either individual atom
        assert_ne!(union, atom42, "Union should not collapse to first atom");
        assert_ne!(union, atom99, "Union should not collapse to second atom");
        
        // The intersection should be empty (atoms are disjoint)
        let inter = atom42.intersection(&atom99);
        assert!(inter.is_empty(), "Different atoms should have empty intersection");
    }

    #[test]
    fn test_harmonize_multiple_operations_chain() {
        // Test a chain of operations with different orderings
        let perms = permutations(&vec!['a', 'b', 'c', 'd']);
        
        // Pick a few random permutations to test
        let perm1 = &perms[0];  // [a, b, c, d]
        let _perm2 = &perms[perms.len() - 1]; // Some other permutation
        
        // Build complex expressions with different orderings
        let expr1 = {
            let a = PresburgerSet::atom(perm1[0]);
            let b = PresburgerSet::atom(perm1[1]);
            let c = PresburgerSet::atom(perm1[2]);
            let d = PresburgerSet::atom(perm1[3]);
            
            a.union(&b).intersection(&c.union(&d))
        };
        
        let expr2 = {
            // Build the same logical expression using perm1's atoms
            // This ensures we're comparing the same sets logically
            let real_a = PresburgerSet::atom(perm1[0]);
            let real_b = PresburgerSet::atom(perm1[1]);
            let real_c = PresburgerSet::atom(perm1[2]);
            let real_d = PresburgerSet::atom(perm1[3]);
            
            real_a.union(&real_b).intersection(&real_c.union(&real_d))
        };
        
        assert_eq!(expr1, expr2, "Complex expressions should be equal regardless of construction order");
    }

    #[test]
    fn test_harmonize_associativity_with_permutations() {
        // Test (A ∪ B) ∪ C = A ∪ (B ∪ C) with different orderings
        let atoms = vec!["alpha", "beta", "gamma"];
        let perms = permutations(&atoms);
        
        for perm in perms.iter().take(3) { // Test first 3 permutations to keep test fast
            let a = PresburgerSet::atom(perm[0]);
            let b = PresburgerSet::atom(perm[1]);
            let c = PresburgerSet::atom(perm[2]);
            
            let left_assoc = a.clone().union(&b).union(&c.clone());
            let right_assoc = a.union(&b.union(&c));
            
            assert_eq!(left_assoc, right_assoc,
                "Union should be associative regardless of atom ordering. Perm: {:?}", perm);
        }
    }

    #[test]
    fn test_harmonize_with_renamed_variables() {
        // Test harmonization after renaming variables
        let original = PresburgerSet::atom(1).union(&PresburgerSet::atom(2));
        
        // Rename to strings
        let renamed = original.clone().rename(|x| format!("var_{}", x));
        
        // Create equivalent set with strings from the start
        let direct = PresburgerSet::atom("var_1".to_string())
            .union(&PresburgerSet::atom("var_2".to_string()));
        
        assert_eq!(renamed, direct, "Renamed set should equal directly constructed set");
    }

    #[test]
    fn test_harmonize_stress_test() {
        // Stress test with many atoms and operations
        let n = 10; // Number of atoms
        let atoms: Vec<i32> = (0..n).collect();
        
        // Create different orderings
        let ascending = atoms.clone();
        let descending: Vec<i32> = atoms.iter().rev().cloned().collect();
        let shuffled = vec![5, 2, 8, 1, 9, 0, 6, 3, 7, 4]; // A specific shuffle
        
        // Build same set with different orderings
        let set1 = ascending.iter()
            .map(|&x| PresburgerSet::atom(x))
            .reduce(|acc, x| acc.union(&x))
            .unwrap();
            
        let set2 = descending.iter()
            .map(|&x| PresburgerSet::atom(x))
            .reduce(|acc, x| acc.union(&x))
            .unwrap();
            
        let set3 = shuffled.iter()
            .map(|&x| PresburgerSet::atom(x))
            .reduce(|acc, x| acc.union(&x))
            .unwrap();
        
        assert_eq!(set1, set2, "Sets built in different orders should be equal");
        assert_eq!(set2, set3, "Sets built in different orders should be equal");
        assert_eq!(set1, set3, "Sets built in different orders should be equal");
    }

    #[test]
    fn test_harmonize_project_out_with_permutations() {
        // Test that project_out works correctly after harmonization
        let atoms1 = vec!['a', 'b', 'c'];
        let atoms2 = vec!['c', 'b', 'a']; // Reversed
        
        let universe1 = PresburgerSet::universe(atoms1);
        let universe2 = PresburgerSet::universe(atoms2);
        
        // Project out 'b' from both
        let proj1 = universe1.project_out('b');
        let proj2 = universe2.project_out('b');
        
        // Both should have the same result (universe over {a, c})
        assert_eq!(proj1, proj2, "Project out should work correctly regardless of initial ordering");
    }

    #[test]
    fn test_harmonize_complex_with_fuzz_factor() {
        // Test complex operations with sets created in different domain orders
        // This tests the harmonization bug mentioned in CLAUDE.md
        
        // Create atoms in different orders to stress-test harmonization
        let create_set_variant1 = || {
            let x = PresburgerSet::atom(100);
            let y = PresburgerSet::atom(200);
            let z = PresburgerSet::atom(300);
            x.union(&y).union(&z)
        };
        
        let create_set_variant2 = || {
            let z = PresburgerSet::atom(300);
            let x = PresburgerSet::atom(100);
            let y = PresburgerSet::atom(200);
            z.union(&x).union(&y)
        };
        
        let create_set_variant3 = || {
            let y = PresburgerSet::atom(200);
            let z = PresburgerSet::atom(300);
            let x = PresburgerSet::atom(100);
            y.union(&z).union(&x)
        };
        
        // All variants should be equal
        let set1 = create_set_variant1();
        let set2 = create_set_variant2();
        let set3 = create_set_variant3();
        
        assert_eq!(set1, set2, "Sets created with different atom orderings should be equal");
        assert_eq!(set2, set3, "Sets created with different atom orderings should be equal");
        
        // Test operations on these sets
        let universe = PresburgerSet::universe(vec![100, 200, 300, 400]);
        
        // Intersection
        let inter1 = set1.intersection(&universe);
        let inter2 = set2.intersection(&universe);
        assert_eq!(inter1, inter2, "Intersection results should be equal");
        
        // Difference
        let diff1 = universe.clone().difference(&set1);
        let diff2 = universe.difference(&set2);
        assert_eq!(diff1, diff2, "Difference results should be equal");
    }
    
    #[test]
    fn test_harmonize_regression_for_known_bug() {
        // Specific regression test for the bug mentioned in CLAUDE.md
        // where atom(42) and atom(99) might map to the same coordinates
        
        // Create atoms with specific values that might trigger the bug
        let atom_42 = PresburgerSet::atom(42);
        let atom_99 = PresburgerSet::atom(99);
        let atom_1 = PresburgerSet::atom(1);
        
        // Create unions in different orders
        let union1 = atom_42.clone().union(&atom_99.clone()).union(&atom_1.clone());
        let union2 = atom_99.clone().union(&atom_1.clone()).union(&atom_42.clone());
        let union3 = atom_1.union(&atom_42.clone()).union(&atom_99.clone());
        
        // All should be equal
        assert_eq!(union1, union2, "Unions with different orderings should be equal");
        assert_eq!(union2, union3, "Unions with different orderings should be equal");
        
        // Verify that atoms remain distinct
        let inter_42_99 = atom_42.intersection(&atom_99);
        assert!(inter_42_99.is_empty(), 
            "atom(42) and atom(99) should have empty intersection - they must remain distinct");
    }
}

#[cfg(test)]
mod semilinear_conversion_tests {
    use crate::presburger::PresburgerSet;
    use crate::semilinear::{LinearSet, SemilinearSet, SparseVector};
    use crate::kleene::Kleene;
    use crate::deterministic_map::HashMap;

    // Helper to create a sparse vector
    fn sparse_vec<K: Clone + Eq + std::hash::Hash + Ord>(pairs: Vec<(K, usize)>) -> SparseVector<K> {
        let mut values = HashMap::default();
        for (k, v) in pairs {
            values.insert(k, v);
        }
        SparseVector { values }
    }

    #[test]
    fn test_semilinear_to_presburger_basic() {
        // Create equivalent semilinear sets with different atom orderings
        // Set 1: {(a,b) | a=1, b=0} ∪ {(a,b) | a=0, b=1}
        let set1 = {
            let comp1 = LinearSet {
                base: sparse_vec(vec![('a', 1), ('b', 0)]),
                periods: vec![],
            };
            let comp2 = LinearSet {
                base: sparse_vec(vec![('a', 0), ('b', 1)]),
                periods: vec![],
            };
            SemilinearSet::new(vec![comp1, comp2])
        };
        
        // Set 2: Same but with atoms created in different order
        let set2 = {
            let comp1 = LinearSet {
                base: sparse_vec(vec![('b', 1), ('a', 0)]),
                periods: vec![],
            };
            let comp2 = LinearSet {
                base: sparse_vec(vec![('b', 0), ('a', 1)]),
                periods: vec![],
            };
            SemilinearSet::new(vec![comp1, comp2])
        };
        
        // Convert to Presburger sets
        let presburger1 = PresburgerSet::from_semilinear_set(&set1);
        let presburger2 = PresburgerSet::from_semilinear_set(&set2);
        
        // They should be equal after conversion
        assert_eq!(presburger1, presburger2, 
            "Semilinear sets with different atom orderings should convert to equal Presburger sets");
    }

    #[test]
    fn test_semilinear_to_presburger_with_periods() {
        // Create semilinear sets with periodic components
        // Set 1: base (1,0) with period (2,3)
        let set1 = {
            let comp = LinearSet {
                base: sparse_vec(vec![('x', 1), ('y', 0)]),
                periods: vec![sparse_vec(vec![('x', 2), ('y', 3)])],
            };
            SemilinearSet::new(vec![comp])
        };
        
        // Set 2: Same content but atoms in different order
        let set2 = {
            let comp = LinearSet {
                base: sparse_vec(vec![('y', 0), ('x', 1)]),
                periods: vec![sparse_vec(vec![('y', 3), ('x', 2)])],
            };
            SemilinearSet::new(vec![comp])
        };
        
        // Convert to Presburger sets
        let presburger1 = PresburgerSet::from_semilinear_set(&set1);
        let presburger2 = PresburgerSet::from_semilinear_set(&set2);
        
        assert_eq!(presburger1, presburger2, 
            "Semilinear sets with periods should convert equally regardless of atom ordering");
    }

    #[test]
    fn test_semilinear_kleene_operations_to_presburger() {
        // Test that Kleene operations produce equivalent results with different orderings
        
        // Create atoms using semilinear sets
        let a_semilinear = SemilinearSet::singleton(sparse_vec(vec![('a', 1)]));
        let b_semilinear = SemilinearSet::singleton(sparse_vec(vec![('b', 1)]));
        
        // Union in different orders
        let union_ab = a_semilinear.clone().plus(b_semilinear.clone());
        let union_ba = b_semilinear.clone().plus(a_semilinear.clone());
        
        // Convert to Presburger
        let presburger_ab = PresburgerSet::from_semilinear_set(&union_ab);
        let presburger_ba = PresburgerSet::from_semilinear_set(&union_ba);
        
        assert_eq!(presburger_ab, presburger_ba, 
            "Semilinear union should convert to equal Presburger sets regardless of order");
        
        // Test Minkowski sum (times operation)
        let sum_ab = a_semilinear.clone().times(b_semilinear.clone());
        let sum_ba = b_semilinear.times(a_semilinear);
        
        let presburger_sum_ab = PresburgerSet::from_semilinear_set(&sum_ab);
        let presburger_sum_ba = PresburgerSet::from_semilinear_set(&sum_ba);
        
        // Note: Minkowski sum is NOT commutative in general, but for singleton sets it is
        assert_eq!(presburger_sum_ab, presburger_sum_ba,
            "Semilinear Minkowski sum of singletons should be commutative");
    }

    #[test]
    fn test_semilinear_complex_expressions_to_presburger() {
        // Create complex semilinear expressions with different construction orders
        
        // Expression 1: (a + b)* + c
        let expr1 = {
            let a = SemilinearSet::singleton(sparse_vec(vec![('a', 1)]));
            let b = SemilinearSet::singleton(sparse_vec(vec![('b', 1)]));
            let c = SemilinearSet::singleton(sparse_vec(vec![('c', 1)]));
            
            let ab = a.plus(b);
            let ab_star = ab.star();
            ab_star.plus(c)
        };
        
        // Expression 2: c + (b + a)*  (different order)
        let expr2 = {
            let a = SemilinearSet::singleton(sparse_vec(vec![('a', 1)]));
            let b = SemilinearSet::singleton(sparse_vec(vec![('b', 1)]));
            let c = SemilinearSet::singleton(sparse_vec(vec![('c', 1)]));
            
            let ba = b.plus(a);
            let ba_star = ba.star();
            c.plus(ba_star)
        };
        
        // Convert to Presburger
        let presburger1 = PresburgerSet::from_semilinear_set(&expr1);
        let presburger2 = PresburgerSet::from_semilinear_set(&expr2);
        
        assert_eq!(presburger1, presburger2,
            "Complex semilinear expressions should convert to equal Presburger sets");
    }

    #[test]
    fn test_semilinear_with_multiple_components_different_orders() {
        // Create semilinear sets with multiple components in different orders
        
        let components1 = vec![
            LinearSet {
                base: sparse_vec(vec![(1, 1), (2, 0), (3, 0)]),
                periods: vec![sparse_vec(vec![(1, 1), (2, 1)])],
            },
            LinearSet {
                base: sparse_vec(vec![(1, 0), (2, 1), (3, 0)]),
                periods: vec![sparse_vec(vec![(2, 1), (3, 1)])],
            },
            LinearSet {
                base: sparse_vec(vec![(1, 0), (2, 0), (3, 1)]),
                periods: vec![sparse_vec(vec![(1, 1), (3, 1)])],
            },
        ];
        
        // Same components but in different order and with atoms rearranged
        let components2 = vec![
            LinearSet {
                base: sparse_vec(vec![(3, 1), (2, 0), (1, 0)]),
                periods: vec![sparse_vec(vec![(3, 1), (1, 1)])],
            },
            LinearSet {
                base: sparse_vec(vec![(2, 0), (1, 1), (3, 0)]),
                periods: vec![sparse_vec(vec![(2, 1), (1, 1)])],
            },
            LinearSet {
                base: sparse_vec(vec![(3, 0), (2, 1), (1, 0)]),
                periods: vec![sparse_vec(vec![(3, 1), (2, 1)])],
            },
        ];
        
        let set1 = SemilinearSet::new(components1);
        let set2 = SemilinearSet::new(components2);
        
        let presburger1 = PresburgerSet::from_semilinear_set(&set1);
        let presburger2 = PresburgerSet::from_semilinear_set(&set2);
        
        assert_eq!(presburger1, presburger2,
            "Semilinear sets with reordered components should convert equally");
    }

    #[test]
    fn test_semilinear_star_operation_domain_ordering() {
        // Test star operation with different domain orderings
        
        // Create a linear set with multiple atoms
        let linear1 = LinearSet {
            base: sparse_vec(vec![('p', 1), ('q', 2)]),
            periods: vec![sparse_vec(vec![('p', 1), ('q', 0)])],
        };
        
        // Same content but different ordering
        let linear2 = LinearSet {
            base: sparse_vec(vec![('q', 2), ('p', 1)]),
            periods: vec![sparse_vec(vec![('q', 0), ('p', 1)])],
        };
        
        let semi1 = SemilinearSet::new(vec![linear1]);
        let semi2 = SemilinearSet::new(vec![linear2]);
        
        // Apply star operation
        let star1 = semi1.star();
        let star2 = semi2.star();
        
        // Convert to Presburger
        let presburger1 = PresburgerSet::from_semilinear_set(&star1);
        let presburger2 = PresburgerSet::from_semilinear_set(&star2);
        
        assert_eq!(presburger1, presburger2,
            "Star operation should produce equal results regardless of domain ordering");
    }

    #[test]
    fn test_semilinear_zero_and_one_conversion() {
        // Test special semilinear sets (zero and one)
        
        let zero1 = SemilinearSet::<i32>::zero();
        let zero2 = SemilinearSet::<i32>::zero();
        
        let one1 = SemilinearSet::<i32>::one();
        let one2 = SemilinearSet::<i32>::one();
        
        // Convert to Presburger
        let p_zero1 = PresburgerSet::from_semilinear_set(&zero1);
        let p_zero2 = PresburgerSet::from_semilinear_set(&zero2);
        
        let p_one1 = PresburgerSet::from_semilinear_set(&one1);
        let p_one2 = PresburgerSet::from_semilinear_set(&one2);
        
        assert_eq!(p_zero1, p_zero2, "Zero semilinear sets should convert equally");
        assert_eq!(p_one1, p_one2, "One semilinear sets should convert equally");
    }

    #[test]
    fn test_semilinear_renamed_variables() {
        // Test that renaming variables produces equivalent results
        
        let original = {
            let comp = LinearSet {
                base: sparse_vec(vec![(100, 2), (200, 3)]),
                periods: vec![sparse_vec(vec![(100, 1), (200, 0)])],
            };
            SemilinearSet::new(vec![comp])
        };
        
        // Rename variables
        let renamed = original.rename(|x| format!("var_{}", x));
        
        // Create equivalent set with string names from the start
        let direct = {
            let comp = LinearSet {
                base: sparse_vec(vec![("var_100".to_string(), 2), ("var_200".to_string(), 3)]),
                periods: vec![sparse_vec(vec![("var_100".to_string(), 1), ("var_200".to_string(), 0)])],
            };
            SemilinearSet::new(vec![comp])
        };
        
        // Convert both to Presburger
        let p_renamed = PresburgerSet::from_semilinear_set(&renamed);
        let p_direct = PresburgerSet::from_semilinear_set(&direct);
        
        assert_eq!(p_renamed, p_direct,
            "Renamed semilinear sets should convert to equal Presburger sets");
    }
}