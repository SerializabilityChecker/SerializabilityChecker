use crate::debug_report::DebugLogger;
use crate::deterministic_map::{HashMap, HashSet};
use crate::kleene::Kleene;
use crate::petri::*;
use crate::proof_parser::ProofInvariant;
use crate::semilinear::*;
use crate::size_logger::{PetriNetSize, log_petri_size_csv};
use crate::spresburger::SPresburgerSet;
use colored::*;
use either::{Either, Left, Right};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::path::Path;
use std::sync::Mutex;

/// Decision enum for reachability analysis results with proof/trace support
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Decision<P: Eq + Hash> {
    CounterExample { trace: Vec<(Vec<P>, Vec<P>)> },
    Proof { proof: Option<ProofInvariant<P>> },
    Timeout { message: String },
}

/// Global debug logger for reachability analysis
static DEBUG_LOGGER: Mutex<Option<DebugLogger>> = Mutex::new(None);

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
) -> Decision<Either<P, Q>>
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
pub fn is_petri_reachability_set_subset_of_semilinear_new<P, Q>(
    petri: Petri<Either<P, Q>>,
    places_that_must_be_zero: &[P],
    semilinear: SemilinearSet<Q>,
    out_dir: &str,
) -> Decision<Either<P, Q>>
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
        let can_reach_decision = can_reach_presburger(petri, end_result_set, out_dir);

        // IMPORTANT: Decision variants are based on the TYPE of evidence, not the answer:
        // - If complement IS reachable: subset property FAILS, we have a counterexample trace → Decision::CounterExample
        // - If complement is NOT reachable: subset property HOLDS, we have a proof → Decision::Proof
        match can_reach_decision {
            Decision::CounterExample { trace } => {
                // Complement is reachable, so subset property does NOT hold
                // We have a trace showing non-serializability
                debug_logger.step(
                    "Final Result",
                    "Subset property FAILS - NOT serializable",
                    &format!("Found counterexample trace: {:?}", trace),
                );
                Decision::CounterExample { trace }
            }
            Decision::Proof { proof } => {
                // Complement is unreachable, so subset property HOLDS
                // We have a proof of serializability
                debug_logger.step(
                    "Final Result",
                    "Subset property HOLDS - IS serializable",
                    &format!("Have proof certificate: {}", proof.is_some()),
                );
                Decision::Proof { proof }
            }
            Decision::Timeout { message } => {
                // Analysis timed out
                debug_logger.step(
                    "Final Result",
                    "Analysis TIMED OUT",
                    &message,
                );
                Decision::Timeout { message }
            }
        }
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
) -> Decision<P>
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

        // Check if ANY disjunct is reachable, collecting proofs along the way
        let mut disjunct_proofs = Vec::new();

        for (i, quantified_set) in disjuncts.iter().enumerate() {
            debug_logger.log_disjunct_start(i, quantified_set);
            println!("Checking disjunct {}: {}", i, quantified_set);
            
            // Record initial petri net size for this disjunct
            let initial_places = petri.get_places().len();
            let initial_transitions = petri.get_transitions().len();
            
            // Start disjunct stats collection
            crate::stats::start_disjunct_analysis(i, initial_places, initial_transitions);

            match can_reach_quantified_set(petri.clone(), quantified_set.clone(), out_dir, i) {
                Decision::CounterExample { trace } => {
                    println!(
                        "Disjunct {} is reachable - constraint set is satisfiable",
                        i
                    );
                    debug_logger.step(
                        &format!("Disjunct {} Result", i),
                        "Disjunct is REACHABLE - constraint set is satisfiable",
                        &format!("Disjunct {}: REACHABLE", i),
                    );
                    return Decision::CounterExample { trace };
                }
                Decision::Proof { proof } => {
                    debug_logger.step(
                        &format!("Disjunct {} Result", i),
                        "Disjunct is UNREACHABLE",
                        &format!("Disjunct {}: UNREACHABLE", i),
                    );
                    if let Some(p) = proof {
                        disjunct_proofs.push(p);
                    }
                }
                Decision::Timeout { message } => {
                    debug_logger.step(
                        &format!("Disjunct {} Result", i),
                        "Analysis TIMED OUT",
                        &format!("Disjunct {}: TIMEOUT - {}", i, message),
                    );
                    return Decision::Timeout { message };
                }
            }
        }

        println!("No disjuncts are reachable - constraint set is unsatisfiable");
        debug_logger.step(
            "All Disjuncts Checked",
            "No disjuncts are reachable - constraint set is unsatisfiable",
            &format!("Checked {} disjuncts, all UNREACHABLE", disjuncts.len()),
        );

        // Combine all disjunct proofs by ANDing them together
        // This handles all cases: empty (And([])), single element (And([x])), and multiple elements
        use crate::proof_parser::Formula;

        // Collect all variables from all proofs
        let mut all_variables = HashSet::default();
        for proof in &disjunct_proofs {
            all_variables.extend(proof.variables.iter().cloned());
        }

        // Create AND of all formulas
        let formulas: Vec<Formula<P>> = disjunct_proofs
            .into_iter()
            .map(|proof| proof.formula)
            .collect();

        let combined_formula = Formula::And(formulas);
        let mut combined_variables: Vec<P> = all_variables.into_iter().collect();
        combined_variables.sort();
        combined_variables.dedup();

        let combined_proof = Some(ProofInvariant::new(
            combined_variables,
            combined_formula,
        ));

        Decision::Proof {
            proof: combined_proof,
        }
    })
}

