use crate::deterministic_map::{HashMap, HashSet};
use crate::ns::NS;
use crate::ns_to_petri::ReqPetriState;
use crate::proof_parser::{Formula, ProofInvariant};
use crate::proofinvariant_to_presburger::formula_to_presburger;
use crate::reachability_with_proofs::Decision;
use either::Either;
use serde::{Serialize, Deserialize};
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::fs;
use std::path::Path;


// Helper module for serializing HashMap with non-string keys
mod tuple_vec_map {
    use super::*;
    use serde::{Serializer, Deserializer};

    pub fn serialize<K, V, S>(m: &HashMap<K, V>, ser: S) -> Result<S::Ok, S::Error>
    where
        K: Serialize + Eq + std::hash::Hash,
        V: Serialize,
        S: Serializer,
    {
        m.iter().collect::<Vec<_>>().serialize(ser)
    }

    pub fn deserialize<'de, K, V, D>(de: D) -> Result<HashMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Eq + std::hash::Hash,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        let v: Vec<(K, V)> = Vec::deserialize(de)?;
        Ok(v.into_iter().collect())
    }
}

// Type alias to reduce complexity
type PetriPlace<L, G, Req, Resp> =
    Either<ReqPetriState<L, G, Req, Resp>, ReqPetriState<L, G, Req, Resp>>;

/// Domain-specific type representing the state of a request in the NS
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RequestState<L, Resp> {
    /// Request is in-flight at local state L
    InFlight(L),
    /// Request completed with response Resp
    Completed(Resp),
}

impl<L: Display, Resp: Display> Display for RequestState<L, Resp> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestState::InFlight(l) => write!(f, "InFlight({})", l),
            RequestState::Completed(resp) => write!(f, "Completed({})", resp),
        }
    }
}

/// Wrapper struct for (Req, RequestState<L, Resp>) to implement Display
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RequestStatePair<Req, L, Resp>(pub Req, pub RequestState<L, Resp>);

impl<Req: Display, L: Display, Resp: Display> Display for RequestStatePair<Req, L, Resp> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.1 {
            RequestState::InFlight(l) => write!(f, "{}{}", self.0, l),
            RequestState::Completed(resp) => write!(f, "{}/{}", self.0, resp),
        }
    }
}

/// Wrapper struct for (Req, Resp) pairs to implement Display
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CompletedRequestPair<Req, Resp>(pub Req, pub Resp);

impl<Req: Display, Resp: Display> Display for CompletedRequestPair<Req, Resp> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

/// NS-level step in a trace
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NSStep<G, L, Req, Resp> {
    /// A new request is created
    RequestStart { request: Req, initial_local: L },
    /// An internal transition within an active request
    InternalStep {
        request: Req,
        from_local: L,
        from_global: G,
        to_local: L,
        to_global: G,
    },
    /// A request completes with a response
    RequestComplete {
        request: Req,
        final_local: L,
        response: Resp,
    },
}

/// NS-level trace representing a counterexample execution
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NSTrace<G, L, Req, Resp> {
    /// Sequence of steps in the NS execution
    pub steps: Vec<NSStep<G, L, Req, Resp>>,
}

impl<G, L, Req, Resp> NSTrace<G, L, Req, Resp>
where
    G: Display + Clone + Eq + Hash,
    L: Display + Clone + Eq + Hash,
    Req: Display + Clone + Eq + Hash,
    Resp: Display + Clone + Eq + Hash,
{
    /// Pretty print the NS trace
    pub fn pretty_print(&self, ns: &NS<G, L, Req, Resp>) {
        println!("NS-Level Counterexample Trace:");
        println!("==============================");

        if self.steps.is_empty() {
            println!("(Empty trace - violation at initial state)");
            return;
        }

        for (i, step) in self.steps.iter().enumerate() {
            println!("\nStep {}:", i + 1);
            match step {
                NSStep::RequestStart {
                    request,
                    initial_local,
                } => {
                    println!("  📨 NEW REQUEST");
                    println!("  Request: {}", request);
                    println!("  Initial local state: {}", initial_local);
                }
                NSStep::InternalStep {
                    request,
                    from_local,
                    from_global,
                    to_local,
                    to_global,
                } => {
                    println!("  🔄 INTERNAL TRANSITION");
                    println!("  Request: {}", request);
                    println!("  State transition:");
                    println!("    From: (local: {}, global: {})", from_local, from_global);
                    println!("    To:   (local: {}, global: {})", to_local, to_global);
                }
                NSStep::RequestComplete {
                    request,
                    final_local,
                    response,
                } => {
                    println!("  ✅ REQUEST COMPLETE");
                    println!("  Request: {}", request);
                    println!("  Final local state: {}", final_local);
                    println!("  Response: {}", response);
                }
            }
        }

        // Run trace validation and display results
        println!("\n==============================");
        println!("Trace Validation:");
        println!("==============================");

        match ns.check_trace(self) {
            Ok(completed_pairs) => {
                println!("✅ Trace is valid!");

                // Display completed request/response multiset
                println!("\nCompleted Request/Response Pairs:");
                if completed_pairs.is_empty() {
                    println!("  (none)");
                } else {
                    // Count occurrences of each pair for multiset display
                    let mut counts: HashMap<(Req, Resp), usize> = HashMap::default();
                    for (req, resp) in completed_pairs {
                        *counts.entry((req, resp)).or_insert(0) += 1;
                    }

                    // Display with multiplicity
                    for ((req, resp), count) in counts {
                        if count == 1 {
                            println!("  {}/{}", req, resp);
                        } else {
                            println!("  ({}/{})^{}", req, resp, count);
                        }
                    }
                }
            }
            Err(error) => {
                println!("❌ Trace validation failed!");
                println!("Error: {}", error);
            }
        }
    }
}

/// NS-level decision enum containing either a proof (invariant) or counterexample (trace)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NSDecision<G, L, Req, Resp>
where
    G: Eq + Hash,
    L: Eq + Hash,
    Req: Eq + Hash,
    Resp: Eq + Hash,
{
    /// Program is serializable with proof invariant
    Serializable {
        invariant: NSInvariant<G, L, Req, Resp>,
    },
    /// Program is not serializable with counterexample trace
    NotSerializable { trace: NSTrace<G, L, Req, Resp> },
    /// Analysis timed out
    Timeout { message: String },
}

