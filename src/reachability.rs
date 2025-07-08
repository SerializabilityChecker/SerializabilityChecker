use crate::debug_report::DebugLogger;
use crate::deterministic_map::HashSet;
use crate::kleene::Kleene;
use crate::petri::*;
use crate::semilinear::*;
use crate::spresburger::SPresburgerSet;
use colored::Colorize;
use either::{Either, Left, Right};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global debug logger for reachability analysis
static DEBUG_LOGGER: Mutex<Option<DebugLogger>> = Mutex::new(None);

pub static BIDIRECTIONAL_PRUNING_ENABLED: AtomicBool = AtomicBool::new(true);

/// Initialize the global debug logger
pub fn init_debug_logger(program_name: String, program_content: String) {
    let logger = DebugLogger::new(program_name, program_content);
    *DEBUG_LOGGER.lock().unwrap() = Some(logger);
}

/// Get a reference to the global debug logger, or create a default one if not initialized
pub fn get_debug_logger() -> DebugLogger {
    let mut guard = DEBUG_LOGGER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(DebugLogger::new(
            "default".to_string(),
            "No program content".to_string(),
        ));
    }
    guard.as_ref().unwrap().clone()
}

/// Set the optimize flag (called from `main.rs`)
pub fn set_optimize_flag(enabled: bool) {
    BIDIRECTIONAL_PRUNING_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Helper to check whether optimization should run
pub fn optimize_enabled() -> bool {
    BIDIRECTIONAL_PRUNING_ENABLED.load(Ordering::SeqCst)
}

/// Execute a closure with the debug logger
fn with_debug_logger<F, R>(f: F) -> R
where
    F: FnOnce(&DebugLogger) -> R,
{
    let logger = get_debug_logger();
    f(&logger)
}

/// Alias for the new implementation - uses the new SPresburgerSet-based architecture
pub fn is_petri_reachability_set_subset_of_semilinear<P, Q>(
    petri: Petri<Either<P, Q>>,
    places_that_must_be_zero: &[P],
    semilinear: SemilinearSet<Q>,
    out_dir: &str,
) -> bool
where
    P: Clone + Hash + Ord + Display + Debug,
    Q: Clone + Hash + Ord + Display + Debug,
{
    is_petri_reachability_set_subset_of_semilinear_new(
        petri,
        places_that_must_be_zero,
        semilinear,
        out_dir,
    )
}

//=============================================================================
// NEW ARCHITECTURE - REACHABILITY CHECKING WITH SPRESBURGER SETS
//=============================================================================

/// NEW IMPLEMENTATION: Checks if the reachability set of a Petri net is a subset of a semilinear set
/// using the new SPresburgerSet-based architecture.
///
/// GOAL: Check if Reachable(petri) ⊆ semilinear when places_that_must_be_zero = 0
/// APPROACH: Check if ¬semilinear ∩ {places_that_must_be_zero = 0} is reachable
///          If this intersection is reachable, then the subset property is violated
#[must_use]
pub fn is_petri_reachability_set_subset_of_semilinear_new<P, Q>(
    petri: Petri<Either<P, Q>>,
    places_that_must_be_zero: &[P],
    semilinear: SemilinearSet<Q>,
    out_dir: &str,
) -> bool
where
    P: Clone + Hash + Ord + Display + Debug,
    Q: Clone + Hash + Ord + Display + Debug,
{
    with_debug_logger(|debug_logger| {
        debug_logger.step(
            "Reachability Analysis Start",
            "Starting new SPresburgerSet-based reachability analysis",
            &format!(
                "Petri net places: [{}]\nPlaces that must be zero: [{}]\nSemilinear set: {}",
                petri
                    .get_places()
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                places_that_must_be_zero
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                semilinear
            ),
        );

        // Step 1: Convert semilinear set to SPresburgerSet and embed it in Either<P,Q> domain
        let q_spresburger = SPresburgerSet::from_semilinear(semilinear);

        // Step 2: Create universe over places that can vary (filter out places_that_must_be_zero)
        // Since places_that_must_be_zero are constrained to 0, they don't participate in the analysis
        let all_places = petri.get_places();
        let places_that_can_vary: Vec<_> = all_places
            .into_iter()
            .filter(|place| {
                // Keep the place if it's not in places_that_must_be_zero
                match place {
                    Left(p) => !places_that_must_be_zero.contains(p),
                    Right(_) => false, // All Q-places can vary
                }
            })
            .collect();

        let varying_universe = SPresburgerSet::universe(places_that_can_vary);
        debug_logger.step(
            "Varying Universe",
            "Varying universe",
            &format!("Varying universe: {}", varying_universe),
        );

        let response_places = petri
            .get_places()
            .iter()
            .filter_map(|place| match place {
                Right(q) => Some(q.clone()),
                Left(_) => None,
            })
            .collect::<Vec<_>>();

        let response_universe = SPresburgerSet::universe(response_places);
        debug_logger.step(
            "Response Universe",
            "Response universe",
            &format!("Response universe: {}", response_universe),
        );

        // Step 3: Compute complement: universe - embedded_semilinear
        let complement = response_universe.difference(q_spresburger);
        debug_logger.step(
            "Compute Complement",
            "Computing complement (universe - embedded_semilinear)",
            &format!("Complement: {}", complement),
        );

        let complement_embedded = complement.rename(|q| Right(q));
        debug_logger.step(
            "Complement Embedded",
            "Complement embedded in Either<P,Q> domain",
            &format!("Complement embedded: {}", complement_embedded),
        );

        let end_result_set = varying_universe.times(complement_embedded);
        debug_logger.step(
            "End Result Set",
            "End result set",
            &format!("End result set: {}", end_result_set),
        );

        // Step 4: Check if this constraint set is reachable
        // Note: we've effectively incorporated the zero constraints by filtering the universe
        let result = !can_reach_presburger(petri, end_result_set, out_dir);

        debug_logger.step(
            "Final Result",
            "Reachability analysis complete",
            &format!("Subset property holds: {}", result),
        );

        result
    })
}

/// Checks if a Petri net can reach any state satisfying the given SPresburgerSet constraints.
///
/// APPROACH: Convert SPresburgerSet to disjunctive normal form and check each disjunct.
/// A SPresburgerSet represents a union of constraint sets (disjuncts).
/// The Petri net can reach the SPresburgerSet if it can reach ANY of the disjuncts.
pub fn can_reach_presburger<P>(
    petri: Petri<P>,
    mut presburger: SPresburgerSet<P>,
    out_dir: &str,
) -> bool
where
    P: Clone + Hash + Ord + Display + Debug,
{
    with_debug_logger(|debug_logger| {
        debug_logger.step(
            "Presburger Reachability Start",
            "Expanding domain and converting to disjunctive normal form",
            &format!("SPresburgerSet to be checked: {}", presburger),
        );

        // First step: Expand the domain of the presburger set to include all places in the Petri net
        let all_petri_places = petri.get_places();
        debug_logger.step(
            "Domain Expansion",
            "Expanding presburger set domain to match Petri net",
            &format!(
                "Petri net places: [{}]",
                all_petri_places
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        presburger = presburger.expand_domain(all_petri_places);
        debug_logger.step(
            "Domain Expanded",
            "Presburger set domain expanded",
            &format!("Expanded presburger set: {}", presburger),
        );

        // Convert SPresburgerSet to disjunctive normal form (list of quantified sets)
        let disjuncts = presburger.extract_constraint_disjuncts();

        debug_logger.step(
            "Disjunct Conversion",
            "SPresburgerSet converted to disjuncts",
            &format!(
                "Number of disjuncts: {}\nDisjuncts: {}",
                disjuncts.len(),
                disjuncts
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        // Check if ANY disjunct is reachable
        for (i, quantified_set) in disjuncts.iter().enumerate() {
            debug_logger.log_disjunct_start(i, quantified_set);
            println!("Checking disjunct {}: {}", i, quantified_set);

            if can_reach_quantified_set(petri.clone(), quantified_set.clone(), out_dir, i) {
                println!(
                    "Disjunct {} is reachable - constraint set is satisfiable",
                    i
                );
                debug_logger.step(
                    &format!("Disjunct {} Result", i),
                    "Disjunct is REACHABLE - constraint set is satisfiable",
                    &format!("Disjunct {}: REACHABLE", i),
                );
                return true;
            }
            debug_logger.step(
                &format!("Disjunct {} Result", i),
                "Disjunct is UNREACHABLE",
                &format!("Disjunct {}: UNREACHABLE", i),
            );
        }

        println!("No disjuncts are reachable - constraint set is unsatisfiable");
        debug_logger.step(
            "All Disjuncts Checked",
            "No disjuncts are reachable - constraint set is unsatisfiable",
            &format!("Checked {} disjuncts, all UNREACHABLE", disjuncts.len()),
        );
        false
    })
}

/// Check if a Petri net can reach any state satisfying a quantified constraint set.
///
/// This function handles existentially quantified variables by adding them as fresh places
/// to the Petri net and checking reachability of the resulting constraint system.
///
/// # Arguments
/// * `petri` - The Petri net to analyze
/// * `quantified_set` - Set with existentially quantified variables and constraints
/// * `out_dir` - Directory for debug output
/// * `disjunct_id` - Identifier for this disjunct (for debug output)
///
/// # Returns
/// `true` if the Petri net can reach a state satisfying the constraints
pub fn can_reach_quantified_set<P>(
    petri: Petri<P>,
    quantified_set: super::presburger::QuantifiedSet<P>,
    out_dir: &str,
    disjunct_id: usize,
) -> bool
where
    P: Clone + Hash + Ord + Display + Debug,
{
    with_debug_logger(|debug_logger| {
        debug_logger.step(
            &format!("Quantified Set {} Start", disjunct_id),
            "Extracting and reifying existential variables",
            &format!("Quantified set: {}", quantified_set),
        );

        let (variables, basic_constraint_set) =
            quantified_set.extract_and_reify_existential_variables();

        debug_logger.step(
            &format!("Quantified Set {} Variables", disjunct_id),
            "Existential variables extracted",
            &format!(
                "Variables: {:?}\nBasic constraint set: {}",
                variables,
                basic_constraint_set
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        // Transform the Petri net from Petri<P> to Petri<Either<usize, P>>
        // by mapping all existing places to Right(p) and adding existential places as Left(i)
        let mut new_petri = petri.rename(|p| Either::Right(p));
        for place in variables {
            new_petri.add_existential_place(place);
        }

        debug_logger.log_petri_net(
            &format!("Transformed Petri Net {}", disjunct_id),
            "Petri net with existential variables added",
            &new_petri,
        );
        debug_logger.log_constraints(
            &format!("Final Constraints {}", disjunct_id),
            "Final constraints to be checked with SMPT",
            &basic_constraint_set,
        );

        can_reach_constraint_set_with_debug(new_petri, basic_constraint_set, out_dir, disjunct_id)
    })
}

/// Reachability check with constraints using SMPT with pruning and debug logging
/// Check if a Petri net can reach a state satisfying the given constraints.
///
/// This is the core reachability checking function that interfaces with the SMPT
/// verification tool. It converts constraints to SMPT format and analyzes reachability.
///
/// # Arguments
/// * `petri` - The Petri net to analyze
/// * `constraints` - List of linear constraints that must be satisfied
/// * `out_dir` - Directory for debug output and SMPT files
/// * `disjunct_id` - Identifier for this disjunct (for logging)
///
/// # Returns
/// `true` if constraints are reachable (program is NOT serializable)
/// `false` if constraints are unreachable (program IS serializable)
///
/// # Panics
/// Panics if SMPT verification fails, as we cannot safely assume serializability
pub fn can_reach_constraint_set_with_debug<P>(
    mut petri: Petri<P>,
    constraints: Vec<super::presburger::Constraint<P>>,
    out_dir: &str,
    disjunct_id: usize,
) -> bool
where
    P: Clone + Hash + Ord + Display + Debug,
{
    with_debug_logger(|debug_logger| {
        debug_logger.log_petri_net(
            &format!("Pre-Pruning Petri Net {}", disjunct_id),
            "Petri net before pruning and optimization",
            &petri,
        );
        debug_logger.log_constraints(
            &format!("Input Constraints {}", disjunct_id),
            "Constraints for reachability check",
            &constraints,
        );

        // Prune the petri net here by doing the iterative filtering where the target places
        // are all the nonzero variables (i.e. all places in the petri net that are not part
        // of the zero variables)

        // Extract zero variables from constraints
        let zero_variables = super::presburger::Constraint::extract_zero_variables(&constraints);
        let zero_variables_set: HashSet<P> = zero_variables.into_iter().collect();

        debug_logger.step(
            &format!("Zero Variables {}", disjunct_id),
            "Extracted zero variables from constraints",
            &format!("Zero variables: {:?}", zero_variables_set),
        );

        // Get all places in the Petri net
        let all_places = petri.get_places();

        // Find nonzero variables (target places for filtering)
        let nonzero_places: Vec<P> = all_places
            .into_iter()
            .filter(|place| !zero_variables_set.contains(place))
            .collect();

        debug_logger.step(
            &format!("Nonzero Places {}", disjunct_id),
            "Determined nonzero places for bidirectional filtering",
            &format!(
                "Nonzero places: [{}]",
                nonzero_places
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );

        let (removed_forward, removed_backward) =
            petri.filter_bidirectional_reachable(&nonzero_places);

        // Pretty print removed transitions if any were removed
        if !removed_forward.is_empty() || !removed_backward.is_empty() {
            println!("  {} Pruning results:", "✂️".bright_black());
            if !removed_forward.is_empty() {
                println!(
                    "    {} Forward-removed transitions: {}",
                    "➡️".bright_black(),
                    removed_forward.len()
                );
                for (pre, post) in &removed_forward {
                    // Format places using their Display implementation to get pretty names
                    let pre_str = if pre.is_empty() {
                        "∅".to_string()
                    } else {
                        pre.iter()
                            .map(|p| format!("{}", p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let post_str = if post.is_empty() {
                        "∅".to_string()
                    } else {
                        post.iter()
                            .map(|p| format!("{}", p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    println!("      {} {} → {}", "-".red(), pre_str, post_str);
                }
            }
            if !removed_backward.is_empty() {
                println!(
                    "    {} Backward-removed transitions: {}",
                    "⬅️".bright_black(),
                    removed_backward.len()
                );
                for (pre, post) in &removed_backward {
                    // Format places using their Display implementation to get pretty names
                    let pre_str = if pre.is_empty() {
                        "∅".to_string()
                    } else {
                        pre.iter()
                            .map(|p| format!("{}", p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let post_str = if post.is_empty() {
                        "∅".to_string()
                    } else {
                        post.iter()
                            .map(|p| format!("{}", p))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    println!("      {} {} → {}", "-".red(), pre_str, post_str);
                }
            }
        }

        debug_logger.log_petri_net(
            &format!("Post-Pruning Petri Net {}", disjunct_id),
            "Petri net after bidirectional filtering",
            &petri,
        );

        let result =
            crate::smpt::can_reach_constraint_set(petri, constraints, out_dir, disjunct_id);
        match result.outcome {
            crate::smpt::SmptVerificationOutcome::Reachable { .. } => true, // Reachable means not serializable
            crate::smpt::SmptVerificationOutcome::Unreachable { .. } => false, // Unreachable means serializable
            crate::smpt::SmptVerificationOutcome::Error { message } => {
                eprintln!(
                    "CRITICAL ERROR: SMPT verification failed in disjunct {}: {}",
                    disjunct_id, message
                );
                eprintln!("Cannot determine serializability - analysis is inconclusive");
                eprintln!("This could indicate a bug when --without-bidirectional is used");
                // Log this as an error to the JSONL file before panicking
                crate::stats::set_analysis_result("error");
                crate::stats::finalize_stats();
                panic!("SMPT verification failed: {}", message);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presburger::{Constraint, ConstraintType};

    #[test]
    fn test_petri_net_pruning_with_zero_constraints() {
        // Create a Petri net: Start -> A -> B -> C, with unreachable D -> E
        let mut petri = Petri::new(vec!["Start"]);
        petri.add_transition(vec!["Start"], vec!["A"]); // t0: Start -> A (reachable)
        petri.add_transition(vec!["A"], vec!["B"]); // t1: A -> B (reachable)
        petri.add_transition(vec!["B"], vec!["C"]); // t2: B -> C (reachable)
        petri.add_transition(vec!["D"], vec!["E"]); // t3: D -> E (unreachable from Start)
        petri.add_transition(vec!["C"], vec!["F"]); // t4: C -> F (reachable)

        // Before pruning: 5 transitions
        assert_eq!(petri.get_transitions().len(), 5);

        // Create constraints: A = 0, C = 0 (so B and F are nonzero)
        let constraints = vec![
            Constraint::new(vec![(1, "A")], 0, ConstraintType::EqualToZero), // A = 0
            Constraint::new(vec![(1, "C")], 0, ConstraintType::EqualToZero), // C = 0
        ];

        // Extract zero variables
        let zero_vars = Constraint::extract_zero_variables(&constraints);
        assert_eq!(zero_vars.len(), 2);
        assert!(zero_vars.contains(&"A"));
        assert!(zero_vars.contains(&"C"));

        // Get all places and find nonzero places
        let all_places = petri.get_places();
        let zero_vars_set: HashSet<&str> = zero_vars.into_iter().collect();
        let nonzero_places: Vec<&str> = all_places
            .into_iter()
            .filter(|place| !zero_vars_set.contains(place))
            .collect();

        // Nonzero places should be: Start, B, D, E, F
        assert_eq!(nonzero_places.len(), 5);
        assert!(nonzero_places.contains(&"Start"));
        assert!(nonzero_places.contains(&"B"));
        assert!(nonzero_places.contains(&"D"));
        assert!(nonzero_places.contains(&"E"));
        assert!(nonzero_places.contains(&"F"));

        // Apply bidirectional filtering
        let (_removed_forward, _removed_backward) =
            petri.filter_bidirectional_reachable(&nonzero_places);

        // After filtering, should keep only transitions that can reach nonzero places
        // from the initial marking: Start -> A -> B and B -> C -> F
        // t3 (D -> E) should be removed as it's not reachable from Start
        let remaining_transitions = petri.get_transitions();
        assert!(remaining_transitions.len() <= 4); // Should remove at least t3

        // Verify the remaining transitions form a path from Start to nonzero places
        let has_start_to_a = remaining_transitions.contains(&(vec!["Start"], vec!["A"]));
        let has_a_to_b = remaining_transitions.contains(&(vec!["A"], vec!["B"]));
        let _has_b_to_c = remaining_transitions.contains(&(vec!["B"], vec!["C"]));
        let _has_c_to_f = remaining_transitions.contains(&(vec!["C"], vec!["F"]));
        let has_d_to_e = remaining_transitions.contains(&(vec!["D"], vec!["E"]));

        // Should keep transitions that lead to nonzero places (B, F)
        assert!(has_start_to_a); // Needed to reach B
        assert!(has_a_to_b); // Creates nonzero place B

        // Should not keep isolated transition
        assert!(!has_d_to_e); // Not reachable from Start
    }
}