pub fn can_reach_quantified_set<P>(
    petri: Petri<P>,
    quantified_set: super::presburger::QuantifiedSet<P>,
    out_dir: &str,
    disjunct_id: usize,
) -> Decision<P>
where
    P: Clone + Hash + Ord + Display + Debug,
{
    with_debug_logger(|debug_logger| {
        debug_logger.step(
            &format!("Quantified Set {} Start", disjunct_id),
            "Extracting and reifying existential variables",
            &format!("Quantified set: {}", quantified_set),
        );

        let (existential_places, basic_constraint_set) =
            quantified_set.extract_and_reify_existential_variables();

        // Extract just the usize indices from the Either<usize, P> type
        let existential_indices: Vec<usize> = existential_places
            .iter()
            .filter_map(|place| match place {
                Either::Left(idx) => Some(*idx),
                Either::Right(_) => None,
            })
            .collect();

        debug_logger.step(
            &format!("Quantified Set {} Variables", disjunct_id),
            "Existential variables extracted",
            &format!(
                "Variables: {:?}\nBasic constraint set: {}",
                existential_indices,
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
        for idx in &existential_indices {
            new_petri.add_existential_place(Either::Left(*idx));
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

        // Build mapping from sanitized names to Either<usize, P> for proof conversion
        let mut name_to_place: HashMap<String, Either<usize, P>> = HashMap::default();
        for place in new_petri.get_places() {
            let sanitized_name = crate::utils::string::sanitize(&place.to_string());
            name_to_place.insert(sanitized_name, place);
        }

        // Get the result with Either<usize, P> type
        let result = can_reach_constraint_set_with_debug_mapped(
            new_petri,
            basic_constraint_set,
            out_dir,
            disjunct_id,
            name_to_place,
        );

        // Handle existential quantification for proofs and traces
        match result {
            Decision::CounterExample { trace } => {
                // Transform trace from Either<usize, P> to P by filtering out existential places
                let transformed_trace: Vec<(Vec<P>, Vec<P>)> = trace
                    .into_iter()
                    .map(|(inputs, outputs)| {
                        let transformed_inputs: Vec<P> = inputs
                            .into_iter()
                            .filter_map(|place| match place {
                                Either::Left(_) => None, // Skip existential places
                                Either::Right(p) => Some(p),
                            })
                            .collect();
                        let transformed_outputs: Vec<P> = outputs
                            .into_iter()
                            .filter_map(|place| match place {
                                Either::Left(_) => None, // Skip existential places
                                Either::Right(p) => Some(p),
                            })
                            .collect();
                        (transformed_inputs, transformed_outputs)
                    })
                    .collect();
                Decision::CounterExample {
                    trace: transformed_trace,
                }
            }
            Decision::Proof { proof } => {
                // If we have a proof, we need to existentially quantify and project
                let final_proof = proof.map(|p| {
                    use crate::proofinvariant_to_presburger::{
                        existentially_quantify_keep_either, project_proof_from_either,
                    };

                    // First quantify over the existential variables (indices)
                    let quantified = existentially_quantify_keep_either(p, &existential_indices);

                    // Then project from Either<usize, P> to P
                    project_proof_from_either(quantified)
                });

                Decision::Proof { proof: final_proof }
            }
            Decision::Timeout { message } => {
                Decision::Timeout { message }
            }
        }
    })
}

/// Reachability check with constraints using SMPT with pruning and debug logging
pub fn can_reach_constraint_set_with_debug<P>(
    petri: Petri<P>,
    constraints: Vec<super::presburger::Constraint<P>>,
    out_dir: &str,
    disjunct_id: usize,
) -> Decision<P>
where
    P: Clone + Hash + Ord + Display + Debug,
{
    // Build default mapping (sanitized name maps to itself when P = String)
    let name_to_place = HashMap::default();
    can_reach_constraint_set_with_debug_mapped(
        petri,
        constraints,
        out_dir,
        disjunct_id,
        name_to_place,
    )
}

/// Reachability check with constraints using SMPT with pruning, debug logging, and name mapping
fn can_reach_constraint_set_with_debug_mapped<P>(
    petri: Petri<P>,
    constraints: Vec<super::presburger::Constraint<P>>,
    out_dir: &str,
    disjunct_id: usize,
    name_to_place: HashMap<String, P>,
) -> Decision<P>
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

        debug_logger.log_petri_net(
            &format!("Pre-Pruning Petri Net {}", disjunct_id),
            "Petri net before pruning and optimization",
            &petri,
        );
        let csv_path = Path::new(out_dir).join("petri_size_stats.csv");
        let before = PetriNetSize {
            program_name: Path::new(out_dir)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
            disjunct_id: disjunct_id,
            stage: "pre_pruning",
            num_places: petri.get_places().len(),
            num_transitions: petri.get_transitions().len(),
        };
        log_petri_size_csv(&csv_path, &before).expect("Failed to log Petri‐net size (pre‐pruning)");

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
            .clone()
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

        // Check if optimization is enabled
        if crate::reachability::optimize_enabled() {
            // Use recursive approach with pruning and proof translation
            debug_logger.step(
                &format!("Starting Recursive Pruning {}", disjunct_id),
                "Using recursive approach for pruning with proof translation",
                "",
            );

            can_reach_constraint_set_recursive_with_proof(
                petri,
                constraints,
                &nonzero_places,
                out_dir,
                disjunct_id,
                name_to_place,
                0, // Start at iteration 0
            )
        } else {
            // Optimization disabled, call SMPT directly without pruning
            debug_logger.step(
                &format!("Pruning Skipped {}", disjunct_id),
                "Optimization disabled, calling SMPT directly",
                "",
            );

            let result =
                crate::smpt::can_reach_constraint_set(petri, constraints, out_dir, disjunct_id);
            convert_smpt_result_to_decision(result, &name_to_place)
        }
    })
}