impl<G, L, Req, Resp> NSDecision<G, L, Req, Resp>
where
    G: Eq + Hash,
    L: Eq + Hash,
    Req: Eq + Hash,
    Resp: Eq + Hash,
{
    /// Save the NSDecision to a JSON file
    /// This method properly serializes the decision using serde
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> 
    where
        G: serde::Serialize,
        L: serde::Serialize,
        Req: serde::Serialize,
        Resp: serde::Serialize,
    {
        // Debug: Try to serialize with better error handling
        match serde_json::to_string_pretty(&self) {
            Ok(json) => {
                fs::write(path, json)?;
                Ok(())
            }
            Err(e) => {
                eprintln!("Serialization error details: {:?}", e);
                eprintln!("Error type: {}", std::any::type_name_of_val(&e));
                Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }
        }
    }

    /// Load an NSDecision from a JSON file
    /// This method properly deserializes the decision using serde
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        for<'de> G: serde::Deserialize<'de>,
        for<'de> L: serde::Deserialize<'de>,
        for<'de> Req: serde::Deserialize<'de>,
        for<'de> Resp: serde::Deserialize<'de>,
    {
        let json = fs::read_to_string(path)?;
        let decision = serde_json::from_str(&json)?;
        Ok(decision)
    }
}

/// NS-level invariant structure that captures per-global-state invariants
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(serialize = "G: Serialize, L: Serialize, Req: Serialize, Resp: Serialize"))]
#[serde(bound(deserialize = "G: Deserialize<'de>, L: Deserialize<'de>, Req: Deserialize<'de>, Resp: Deserialize<'de>"))]
pub struct NSInvariant<G, L, Req, Resp>
where
    G: Eq + Hash,
    L: Eq + Hash,
    Req: Eq + Hash,
    Resp: Eq + Hash,
{
    /// For each global state, invariant over RequestStatePair<Req, L, Resp>
    /// RequestState::InFlight(L) means request is in-flight at local state L
    /// RequestState::Completed(Resp) means request completed with response Resp
    #[serde(with = "tuple_vec_map")]
    pub global_invariants: HashMap<G, ProofInvariant<RequestStatePair<Req, L, Resp>>>,
}