/// Helper function to convert SMPT result to Decision with proof mapping
fn convert_smpt_result_to_decision<P>(
    result: crate::smpt::SmptVerificationResult<P>,
    name_to_place: &HashMap<String, P>,
) -> Decision<P>
where
    P: Clone + Hash + Ord + Display + Debug,
{
    use crate::smpt::SmptVerificationOutcome;

    match result.outcome {
        SmptVerificationOutcome::Reachable { trace } => Decision::CounterExample { trace },
        SmptVerificationOutcome::Unreachable { parsed_proof, .. } => {
            // Convert the proof from String to P using the provided mapping
            let proof = parsed_proof.and_then(|string_proof| {
                if name_to_place.is_empty() {
                    // No mapping provided, can't convert
                    None
                } else {
                    // Use the specialized mapping function to avoid infinite recursion
                    crate::proof_parser::map_proof_variables(string_proof, name_to_place)
                }
            });

            Decision::Proof { proof }
        }
        SmptVerificationOutcome::Error { message } => {
            eprintln!("SMPT verification error: {}", message);
            // Check if this is a timeout error
            if message.contains("timeout") || message.contains("timed out") {
                Decision::Timeout { message }
            } else {
                eprintln!("CRITICAL ERROR: SMPT verification failed: {}", message);
                eprintln!("Cannot determine serializability - analysis is inconclusive");
                eprintln!("This could indicate a bug in the verification pipeline");
                // Log this as an error to the JSONL file before panicking
                crate::stats::set_analysis_result("error");
                crate::stats::finalize_stats();
                panic!("SMPT verification failed: {}", message);
            }
        }
    }
}

/// Recursive reachability check with pruning and proof translation
///
/// This function implements the recursive approach where:
/// 1. Pruning happens top-down (forward->backward on each recursive call)
/// 2. Proof translation happens bottom-up (backward->forward as we return)
fn can_reach_constraint_set_recursive_with_proof<P>(
    mut petri: Petri<P>,
    constraints: Vec<super::presburger::Constraint<P>>,
    target_places: &[P],
    out_dir: &str,
    disjunct_id: usize,
    name_to_place: HashMap<String, P>,
    iteration: usize,
) -> Decision<P>
where
    P: Clone + Hash + Ord + Display + Debug,
{
    with_debug_logger(|debug_logger| {
        debug_logger.step(
            &format!(
                "Recursive Iteration {} for Disjunct {}",
                iteration, disjunct_id
            ),
            "Starting recursive pruning iteration",
            &format!(
                "Petri net has {} transitions",
                petri.get_transitions().len()
            ),
        );

        // Safety check to prevent infinite recursion
        if iteration > 100 {
            eprintln!("WARNING: Pruning recursion exceeded 100 iterations, stopping");
            let result =
                crate::smpt::can_reach_constraint_set(petri, constraints, out_dir, disjunct_id);
            return convert_smpt_result_to_decision(result, &name_to_place);
        }

        // Check if optimization is enabled - if not, go directly to base case
        if !crate::reachability::optimize_enabled() {
            debug_logger.step(
                &format!("Optimization Disabled - Iteration {}", iteration),
                "Optimization is disabled, skipping pruning",
                "",
            );

            let result =
                crate::smpt::can_reach_constraint_set(petri, constraints, out_dir, disjunct_id);

            return convert_smpt_result_to_decision(result, &name_to_place);
        }

        // Get initial marking for forward pruning
        let initial_places = petri.get_initial_marking();

        // Track the number of transitions before pruning
        let transitions_before = petri.get_transitions().len();

        // Attempt one round of pruning
        let removed_forward = petri.filter_reachable(&initial_places);
        let removed_backward = petri.filter_backwards_reachable(target_places);

        // Track the number of transitions after pruning
        let transitions_after = petri.get_transitions().len();
        
        // Record pruning iteration
        crate::stats::record_pruning_iteration();

        debug_logger.step(
            &format!("Pruning Results - Iteration {}", iteration),
            "Completed one round of bidirectional pruning",
            &format!(
                "Transitions: {} -> {} (removed {})\nRemoved {} places forward, {} places backward",
                transitions_before,
                transitions_after,
                transitions_before - transitions_after,
                removed_forward.len(),
                removed_backward.len()
            ),
        );

        // Debug: Print removed places and transition count
        if iteration < 5 || transitions_before != transitions_after {
            eprintln!(
                "Iteration {}: Transitions {} -> {} (removed {})",
                iteration,
                transitions_before,
                transitions_after,
                transitions_before - transitions_after
            );
            if !removed_forward.is_empty() {
                eprint!("{}", "  Forward removed places: ".green());
                for (i, place) in removed_forward.iter().enumerate() {
                    if i > 0 {
                        eprint!(", ");
                    }
                    eprint!("{}", place);
                }
                eprintln!();
            }
            if !removed_backward.is_empty() {
                eprint!("{}", "  Backward removed places: ".green());
                for (i, place) in removed_backward.iter().enumerate() {
                    if i > 0 {
                        eprint!(", ");
                    }
                    eprint!("{}", place);
                }
                eprintln!();
            }
        }

        // Check if we're at base case (no transitions were removed)
        if transitions_before == transitions_after {
            // BASE CASE: No more pruning possible, run SMPT
            debug_logger.step(
                &format!("Base Case Reached - Iteration {}", iteration),
                "No transitions removed (fixed point reached), running SMPT",
                &format!(
                    "Final Petri net has {} transitions",
                    petri.get_transitions().len()
                ),
            );

            debug_logger.log_petri_net(
                &format!("Post-Pruning Petri Net {}", disjunct_id),
                "Petri net after bidirectional filtering",
                &petri,
            ); // :contentReference[oaicite:1]{index=1}

            // **New: record CSV line for post‐pruning size**
            let after = PetriNetSize {
                program_name: Path::new(out_dir)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
                disjunct_id: disjunct_id,
                stage: "post_pruning",
                num_places: petri.get_places().len(),
                num_transitions: petri.get_transitions().len(),
            };

            let csv_path = Path::new(out_dir).join("petri_size_stats.csv");
            log_petri_size_csv(&csv_path, &after).expect("Failed to log Petri‐net size (post‐pruning)");
            
            // Finalize disjunct stats
            crate::stats::finalize_disjunct(after.num_places, after.num_transitions);

            let result =
                crate::smpt::can_reach_constraint_set(petri, constraints, out_dir, disjunct_id);

            return convert_smpt_result_to_decision(result, &name_to_place);
        }

        // RECURSIVE CASE: Some pruning occurred
        debug_logger.step(
            &format!("Recursive Case - Iteration {}", iteration),
            "Pruning occurred, making recursive call",
            "",
        );

        let decision = can_reach_constraint_set_recursive_with_proof(
            petri,
            constraints,
            target_places,
            out_dir,
            disjunct_id,
            name_to_place,
            iteration + 1,
        );

        // Transform the proof or trace on the way back up
        match decision {
            Decision::Proof { proof: Some(mut p) } => {
                use crate::proofinvariant_to_presburger::{eliminate_backward, eliminate_forward};

                debug_logger.step(
                    &format!("Proof Translation - Iteration {}", iteration),
                    "Applying proof eliminations in reverse order",
                    &format!(
                        "Applying {} backward eliminations, {} forward eliminations",
                        removed_backward.len(),
                        removed_forward.len()
                    ),
                );

                // Apply eliminations in REVERSE order of pruning
                if !removed_backward.is_empty() {
                    p = eliminate_backward(&p, &removed_backward);
                }
                if !removed_forward.is_empty() {
                    p = eliminate_forward(&p, &removed_forward);
                }
                Decision::Proof { proof: Some(p) }
            }
            Decision::CounterExample { trace } => {
                debug_logger.step(
                    &format!("Trace Translation - Iteration {}", iteration),
                    "Restoring removed transitions to trace",
                    &format!(
                        "Adding {} backward transitions, {} forward transitions back to trace",
                        removed_backward.len(),
                        removed_forward.len()
                    ),
                );

                // For counterexamples, we need to add back the removed transitions
                // Note: This is a simplified approach - in reality, we might need
                // to carefully reconstruct the trace through the removed transitions
                // For now, we just pass through the trace as-is since it already
                // contains the actual transitions, not indices
                Decision::CounterExample { trace }
            }
            other => other, // Pass through Proof without proof
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