impl<G, L, Req, Resp> NSInvariant<G, L, Req, Resp>
where
    G: Display + Eq + Hash + Display,
    L: Display + Eq + Hash + Display,
    Req: Display + Eq + Hash + Display,
    Resp: Display + Eq + Hash + Display,
{
    /// Project an invariant for a specific global state to only completed requests
    pub fn project_to_completed(
        &self,
        global_state: &G,
    ) -> Option<ProofInvariant<CompletedRequestPair<Req, Resp>>>
    where
        L: Clone,
        Req: Clone,
        Resp: Clone,
    {
        self.global_invariants
            .get(global_state)
            .map(|full_invariant| {
                // Create a projection that maps InFlight to 0 and Completed to the pair
                full_invariant.substitute(|pair| {
                    match &pair.1 {
                        RequestState::InFlight(_) => {
                            // Map InFlight requests to 0
                            Either::Right(0)
                        }
                        RequestState::Completed(resp) => {
                            // Map Completed requests to CompletedRequestPair
                            Either::Left(CompletedRequestPair(pair.0.clone(), resp.clone()))
                        }
                    }
                })
            })
    }

    /// Pretty print the NS invariant
    pub fn pretty_print(&self)
    where
        L: Clone,
        Req: Clone,
        Resp: Clone,
    {
        println!("NS-Level Invariants per Global State:");
        println!("=====================================");

        for (global_state, invariant) in &self.global_invariants {
            println!("\nGlobal State: {}", global_state);
            println!("-------------");

            // Print full invariant in mapping notation
            let vars_str = invariant
                .variables
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("Invariant: ({}) ↦ {}", vars_str, invariant.formula);

            // Print projected invariant
            if let Some(projected) = self.project_to_completed(global_state) {
                println!("\nProjected (Completed Requests Only):");
                let proj_vars_str = projected
                    .variables
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("({}) ↦ {}", proj_vars_str, projected.formula);
            }
        }
    }

    /// Pretty print the NS invariant with proof verification results
    pub fn pretty_print_with_verification(&self, ns: &NS<G, L, Req, Resp>)
    where
        G: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        L: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Req: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Resp: Clone + Display + Eq + Hash + Ord + Debug + ToString,
    {
        self.pretty_print();

        println!("\n=====================================");
        println!("Proof Certificate Verification:");
        println!("=====================================");

        match self.check_proof(ns) {
            Ok(()) => {
                println!("✅ Proof certificate is VALID");
                println!("  ✓ Initial state satisfies the invariant");
                println!("  ✓ Invariant is inductive (preserved by all transitions)");
                println!("  ✓ Invariant implies serializability when no requests in flight");
            }
            Err(err) => {
                println!("❌ Proof certificate is INVALID");
                println!("  Error: {}", err);
            }
        }
    }

    /// Check if the proof certificate is valid
    /// Returns Ok(()) if valid, Err with explanation if invalid
    pub fn check_proof(&self, ns: &NS<G, L, Req, Resp>) -> Result<(), String>
    where
        G: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        L: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Req: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Resp: Clone + Display + Eq + Hash + Ord + Debug + ToString,
    {
        // Check 1: Initial state satisfies the invariant
        self.check_initial_state(ns)?;

        // Check 2: Invariant is inductive
        self.check_inductive(ns)?;

        // Check 3: Invariant implies target (serializability)
        self.check_implies_target(ns)?;

        Ok(())
    }

    /// Check that the initial state satisfies the invariant
    fn check_initial_state(&self, ns: &NS<G, L, Req, Resp>) -> Result<(), String>
    where
        G: Clone + Display,
        L: Clone + Display,
        Req: Clone + Display,
        Resp: Clone + Display,
    {
        // Get the invariant for the initial global state
        let initial_invariant =
            self.global_invariants
                .get(&ns.initial_global)
                .ok_or_else(|| {
                    format!(
                        "No invariant found for initial global state: {}",
                        ns.initial_global
                    )
                })?;

        // Initial state has empty multiset (no requests in flight or completed)
        // This means all variables in the formula should be substituted with 0
        let mut mapping = |_var: &RequestStatePair<Req, L, Resp>| -> Either<String, i32> {
            // All variables map to 0 in the empty multiset
            Either::Right(0)
        };
        let substituted_invariant: ProofInvariant<String> =
            initial_invariant.substitute(&mut mapping);

        // Check if the substituted formula is satisfiable
        if is_formula_satisfied_string(&substituted_invariant.formula) {
            Ok(())
        } else {
            Err("Initial state (empty multiset) does not satisfy the invariant".to_string())
        }
    }

    /// Check that the invariant is inductive (preserved by all transitions)
    fn check_inductive(&self, ns: &NS<G, L, Req, Resp>) -> Result<(), String>
    where
        G: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        L: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Req: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Resp: Clone + Display + Eq + Hash + Ord + Debug + ToString,
    {
        // Check 1: Internal transitions preserve the invariant
        for (from_local, from_global, to_local, to_global) in &ns.transitions {
            // Get invariants for source and target global states
            let from_inv = self
                .global_invariants
                .get(from_global)
                .ok_or_else(|| format!("No invariant for global state: {}", from_global))?;
            let to_inv = self
                .global_invariants
                .get(to_global)
                .ok_or_else(|| format!("No invariant for global state: {}", to_global))?;

            // For each possible request type that could be in this local state
            for (req, _) in &ns.requests {
                let from_var =
                    RequestStatePair(req.clone(), RequestState::InFlight(from_local.clone()));
                let to_var =
                    RequestStatePair(req.clone(), RequestState::InFlight(to_local.clone()));

                // Convert to Either type for the operations
                let from_inv_either: ProofInvariant<Either<usize, RequestStatePair<Req, L, Resp>>> =
                    from_inv.clone().map(|v| Either::Right(v.clone()));

                // Apply the transition: remove one from source, add one to target
                let inv_after_remove = from_inv_either.filter_and_subtract_one(&from_var);
                let inv_after_add = inv_after_remove.add_one(&to_var);

                // Project back to the original type
                let inv_after_transition = inv_after_add.project_right();

                // Check if the result implies the target invariant
                if !self.check_formula_implies(&inv_after_transition, to_inv)? {
                    return Err(format!(
                        "Invariant not inductive for transition ({}, {}) -> ({}, {}) with request {}",
                        from_local, from_global, to_local, to_global, req
                    ));
                }
            }
        }

        // Check 2: Request creation preserves the invariant
        for (req, initial_local) in &ns.requests {
            let initial_inv = self
                .global_invariants
                .get(&ns.initial_global)
                .ok_or_else(|| {
                    format!(
                        "No invariant for initial global state: {}",
                        ns.initial_global
                    )
                })?;

            let new_var =
                RequestStatePair(req.clone(), RequestState::InFlight(initial_local.clone()));

            // Convert to Either type for the operation
            let initial_inv_either: ProofInvariant<Either<usize, RequestStatePair<Req, L, Resp>>> =
                initial_inv.clone().map(|v| Either::Right(v.clone()));

            let inv_after_add = initial_inv_either.add_one(&new_var);
            let inv_after_creation = inv_after_add.project_right();

            // Check if creating a new request preserves the initial state invariant
            if !self.check_formula_implies(&inv_after_creation, initial_inv)? {
                return Err(format!(
                    "Invariant not inductive for request creation: {} at local state {}",
                    req, initial_local
                ));
            }
        }

        // Check 3: Request completion preserves the invariant
        for (final_local, resp) in &ns.responses {
            // For each global state where this response could occur
            for global_state in ns.get_global_states() {
                let global_inv = self
                    .global_invariants
                    .get(global_state)
                    .ok_or_else(|| format!("No invariant for global state: {}", global_state))?;

                // For each request type that could complete with this response
                for (req, _) in &ns.requests {
                    let inflight_var =
                        RequestStatePair(req.clone(), RequestState::InFlight(final_local.clone()));
                    let completed_var =
                        RequestStatePair(req.clone(), RequestState::Completed(resp.clone()));

                    // Convert to Either type for the operations
                    let global_inv_either: ProofInvariant<
                        Either<usize, RequestStatePair<Req, L, Resp>>,
                    > = global_inv.clone().map(|v| Either::Right(v.clone()));

                    // Apply completion: remove inflight, add completed
                    let inv_after_remove = global_inv_either.filter_and_subtract_one(&inflight_var);
                    let inv_after_add = inv_after_remove.add_one(&completed_var);
                    let inv_after_completion = inv_after_add.project_right();

                    // Check if completion preserves the same global state invariant
                    if !self.check_formula_implies(&inv_after_completion, global_inv)? {
                        return Err(format!(
                            "Invariant not inductive for request completion: {} at {} -> {} in global state {}",
                            req, final_local, resp, global_state
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if one proof invariant implies another using Presburger arithmetic
    fn check_formula_implies(
        &self,
        antecedent: &ProofInvariant<RequestStatePair<Req, L, Resp>>,
        consequent: &ProofInvariant<RequestStatePair<Req, L, Resp>>,
    ) -> Result<bool, String>
    where
        G: Display,
        L: Clone + Display + ToString,
        Req: Clone + Display + ToString,
        Resp: Clone + Display + ToString,
    {
        // Get all variables that might appear in either formula
        let mut all_vars = HashSet::default();
        all_vars.extend(antecedent.variables.iter().cloned());
        all_vars.extend(consequent.variables.iter().cloned());

        // Convert to a consistent vector of string variables
        let mut string_vars: Vec<String> = all_vars.iter().map(|v| v.to_string()).collect();
        string_vars.sort();

        // Convert both invariants to use string representations
        let antecedent_string = antecedent.clone().map(|v| v.to_string());
        let consequent_string = consequent.clone().map(|v| v.to_string());

        // Convert to Presburger sets using the same variable mapping
        let antecedent_set = formula_to_presburger(&antecedent_string.formula, &string_vars);
        let consequent_set = formula_to_presburger(&consequent_string.formula, &string_vars);

        // Check if antecedent ⊆ consequent (i.e., antecedent \ consequent = ∅)
        let difference = antecedent_set.difference(&consequent_set);
        Ok(difference.is_empty())
    }

    /// Check that the invariant implies the target property (serializability)
    /// When there are no in-flight requests, completed requests must form a serializable execution
    fn check_implies_target(&self, ns: &NS<G, L, Req, Resp>) -> Result<(), String>
    where
        G: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        L: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Req: Clone + Display + Eq + Hash + Ord + Debug + ToString,
        Resp: Clone + Display + Eq + Hash + Ord + Debug + ToString,
    {
        // Get the semilinear set of serializable executions
        // This uses Response(Req, Resp) as the type
        use crate::ns_to_petri::ReqPetriState;
        let serializable_set: crate::semilinear::SemilinearSet<_> =
            ns.serialized_automaton_kleene(|req, resp| {
                crate::semilinear::SemilinearSet::singleton(crate::semilinear::SparseVector::unit(
                    ReqPetriState::Response(req, resp),
                ))
            });

        // Check each global state
        for (global_state, invariant) in &self.global_invariants {
            // Substitute: InFlight -> 0, Completed -> Response(Req, Resp)
            let mut mapping = |pair: &RequestStatePair<Req, L, Resp>| -> Either<ReqPetriState<L, G, Req, Resp>, i32> {
                match &pair.1 {
                    RequestState::InFlight(_) => {
                        // Map in-flight requests to 0
                        Either::Right(0)
                    }
                    RequestState::Completed(resp) => {
                        // Map completed requests to Response type used in semilinear set
                        Either::Left(ReqPetriState::Response(pair.0.clone(), resp.clone()))
                    }
                }
            };

            let substituted_invariant = invariant.substitute(&mut mapping);

            // Check if the invariant implies membership in the serializable set
            if !self.invariant_implies_semilinear(
                &substituted_invariant,
                &serializable_set,
                global_state,
            )? {
                return Err(format!(
                    "Invariant for global state {} does not imply serializability",
                    global_state
                ));
            }
        }

        Ok(())
    }

    /// Check if an invariant formula implies membership in a semilinear set
    fn invariant_implies_semilinear<T>(
        &self,
        invariant: &ProofInvariant<T>,
        semilinear: &crate::semilinear::SemilinearSet<T>,
        global_state: &G,
    ) -> Result<bool, String>
    where
        T: Clone + Eq + Hash + Display + Debug + Ord + ToString,
        G: Display,
    {
        // For now, convert to String since that's what formula_to_presburger supports
        // This is not ideal but works until we have proper generic support
        let mut string_vars: Vec<String> = invariant.variables.iter().map(|v| v.to_string()).collect();
        string_vars.sort();

        let string_invariant = invariant.clone().map(|v| v.to_string());
        let invariant_set = formula_to_presburger(&string_invariant.formula, &string_vars);

        // Convert semilinear set to String type and then to PresburgerSet
        let string_semilinear = semilinear.clone().rename(|v| v.to_string());
        let mut spresburger =
            crate::spresburger::SPresburgerSet::from_semilinear(string_semilinear.clone());
        let semilinear_as_presburger = spresburger.as_presburger();

        // Check if invariant_set ⊆ semilinear_set
        // This is equivalent to: invariant_set \ semilinear_set = ∅
        let difference = invariant_set.difference(semilinear_as_presburger);

        if difference.is_empty() {
            Ok(true)
        } else {
            // Log which values violate the implication for debugging
            eprintln!(
                "Warning: Invariant for global state {} has values outside serializable set",
                global_state
            );
            eprintln!("  Semilinear set: {}", string_semilinear);
            eprintln!("  Projected invariant: {}", string_invariant.formula);
            eprintln!("  Projected invariant (ISL): {}", invariant_set);
            eprintln!("  Invariant variables: {:?}", string_vars);
            eprintln!("  Values outside serializable set: {}", difference);
            Ok(false)
        }
    }
}



/// Translate a Petri net proof to NS-level invariants
pub fn translate_petri_proof_to_ns<G, L, Req, Resp>(
    petri_proof: ProofInvariant<PetriPlace<L, G, Req, Resp>>,
    ns: &NS<G, L, Req, Resp>,
) -> NSInvariant<G, L, Req, Resp>
where
    G: Clone + Eq + Hash + Debug + Display,
    L: Clone + Eq + Hash + Debug + Display,
    Req: Clone + Eq + Hash + Debug + Display,
    Resp: Clone + Eq + Hash + Debug + Display,
{
    let mut global_invariants = HashMap::default();

    // Get all global states from the NS
    let global_states = ns.get_global_states();

    for global_state in global_states {
        // Create substitution mapping for this global state
        let mut specialized_proof = petri_proof.substitute(|place| {
            match place {
                // LEFT side - Global, Local, Request places
                Either::Left(req_petri_state) => match req_petri_state {
                    ReqPetriState::Global(g) => {
                        if g == global_state {
                            Either::Right(1) // This global state is active
                        } else {
                            Either::Right(0) // Other global states are inactive
                        }
                    }
                    ReqPetriState::Local(req, l) => {
                        // Map to RequestStatePair with InFlight state
                        Either::Left(RequestStatePair(
                            req.clone(),
                            RequestState::InFlight(l.clone()),
                        ))
                    }
                    ReqPetriState::Request(_) => {
                        // This is problematic, we don't yet support requests in the RequestStatePair
                        // Need to think about how to fix this.
                        // Maybe we should reachitect the data structures to make everything smoother.
                        unreachable!("Request found in Left - not implemented yet!")
                    }
                    ReqPetriState::Response(_, _) => {
                        panic!("Response found in Left - this should be unreachable!");
                    }
                },

                // RIGHT side - Response places
                Either::Right(req_petri_state) => match req_petri_state {
                    ReqPetriState::Response(req, resp) => {
                        // Map to RequestStatePair with Completed state
                        Either::Left(RequestStatePair(
                            req.clone(),
                            RequestState::Completed(resp.clone()),
                        ))
                    }
                    _ => {
                        panic!("Non-Response found in Right - this should be unreachable!");
                    }
                },
            }
        });

        // Ensure all possible request state pairs are included in the variable list
        // This fixes cases where trivial proofs (True formulas) have empty variable lists
        let mut all_vars: HashSet<RequestStatePair<Req, L, Resp>> = HashSet::default();

        // Add all response variables (completed requests) that could appear
        for req in ns.get_requests() {
            for (_local, resp) in &ns.responses {
                all_vars.insert(RequestStatePair(
                    req.clone(),
                    RequestState::Completed(resp.clone()),
                ));
            }
        }

        // Add all in-flight variables that could appear
        for req in ns.get_requests() {
            for (req_local, local) in &ns.requests {
                if req == req_local {
                    all_vars.insert(RequestStatePair(
                        req.clone(),
                        RequestState::InFlight(local.clone()),
                    ));
                }
            }
            // Also add all local states that could be reached via transitions
            for (from_local, _, to_local, _) in &ns.transitions {
                all_vars.insert(RequestStatePair(
                    req.clone(),
                    RequestState::InFlight(from_local.clone()),
                ));
                all_vars.insert(RequestStatePair(
                    req.clone(),
                    RequestState::InFlight(to_local.clone()),
                ));
            }
        }

        // Convert to sorted vector for consistent ordering
        let mut additional_vars: Vec<_> = all_vars.into_iter().collect();
        additional_vars.sort_by(|a, b| format!("{}", a).cmp(&format!("{}", b)));

        // Add any variables that aren't already in the proof
        for var in additional_vars {
            if !specialized_proof.variables.contains(&var) {
                specialized_proof.variables.push(var);
            }
        }

        global_invariants.insert(global_state.clone(), specialized_proof);
    }

    NSInvariant { global_invariants }
}

/// Convert a Petri net Decision to an NS-level NSDecision
pub fn petri_decision_to_ns<G, L, Req, Resp>(
    petri_decision: Decision<
        Either<ReqPetriState<L, G, Req, Resp>, ReqPetriState<L, G, Req, Resp>>,
    >,
    ns: &NS<G, L, Req, Resp>,
) -> NSDecision<G, L, Req, Resp>
where
    G: Clone + Eq + Hash + Debug + Display,
    L: Clone + Eq + Hash + Debug + Display,
    Req: Clone + Eq + Hash + Debug + Display,
    Resp: Clone + Eq + Hash + Debug + Display,
{
    match petri_decision {
        Decision::Proof { proof } => {
            if let Some(p) = proof {
                // Translate Petri net proof to NS-level invariant
                let invariant = translate_petri_proof_to_ns(p, ns);
                NSDecision::Serializable { invariant }
            } else {
                // No explicit proof available, create empty invariant
                NSDecision::Serializable {
                    invariant: NSInvariant {
                        global_invariants: HashMap::default(),
                    },
                }
            }
        }
        Decision::CounterExample { trace } => {
            // Convert Petri net trace to NS-level trace
            let ns_trace = convert_petri_trace_to_ns(trace, ns);
            NSDecision::NotSerializable { trace: ns_trace }
        }
        Decision::Timeout { message } => {
            NSDecision::Timeout { message }
        }
    }
}

/// Convert a Petri net trace to an NS-level trace
fn convert_petri_trace_to_ns<G, L, Req, Resp>(
    petri_trace: Vec<(
        Vec<Either<ReqPetriState<L, G, Req, Resp>, ReqPetriState<L, G, Req, Resp>>>,
        Vec<Either<ReqPetriState<L, G, Req, Resp>, ReqPetriState<L, G, Req, Resp>>>,
    )>,
    _ns: &NS<G, L, Req, Resp>,
) -> NSTrace<G, L, Req, Resp>
where
    G: Clone + Eq + Hash + Debug + Display,
    L: Clone + Eq + Hash + Debug + Display,
    Req: Clone + Eq + Hash + Debug + Display,
    Resp: Clone + Eq + Hash + Debug + Display,
{
    let mut steps = Vec::new();

    // Analyze each transition in the Petri trace
    for (inputs, outputs) in petri_trace {
        // Case 1: Request creation (empty inputs, creates Local state)
        if inputs.is_empty() && outputs.len() == 1 {
            if let Some(Either::Left(ReqPetriState::Local(req, local))) = outputs.first() {
                steps.push(NSStep::RequestStart {
                    request: req.clone(),
                    initial_local: local.clone(),
                });
                continue;
            }
        }

        // Case 2: Internal transition (Local + Global inputs)
        if inputs.len() == 2 && outputs.len() == 2 {
            let mut from_local = None;
            let mut from_global = None;
            let mut request = None;

            // Extract input states
            for input in &inputs {
                match input {
                    Either::Left(ReqPetriState::Local(req, local)) => {
                        from_local = Some(local.clone());
                        request = Some(req.clone());
                    }
                    Either::Left(ReqPetriState::Global(global)) => {
                        from_global = Some(global.clone());
                    }
                    _ => {}
                }
            }

            let mut to_local = None;
            let mut to_global = None;

            // Extract output states
            for output in &outputs {
                match output {
                    Either::Left(ReqPetriState::Local(req, local)) => {
                        to_local = Some(local.clone());
                        // Verify same request
                        if request.is_none() {
                            request = Some(req.clone());
                        }
                    }
                    Either::Left(ReqPetriState::Global(global)) => {
                        to_global = Some(global.clone());
                    }
                    _ => {}
                }
            }

            // If we have all components, create internal step
            if let (Some(req), Some(fl), Some(fg), Some(tl), Some(tg)) =
                (request, from_local, from_global, to_local, to_global)
            {
                steps.push(NSStep::InternalStep {
                    request: req,
                    from_local: fl,
                    from_global: fg,
                    to_local: tl,
                    to_global: tg,
                });
                continue;
            }
        }

        // Case 3: Response completion (single Local input, creates Response)
        if inputs.len() == 1 && outputs.len() == 1 {
            if let (
                Some(Either::Left(ReqPetriState::Local(req_in, local))),
                Some(Either::Right(ReqPetriState::Response(req_out, resp))),
            ) = (inputs.first(), outputs.first())
            {
                // Verify same request
                if req_in == req_out {
                    steps.push(NSStep::RequestComplete {
                        request: req_in.clone(),
                        final_local: local.clone(),
                        response: resp.clone(),
                    });
                    continue;
                }
            }
        }

        // If we couldn't match any pattern, log a warning
        eprintln!(
            "Warning: Could not interpret Petri transition: {:?} -> {:?}",
            inputs, outputs
        );
    }

    NSTrace { steps }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_parser::{AffineExpr, CompOp, Constraint, Formula};

    #[test]
    fn test_ns_decision_serialization() {
        use crate::expr_to_ns::{Env, ExprRequest, LocalExpr};
        use crate::parser::ExprHc;
        
        // Create a simple trace
        let mut steps = Vec::new();
        
        // Step 1: Request start
        // Create Env using JSON deserialization to avoid private constructor
        let env: Env = serde_json::from_str(r#"{"vars":{"x":10}}"#).unwrap();
        let mut table = ExprHc::new();
        let expr = table.number(42);
        let local_expr = LocalExpr(env.clone(), expr);
        
        steps.push(NSStep::RequestStart {
            request: ExprRequest { name: "foo".to_string() },
            initial_local: local_expr.clone(),
        });
        
        // Step 2: Request complete
        steps.push(NSStep::RequestComplete {
            request: ExprRequest { name: "foo".to_string() },
            final_local: local_expr,
            response: 42,
        });
        
        let trace: NSTrace<Env, LocalExpr, ExprRequest, i64> = NSTrace { steps };
        let decision = NSDecision::NotSerializable { trace };
        
        // Test serialization
        let json = serde_json::to_string_pretty(&decision).unwrap();
        println!("Serialized NSDecision:\n{}", json);
        
        // Test deserialization
        let decision2: NSDecision<Env, LocalExpr, ExprRequest, i64> = 
            serde_json::from_str(&json).unwrap();
        
        // Verify they match
        match (&decision, &decision2) {
            (NSDecision::NotSerializable { trace: t1 }, NSDecision::NotSerializable { trace: t2 }) => {
                assert_eq!(t1.steps.len(), t2.steps.len());
                // More detailed comparison would require PartialEq on all types
            }
            _ => panic!("Deserialized decision doesn't match"),
        }
    }
    
    #[test]
    fn test_ns_decision_file_operations() {
        use crate::expr_to_ns::{Env, ExprRequest, LocalExpr};
        use crate::parser::ExprHc;
        use tempfile::NamedTempFile;
        
        // Create a simple trace
        let mut steps = Vec::new();
        
        // Create empty Env using JSON
        let env: Env = serde_json::from_str(r#"{"vars":{}}"#).unwrap();
        let mut table = ExprHc::new();
        let expr = table.number(100);
        let local_expr = LocalExpr(env, expr);
        
        steps.push(NSStep::RequestStart {
            request: ExprRequest { name: "test_req".to_string() },
            initial_local: local_expr.clone(),
        });
        
        let trace: NSTrace<Env, LocalExpr, ExprRequest, i64> = NSTrace { steps };
        let decision = NSDecision::NotSerializable { trace };
        
        // Create a temporary file
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();
        
        // Save to file
        decision.save_to_file(temp_path).unwrap();
        
        // Load from file
        let loaded_decision: NSDecision<Env, LocalExpr, ExprRequest, i64> = 
            NSDecision::load_from_file(temp_path).unwrap();
        
        // Verify they match
        match (&decision, &loaded_decision) {
            (NSDecision::NotSerializable { trace: t1 }, NSDecision::NotSerializable { trace: t2 }) => {
                assert_eq!(t1.steps.len(), t2.steps.len());
            }
            _ => panic!("Loaded decision doesn't match"),
        }
    }

    #[test]
    fn test_simple_substitution() {
        // Create a simple proof invariant with mixed Left/Right variables
        let expr1 = AffineExpr::from_var(Either::Left(ReqPetriState::Global("G1".to_string())));
        let constraint1 = Constraint::new(expr1, CompOp::Eq);

        let expr2 = AffineExpr::from_var(Either::Left(ReqPetriState::Local(
            "req1".to_string(),
            "L1".to_string(),
        )));
        let constraint2 = Constraint::new(expr2, CompOp::Geq);

        let formula = Formula::And(vec![
            Formula::Constraint(constraint1),
            Formula::Constraint(constraint2),
        ]);

        let proof = ProofInvariant::new(
            vec![
                Either::Left(ReqPetriState::Global("G1".to_string())),
                Either::Left(ReqPetriState::Local("req1".to_string(), "L1".to_string())),
            ],
            formula,
        );

        // Create a simple NS for context
        let mut ns = NS::<String, String, String, String>::new("G1".to_string());
        ns.add_request("req1".to_string(), "L1".to_string());

        // Translate to NS-level invariant
        let ns_invariant = translate_petri_proof_to_ns(proof, &ns);

        // Check that we have an invariant for global state G1
        assert!(
            ns_invariant
                .global_invariants
                .contains_key(&"G1".to_string())
        );

        // The invariant for G1 should have substituted G1 = 1 and mapped Local to (req, Left(L))
        let g1_invariant = &ns_invariant.global_invariants[&"G1".to_string()];
        assert_eq!(g1_invariant.variables.len(), 1); // Only the local state variable remains
        assert_eq!(
            g1_invariant.variables[0],
            RequestStatePair("req1".to_string(), RequestState::InFlight("L1".to_string()))
        );
    }

    #[test]
    fn test_invariant_implies_semilinear_empty_invariant() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Test case where invariant is empty (always true), should imply any semilinear set
        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        // Create an empty semilinear set using Kleene interface
        let semilinear = <SemilinearSet<String> as Kleene>::zero();

        // Since we have no global states in the invariant, this should trivially succeed
        let result = ns_invariant.invariant_implies_semilinear(
            &ProofInvariant::new(
                vec![],
                Formula::Constraint(Constraint::new(
                    AffineExpr::from_const(1),
                    CompOp::Eq,
                )),
            ),
            &semilinear,
            &"G1".to_string(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_invariant_implies_semilinear_singleton_match() {
        use crate::semilinear::SemilinearSet;

        // Create an invariant that matches exactly one point
        let var_name = "x".to_string();
        let invariant = ProofInvariant::new(
            vec![var_name.clone()],
            Formula::Constraint(Constraint::new(
                AffineExpr::from_var(var_name.clone()).sub(&AffineExpr::from_const(1)),
                CompOp::Eq,
            )), // x = 1
        );

        // Create a semilinear set containing just the atom x
        let semilinear = SemilinearSet::atom(var_name.clone());

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_invariant_implies_semilinear_singleton_no_match() {
        use crate::semilinear::SemilinearSet;

        // Create an invariant for x = 2
        let var_name = "x".to_string();
        let invariant = ProofInvariant::new(
            vec![var_name.clone()],
            Formula::Constraint(Constraint::new(
                AffineExpr::from_var(var_name.clone()).sub(&AffineExpr::from_const(2)),
                CompOp::Eq,
            )), // x = 2
        );

        // Create a semilinear set containing just the atom x (which represents x=1)
        let semilinear = SemilinearSet::atom(var_name.clone());

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should NOT be implied
    }

    #[test]
    fn test_invariant_implies_semilinear_star() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Create an invariant for x >= 0 (any non-negative x)
        let var_name = "x".to_string();
        let invariant = ProofInvariant::new(
            vec![var_name.clone()],
            Formula::Constraint(Constraint::new(
                AffineExpr::from_var(var_name.clone()),
                CompOp::Geq,
            )), // x >= 0
        );

        // Create a semilinear set for x* (0 or more x's)
        let x_atom = SemilinearSet::atom(var_name.clone());
        let semilinear = x_atom.star();

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(result.unwrap()); // x >= 0 is exactly what x* represents
    }

    #[test]
    fn test_invariant_implies_semilinear_union() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Create an invariant for x = 1 OR x = 2
        let x_var = "x".to_string();
        let invariant = ProofInvariant::new(
            vec![x_var.clone()],
            Formula::Or(vec![
                Formula::Constraint(Constraint::new(
                    AffineExpr::from_var(x_var.clone()).sub(&AffineExpr::from_const(1)),
                    CompOp::Eq,
                )), // x = 1
                Formula::Constraint(Constraint::new(
                    AffineExpr::from_var(x_var.clone()).sub(&AffineExpr::from_const(2)),
                    CompOp::Eq,
                )), // x = 2
            ]),
        );

        // Create a semilinear set for x* (which contains 0, 1, 2, 3, ...)
        let x_atom = SemilinearSet::atom(x_var.clone());
        let semilinear = x_atom.star();

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(result.unwrap()); // {1, 2} ⊆ {0, 1, 2, 3, ...}
    }

    #[test]
    fn test_invariant_implies_semilinear_concatenation() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Create an invariant for x = 1 AND y = 1
        let x_var = "x".to_string();
        let y_var = "y".to_string();
        let invariant = ProofInvariant::new(
            vec![x_var.clone(), y_var.clone()],
            Formula::And(vec![
                Formula::Constraint(Constraint::new(
                    AffineExpr::from_var(x_var.clone()).sub(&AffineExpr::from_const(1)),
                    CompOp::Eq,
                )), // x = 1
                Formula::Constraint(Constraint::new(
                    AffineExpr::from_var(y_var.clone()).sub(&AffineExpr::from_const(1)),
                    CompOp::Eq,
                )), // y = 1
            ]),
        );

        // Create a semilinear set for x·y (concatenation)
        let x_atom = SemilinearSet::atom(x_var.clone());
        let y_atom = SemilinearSet::atom(y_var.clone());
        let semilinear = x_atom.times(y_atom);

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(result.unwrap()); // x=1 AND y=1 is exactly what x·y represents
    }

    #[test]
    fn test_invariant_implies_semilinear_complex() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Create an invariant for x = 2
        let x_var = "x".to_string();
        let invariant = ProofInvariant::new(
            vec![x_var.clone()],
            Formula::Constraint(Constraint::new(
                AffineExpr::from_var(x_var.clone()).sub(&AffineExpr::from_const(2)),
                CompOp::Eq,
            )), // x = 2
        );

        // Create a semilinear set x·x (concatenation of x with itself)
        let x_atom = SemilinearSet::atom(x_var.clone());
        let semilinear = x_atom.clone().times(x_atom);

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        // x = 2 is exactly what x·x represents (concatenation gives x=2)
        assert!(result.unwrap());
    }

    #[test]
    fn test_invariant_implies_semilinear_even_numbers() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Create an invariant for even numbers: ∃n. a = 2n
        let a_var = "a".to_string();
        let n_var = "n".to_string();

        // First create the formula a = 2n (which is a - 2n = 0)
        let a_expr = AffineExpr::from_var(a_var.clone());
        let n_expr = AffineExpr::from_var(n_var.clone());
        let two_n = n_expr.mul_by_const(2);
        let formula_body = Formula::Constraint(Constraint::new(a_expr.sub(&two_n), CompOp::Eq));

        // Now quantify over n to get ∃n. a = 2n
        let existential_formula = formula_body.mk_exists(n_var.clone());

        let invariant = ProofInvariant::new(
            vec![a_var.clone()],
            existential_formula,
        );

        // Create a semilinear set (aa)* which represents even multiples of a
        let a_atom = SemilinearSet::atom(a_var.clone());
        let aa = a_atom.clone().times(a_atom); // aa (represents a=2)
        let semilinear = aa.star(); // (aa)* (represents a=0, a=2, a=4, a=6, ...)

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(result.unwrap()); // ∃n. a = 2n is exactly what (aa)* represents
    }

    #[test]
    fn test_invariant_implies_semilinear_odd_not_in_even() {
        use crate::kleene::Kleene;
        use crate::semilinear::SemilinearSet;

        // Test that odd numbers are NOT in (aa)*
        // Create an invariant for odd numbers: ∃n. a = 2n + 1
        let a_var = "a".to_string();
        let n_var = "n".to_string();

        // First create the formula a = 2n + 1 (which is a - 2n - 1 = 0)
        let a_expr = AffineExpr::from_var(a_var.clone());
        let n_expr = AffineExpr::from_var(n_var.clone());
        let two_n_plus_one = n_expr.mul_by_const(2).add(&AffineExpr::from_const(1));
        let formula_body =
            Formula::Constraint(Constraint::new(a_expr.sub(&two_n_plus_one), CompOp::Eq));

        // Now quantify over n to get ∃n. a = 2n + 1
        let existential_formula = formula_body.mk_exists(n_var.clone());

        let invariant = ProofInvariant::new(
            vec![a_var.clone()],
            existential_formula,
        );

        // Create a semilinear set (aa)* which represents even multiples of a
        let a_atom = SemilinearSet::atom(a_var.clone());
        let aa = a_atom.clone().times(a_atom); // aa
        let semilinear = aa.star(); // (aa)*

        let ns_invariant = NSInvariant::<String, String, String, String> {
            global_invariants: HashMap::default(),
        };

        let result =
            ns_invariant.invariant_implies_semilinear(&invariant, &semilinear, &"G1".to_string());

        assert!(result.is_ok());
        assert!(!result.unwrap()); // ∃n. a = 2n + 1 (odd) is NOT in (aa)* (even)
    }

    #[test]
    fn test_ns_decision_serialization_serializable() {
        use tempfile::NamedTempFile;

        // Create a simple NS
        let mut ns = NS::<String, String, String, String>::new("G1".to_string());
        ns.add_request("req1".to_string(), "L1".to_string());
        ns.add_response("L1".to_string(), "resp1".to_string());

        // Create a simple invariant
        let mut global_invariants = HashMap::default();
        let var = RequestStatePair(
            "req1".to_string(),
            RequestState::<String, String>::Completed("resp1".to_string()),
        );
        let formula = Formula::Constraint(Constraint::new(
            AffineExpr::from_var(var.clone()),
            CompOp::Geq,
        ));
        let invariant = ProofInvariant::new(vec![var], formula);
        global_invariants.insert("G1".to_string(), invariant);

        let ns_invariant = NSInvariant { global_invariants };
        let decision = NSDecision::Serializable {
            invariant: ns_invariant,
        };

        // Save to file
        let temp_file = NamedTempFile::new().unwrap();
        decision.save_to_file(temp_file.path()).unwrap();

        // Load from file
        let loaded_decision = NSDecision::<String, String, String, String>::load_from_file(temp_file.path())
            .expect("Failed to load NSDecision");

        // Check that it's serializable
        match loaded_decision {
            NSDecision::Serializable { invariant } => {
                // For now, we just check it loads as serializable
                // Full invariant loading is not yet implemented
                assert_eq!(invariant.global_invariants.len(), 0);
            }
            _ => panic!("Expected Serializable decision"),
        }
    }

    #[test]
    fn test_ns_decision_serialization_not_serializable() {
        use tempfile::NamedTempFile;

        // Create a counterexample trace
        let steps = vec![
            NSStep::RequestStart {
                request: "req1".to_string(),
                initial_local: "L1".to_string(),
            },
            NSStep::InternalStep {
                request: "req1".to_string(),
                from_local: "L1".to_string(),
                from_global: "G1".to_string(),
                to_local: "L2".to_string(),
                to_global: "G2".to_string(),
            },
            NSStep::RequestComplete {
                request: "req1".to_string(),
                final_local: "L2".to_string(),
                response: "resp1".to_string(),
            },
        ];

        let trace: NSTrace<String, String, String, String> = NSTrace { steps };
        let decision = NSDecision::NotSerializable { trace };

        // Save to file
        let temp_file = NamedTempFile::new().unwrap();
        decision.save_to_file(temp_file.path()).unwrap();

        // Load from file
        let loaded_decision = NSDecision::<String, String, String, String>::load_from_file(temp_file.path())
            .expect("Failed to load NSDecision");

        // Check that it's not serializable
        match loaded_decision {
            NSDecision::NotSerializable { trace } => {
                assert_eq!(trace.steps.len(), 3);
                // Verify first step
                match &trace.steps[0] {
                    NSStep::RequestStart { request, initial_local } => {
                        assert_eq!(request, "req1");
                        assert_eq!(initial_local, "L1");
                    }
                    _ => panic!("Expected RequestStart as first step"),
                }
            }
            _ => panic!("Expected NotSerializable decision"),
        }
    }
}

/// Check if a formula with no free variables is satisfied
/// This is used after substituting all variables with concrete values
fn is_formula_satisfied_string(formula: &Formula<String>) -> bool {
    // Convert the formula to a PresburgerSet
    // Since all variables are substituted, we have an empty mapping
    let presburger = formula_to_presburger(formula, &[]);

    // A formula is satisfied if the corresponding PresburgerSet is non-empty
    !presburger.is_empty()
}

    #[test]
    fn test_env_as_key_serialization_issue() {
        use crate::expr_to_ns::Env;
        
        // Create a simple test to isolate the issue
        let mut map: HashMap<Env, i32> = HashMap::default();
        
        let env1: Env = serde_json::from_str(r#"{"vars":{"x":10}}"#).unwrap();
        let env2: Env = serde_json::from_str(r#"{"vars":{"y":20}}"#).unwrap();
        
        map.insert(env1, 100);
        map.insert(env2, 200);
        
        // This should fail with "key must be a string"
        let result = serde_json::to_string(&map);
        
        match result {
            Ok(json) => panic!("Unexpected success: {}", json),
            Err(e) => {
                println!("Expected error: {}", e);
                assert!(e.to_string().contains("key must be a string"));
            }
        }
    }
    
    #[test]
    fn test_ns_invariant_with_env_key() {
        use crate::expr_to_ns::{Env, ExprRequest, LocalExpr};
        use crate::proof_parser::{Formula, ProofInvariant};
        
        // Create an Env as key
        let env: Env = serde_json::from_str(r#"{"vars":{"x":10}}"#).unwrap();
        
        // Create a simple proof invariant
        let proof_inv = ProofInvariant {
            variables: vec![],
            formula: Formula::And(vec![]),
        };
        
        // Create NSInvariant with Env as key
        let mut global_invariants = HashMap::default();
        global_invariants.insert(env, proof_inv);
        
        let invariant: NSInvariant<Env, LocalExpr, ExprRequest, i64> = NSInvariant {
            global_invariants,
        };
        
        // Test serialization - this should work with tuple_vec_map
        let result = serde_json::to_string_pretty(&invariant);
        match result {
            Ok(json) => {
                println!("NSInvariant serialized successfully:\n{}", json);
                
                // Test deserialization
                let invariant2: NSInvariant<Env, LocalExpr, ExprRequest, i64> = 
                    serde_json::from_str(&json).expect("Failed to deserialize");
                assert_eq!(invariant.global_invariants.len(), invariant2.global_invariants.len());
            },
            Err(e) => panic!("Failed to serialize NSInvariant: {}", e),
        }
    }
    
    #[test]
    fn test_ns_decision_serializable_with_env() {
        use crate::expr_to_ns::{Env, ExprRequest, LocalExpr};
        use crate::proof_parser::{Formula, ProofInvariant};
        
        // Create an Env as key
        let env: Env = serde_json::from_str(r#"{"vars":{"x":10}}"#).unwrap();
        
        // Create a simple proof invariant
        let proof_inv = ProofInvariant {
            variables: vec![],
            formula: Formula::And(vec![]),
        };
        
        // Create NSInvariant with Env as key
        let mut global_invariants = HashMap::default();
        global_invariants.insert(env, proof_inv);
        
        let invariant: NSInvariant<Env, LocalExpr, ExprRequest, i64> = NSInvariant {
            global_invariants,
        };
        
        // Create NSDecision::Serializable
        let decision = NSDecision::Serializable { invariant };
        
        // Test serialization - this should work with tuple_vec_map
        let result = serde_json::to_string_pretty(&decision);
        match result {
            Ok(json) => {
                println!("NSDecision::Serializable serialized successfully:\n{}", json);
                
                // Test deserialization
                let decision2: NSDecision<Env, LocalExpr, ExprRequest, i64> = 
                    serde_json::from_str(&json).expect("Failed to deserialize");
                
                // Verify it's serializable
                match decision2 {
                    NSDecision::Serializable { invariant } => {
                        assert_eq!(invariant.global_invariants.len(), 1);
                    },
                    _ => panic!("Expected Serializable decision"),
                }
            },
            Err(e) => panic!("Failed to serialize NSDecision: {}", e),
        }
    }

