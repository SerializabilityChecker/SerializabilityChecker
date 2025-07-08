// Network System (NS) automata
//
// A Network System is defined by:
// - Requests (Req -> L): Client requests that transition to a local state
// - Responses (L -> Resp): Server responses from a local state
// - Transitions (L,G -> L',G'): State transitions between local and global states

use crate::deterministic_map::{HashMap, HashSet};
use crate::ns_to_petri::ReqPetriState;
use crate::petri::Petri;
use colored::*;
use either::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::hash::Hash;

use crate::kleene::{Kleene, Regex, nfa_to_kleene};
use crate::semilinear::*;

// Use the shared utility function for GraphViz escaping
use crate::utils::string::escape_for_graphviz_id;

// Type aliases to reduce complexity
type PetriPlace<L, G, Req, Resp> =
    Either<ReqPetriState<L, G, Req, Resp>, ReqPetriState<L, G, Req, Resp>>;
type PetriTraceStep<L, G, Req, Resp> = (
    Vec<PetriPlace<L, G, Req, Resp>>,
    Vec<PetriPlace<L, G, Req, Resp>>,
);

// Helper function to properly quote strings for GraphViz labels
fn quote_for_graphviz(s: &str) -> String {
    format!("\"{}\"", s.replace('\"', "\\\""))
}

/// Network System representation with type parameters:
/// - G: Global state type
/// - L: Local state type
/// - Req: Request type
/// - Resp: Response type
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NS<G, L, Req, Resp> {
    /// Initial global state
    pub initial_global: G,

    /// Requests from clients with their target local states
    pub requests: Vec<(Req, L)>,

    /// Responses from local states
    pub responses: Vec<(L, Resp)>,

    /// State transitions (from_local, from_global, to_local, to_global)
    pub transitions: Vec<(L, G, L, G)>,
}

impl<G, L, Req, Resp> NS<G, L, Req, Resp>
where
    G: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Display,
    L: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Display,
    Req: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Display,
    Resp: Clone + PartialEq + Eq + std::hash::Hash + std::fmt::Display,
{
    /// Create a new Network System with the given initial global state
    pub fn new(initial_global: G) -> Self {
        NS {
            initial_global,
            requests: Vec::new(),
            responses: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Set the initial global state
    pub fn set_initial_global(&mut self, initial_global: G) {
        self.initial_global = initial_global;
    }

    /// Add a client request with its target local state
    pub fn add_request(&mut self, request: Req, local_state: L) {
        if !self
            .requests
            .contains(&(request.clone(), local_state.clone()))
        {
            self.requests.push((request, local_state));
        }
    }

    /// Add a response from a local state
    pub fn add_response(&mut self, local_state: L, response: Resp) {
        if !self
            .responses
            .contains(&(local_state.clone(), response.clone()))
        {
            self.responses.push((local_state, response));
        }
    }

    /// Add a state transition
    pub fn add_transition(&mut self, from_local: L, from_global: G, to_local: L, to_global: G) {
        let transition = (
            from_local.clone(),
            from_global.clone(),
            to_local.clone(),
            to_global.clone(),
        );
        if !self.transitions.contains(&transition) {
            self.transitions.push(transition);
        }
    }

    /// Get all unique local states in the network system
    pub fn get_local_states(&self) -> Vec<&L> {
        let mut local_states = HashSet::default();

        // Collect local states from requests
        for (_, local) in &self.requests {
            local_states.insert(local);
        }

        // Collect local states from responses
        for (local, _) in &self.responses {
            local_states.insert(local);
        }

        // Collect local states from transitions
        for (from_local, _, to_local, _) in &self.transitions {
            local_states.insert(from_local);
            local_states.insert(to_local);
        }

        local_states.into_iter().collect()
    }

    /// Get all unique global states in the network system
    pub fn get_global_states(&self) -> Vec<&G> {
        let mut globals = HashSet::default();
        globals.insert(&self.initial_global);

        // Collect global states from transitions
        for (_, from_global, _, to_global) in &self.transitions {
            globals.insert(from_global);
            globals.insert(to_global);
        }

        globals.into_iter().collect()
    }

    /// Get all unique requests in the network system
    pub fn get_requests(&self) -> Vec<&Req> {
        let mut requests = HashSet::default();
        for (req, _) in &self.requests {
            requests.insert(req);
        }
        requests.into_iter().collect()
    }

    /// Get all unique responses in the network system
    pub fn get_responses(&self) -> Vec<&Resp> {
        let mut responses = HashSet::default();
        for (_, resp) in &self.responses {
            responses.insert(resp);
        }
        responses.into_iter().collect()
    }

    /// Make an automaton corresponding to the serialized executions of the network system
    /// An element (g, req, resp, g') is present if there is a
    /// - request req in the network system that goes to some local state l
    /// - a sequence of transitions from l to l' that transitions from g to g'
    /// - a response from l' to resp
    pub fn serialized_automaton(&self) -> Vec<(G, Req, Resp, G)> {
        let mut serialized_automaton: Vec<(G, Req, Resp, G)> = Vec::new();
        // iterate over all global states
        for g in self.get_global_states() {
            // iterate over all requests
            for (req, l) in &self.requests {
                // find all reachable states from (l, g)
                let mut vect = vec![(l, g)];
                let mut reached = HashSet::default();
                while let Some((l, g)) = vect.pop() {
                    reached.insert((l, g));
                    for (l1, g1, l2, g2) in &self.transitions {
                        if l == l1 && g == g1 && !reached.contains(&(l2, g2)) {
                            vect.push((l2, g2));
                        }
                    }
                }
                // find all reachable responses from (l, g)
                let mut reached_responses: HashSet<(&Resp, &G)> = HashSet::default();
                for (l, g) in reached {
                    for (l2, resp) in &self.responses {
                        if l == l2 {
                            reached_responses.insert((resp, g));
                        }
                    }
                }
                // add all reachable (g, req, resp, g') to the serialized automaton
                for (resp, g2) in reached_responses {
                    serialized_automaton.push((g.clone(), req.clone(), resp.clone(), g2.clone()));
                }
            }
        }
        serialized_automaton
    }

    pub fn serialized_automaton_kleene<K: Kleene + Clone>(
        &self,
        atom: impl Fn(Req, Resp) -> K,
    ) -> K {
        let nfa: Vec<(G, K, G)> = self
            .serialized_automaton()
            .into_iter()
            .map(|(g, req, resp, g2)| (g, atom(req, resp), g2))
            .collect();
        nfa_to_kleene(&nfa, self.initial_global.clone())
    }

    pub fn serialized_automaton_regex(&self) -> Regex<String> {
        self.serialized_automaton_kleene(|req, resp| Regex::Atom(format!("{req}/{resp}")))
    }

    pub fn serialized_automaton_semilinear(&self) -> SemilinearSet<String> {
        self.serialized_automaton_kleene(|req, resp| SemilinearSet::atom(format!("{req}/{resp}")))
    }

    /// Serialize the network system to a JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    where
        G: Serialize,
        L: Serialize,
        Req: Serialize,
        Resp: Serialize,
    {
        serde_json::to_string_pretty(self)
    }

    /// Create a network system from a JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error>
    where
        for<'de> G: Deserialize<'de>,
        for<'de> L: Deserialize<'de>,
        for<'de> Req: Deserialize<'de>,
        for<'de> Resp: Deserialize<'de>,
    {
        serde_json::from_str(json)
    }

    /// Generate Graphviz DOT format for visualizing the network system
    pub fn to_graphviz(&self) -> String {
        let mut dot = String::from("digraph NetworkSystem {\n");
        dot.push_str("  // Graph settings\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [fontsize=10];\n");
        dot.push_str("  edge [fontsize=10];\n\n");

        // Define node styles for different types
        dot.push_str("  // Node styles\n");

        // Define separate styles without wildcards
        // Define local state nodes style with proper escaping
        let local_state_nodes: Vec<_> = self
            .get_local_states()
            .iter()
            .map(|local| format!("L_{}", escape_for_graphviz_id(&format!("{}", local))))
            .collect();
        if !local_state_nodes.is_empty() {
            dot.push_str(&format!(
                "  node [style=\"filled,rounded\", fillcolor=lightblue] {}; // Local states\n",
                local_state_nodes.join(" ")
            ));
        }

        let request_nodes: Vec<_> = self
            .get_requests()
            .iter()
            .map(|req| format!("REQ_{}", escape_for_graphviz_id(&format!("{}", req))))
            .collect();
        if !request_nodes.is_empty() {
            dot.push_str(&format!(
                "  node [shape=diamond, style=filled, fillcolor=lightgreen] {}; // Requests\n",
                request_nodes.join(" ")
            ));
        }

        let response_nodes: Vec<_> = self
            .get_responses()
            .iter()
            .map(|resp| format!("RESP_{}", escape_for_graphviz_id(&format!("{}", resp))))
            .collect();
        if !response_nodes.is_empty() {
            dot.push_str(&format!(
                "  node [shape=diamond, style=filled, fillcolor=salmon] {}; // Responses\n",
                response_nodes.join(" ")
            ));
        }
        dot.push('\n');

        // Define all local states
        dot.push_str("  // Local state nodes\n");
        let local_states = self.get_local_states();
        for local in local_states {
            let id = format!("L_{}", escape_for_graphviz_id(&format!("{}", local)));
            let label = quote_for_graphviz(&format!("{}", local));
            dot.push_str(&format!("  {} [label={}];\n", id, label));
        }

        // Define request nodes and connections to local states
        dot.push_str("\n  // Request nodes and connections\n");
        let unique_requests = self.get_requests();
        for req in unique_requests {
            // Create request node with proper escaping
            let req_id = format!("REQ_{}", escape_for_graphviz_id(&format!("{}", req)));
            let req_label = quote_for_graphviz(&format!("{}", req));
            dot.push_str(&format!("  {} [label={}];\n", req_id, req_label));

            // Connect request to local states
            for (request, local) in &self.requests {
                if request == req {
                    let local_id = format!("L_{}", escape_for_graphviz_id(&format!("{}", local)));
                    dot.push_str(&format!("  {} -> {} [style=dashed];\n", req_id, local_id));
                }
            }
        }

        // Define response nodes and connections from local states
        dot.push_str("\n  // Response nodes and connections\n");
        let unique_responses = self.get_responses();
        for resp in unique_responses {
            // Create response node with proper escaping
            let resp_id = format!("RESP_{}", escape_for_graphviz_id(&format!("{}", resp)));
            let resp_label = quote_for_graphviz(&format!("{}", resp));
            dot.push_str(&format!("  {} [label={}];\n", resp_id, resp_label));

            // Connect local states to responses
            for (local, response) in &self.responses {
                if response == resp {
                    let local_id = format!("L_{}", escape_for_graphviz_id(&format!("{}", local)));
                    dot.push_str(&format!("  {} -> {} [style=dashed];\n", local_id, resp_id));
                }
            }
        }

        // Define transitions between local states with global states
        dot.push_str("\n  // Transitions between local states with global states\n");
        for (from_local, from_global, to_local, to_global) in &self.transitions {
            let from_local_id = format!("L_{}", escape_for_graphviz_id(&format!("{}", from_local)));
            let to_local_id = format!("L_{}", escape_for_graphviz_id(&format!("{}", to_local)));
            let transition_label = quote_for_graphviz(&format!("{} → {}", from_global, to_global));

            dot.push_str(&format!(
                "  {} -> {} [label={}, color=blue, penwidth=1.5];\n",
                from_local_id, to_local_id, transition_label
            ));
        }

        // Add serialized automaton visualization
        dot.push_str("\n  // Serialized automaton\n");
        dot.push_str("  subgraph cluster_serialized {\n");
        dot.push_str("    label=\"Serialized Automaton\";\n");
        dot.push_str("    style=dashed;\n");

        // Global state nodes in serialized view
        dot.push_str("    // Global state nodes\n");
        let global_nodes: Vec<_> = self
            .get_global_states()
            .iter()
            .map(|g| format!("G_{}", escape_for_graphviz_id(&format!("{}", g))))
            .collect();
        if !global_nodes.is_empty() {
            dot.push_str(&format!("    node [style=\"filled, rounded\", fillcolor=lightblue] {}; // Global states\n\n",
                global_nodes.join(" ")));
        }

        // Get all global states for the serialized automaton
        let globals = self.get_global_states();
        for global in globals {
            // Check if this is the initial global state
            let is_initial = &self.initial_global == global;

            // Create properly escaped IDs and labels
            let global_id = format!("G_{}", escape_for_graphviz_id(&format!("{}", global)));
            let global_label = if is_initial {
                quote_for_graphviz(&format!("{} (initial)", global))
            } else {
                quote_for_graphviz(&format!("{}", global))
            };

            // Style initial global state differently
            if is_initial {
                dot.push_str(&format!(
                    "    {} [label={}, penwidth=3, color=darkgreen];\n",
                    global_id, global_label
                ));
            } else {
                dot.push_str(&format!("    {} [label={}];\n", global_id, global_label));
            }
        }

        // Add transitions in the serialized automaton
        dot.push_str("\n    // Transitions in serialized automaton\n");
        let serialized = self.serialized_automaton();
        for (from_global, req, resp, to_global) in &serialized {
            let from_global_id =
                format!("G_{}", escape_for_graphviz_id(&format!("{}", from_global)));
            let to_global_id = format!("G_{}", escape_for_graphviz_id(&format!("{}", to_global)));
            let transition_label = quote_for_graphviz(&format!("{} / {}", req, resp));

            dot.push_str(&format!(
                "    {} -> {} [label={}];\n",
                from_global_id, to_global_id, transition_label
            ));
        }

        dot.push_str("  }\n");

        // Close the graph
        dot.push_str("}\n");

        dot
    }

    /// Save GraphViz DOT files to disk and generate visualizations
    ///
    /// # Arguments
    /// * `name` - Base name for the generated files
    /// * `open_files` - Whether to open the generated PNG files for viewing
    ///
    /// Returns a Result with the paths to the generated files or an error message
    pub fn save_graphviz(&self, name: &str, open_files: bool) -> Result<Vec<String>, String> {
        let dot_content = self.to_graphviz();
        crate::graphviz::save_graphviz(&dot_content, name, "network", open_files)
    }

    /// Save GraphViz DOT files to disk and generate visualizations without opening files
    ///
    /// This is a convenience wrapper that calls save_graphviz(name, false)
    pub fn save_graphviz_no_open(&self, name: &str) -> Result<Vec<String>, String> {
        self.save_graphviz(name, false)
    }

    pub fn merge_requests(&mut self, other: &NS<G, L, Req, Resp>) {
        // Merge all requests
        for (req, l) in &other.requests {
            self.add_request(req.clone(), l.clone());
        }

        // Merge all the transitions
        for (l1, g1, l2, g2) in &other.transitions {
            self.add_transition(l1.clone(), g1.clone(), l2.clone(), g2.clone());
        }

        // Merge all responses
        for (l, resp) in &other.responses {
            self.add_response(l.clone(), resp.clone());
        }
    }

    /// Check if a trace can be executed by this NS
    /// Returns Ok(multiset of (request, response) pairs) if valid and no requests in flight
    /// Returns Err(message) if invalid or if requests remain in flight
    pub fn check_trace(
        &self,
        trace: &crate::ns_decision::NSTrace<G, L, Req, Resp>,
    ) -> Result<Vec<(Req, Resp)>, String> {
        use crate::ns_decision::NSStep;

        // Initialize simulation state
        let mut global_state = self.initial_global.clone();
        let mut in_flight: Vec<(Req, L)> = Vec::new(); // Multiset of active requests
        let mut completed: Vec<(Req, Resp)> = Vec::new(); // Multiset of completed requests

        // Process each step in the trace
        for (step_idx, step) in trace.steps.iter().enumerate() {
            match step {
                NSStep::RequestStart {
                    request,
                    initial_local,
                } => {
                    // Verify this request type exists with the given initial local state
                    if !self
                        .requests
                        .contains(&(request.clone(), initial_local.clone()))
                    {
                        return Err(format!(
                            "Step {}: Unknown request type or wrong initial state: ({}, {})",
                            step_idx, request, initial_local
                        ));
                    }

                    // Add to in-flight multiset
                    in_flight.push((request.clone(), initial_local.clone()));
                }

                NSStep::InternalStep {
                    request,
                    from_local,
                    from_global,
                    to_local,
                    to_global,
                } => {
                    // Verify global state matches
                    if &global_state != from_global {
                        return Err(format!(
                            "Step {}: Global state mismatch: expected {}, found {}",
                            step_idx, from_global, global_state
                        ));
                    }

                    // Verify transition exists
                    let transition = (
                        from_local.clone(),
                        from_global.clone(),
                        to_local.clone(),
                        to_global.clone(),
                    );
                    if !self.transitions.contains(&transition) {
                        return Err(format!(
                            "Step {}: Transition not found in NS: ({}, {}, {}, {})",
                            step_idx, from_local, from_global, to_local, to_global
                        ));
                    }

                    // Find and remove the matching request from in-flight
                    let request_entry = (request.clone(), from_local.clone());
                    if let Some(pos) = in_flight.iter().position(|entry| entry == &request_entry) {
                        in_flight.remove(pos);
                    } else {
                        return Err(format!(
                            "Step {}: No active request found matching: ({}, {})",
                            step_idx, request, from_local
                        ));
                    }

                    // Add updated request back to in-flight
                    in_flight.push((request.clone(), to_local.clone()));

                    // Update global state
                    global_state = to_global.clone();
                }

                NSStep::RequestComplete {
                    request,
                    final_local,
                    response,
                } => {
                    // Verify response exists
                    if !self
                        .responses
                        .contains(&(final_local.clone(), response.clone()))
                    {
                        return Err(format!(
                            "Step {}: Response not found in NS: ({}, {})",
                            step_idx, final_local, response
                        ));
                    }

                    // Find and remove the matching request from in-flight
                    let request_entry = (request.clone(), final_local.clone());
                    if let Some(pos) = in_flight.iter().position(|entry| entry == &request_entry) {
                        in_flight.remove(pos);
                    } else {
                        return Err(format!(
                            "Step {}: No active request found matching: ({}, {})",
                            step_idx, request, final_local
                        ));
                    }

                    // Add to completed multiset
                    completed.push((request.clone(), response.clone()));
                }
            }
        }

        // Check that no requests remain in flight
        if !in_flight.is_empty() {
            let in_flight_str: Vec<String> = in_flight
                .iter()
                .map(|(req, local)| format!("({}, {})", req, local))
                .collect();
            return Err(format!(
                "Requests still in flight at end of trace: [{}]",
                in_flight_str.join(", ")
            ));
        }

        Ok(completed)
    }
}

impl<G, L, Req, Resp> NS<G, L, Req, Resp>
where
    G: Clone + Ord + Hash + Display + Debug,
    L: Clone + Ord + Hash + Display + Debug,
    Req: Clone + Ord + Hash + Display + Debug,
    Resp: Clone + Ord + Hash + Display + Debug,
{
    /// Check if the network system is serializable using both methods and report results
    #[must_use]
    pub fn is_serializable(&self, out_dir: &str) -> bool 
    where
        G: Clone + Ord + Hash + Display + std::fmt::Debug + serde::Serialize + for<'de> serde::Deserialize<'de>,
        L: Clone + Ord + Hash + Display + std::fmt::Debug + serde::Serialize + for<'de> serde::Deserialize<'de>,
        Req: Clone + Ord + Hash + Display + std::fmt::Debug + serde::Serialize + for<'de> serde::Deserialize<'de>,
        Resp: Clone + Ord + Hash + Display + std::fmt::Debug + serde::Serialize + for<'de> serde::Deserialize<'de>,
    {
        // Create certificate with timing
        let decision = crate::stats::record_certificate_creation_time(|| {
            self.create_certificate(out_dir)
        });
        
        // Save certificate to standard location
        let cert_path = format!("{}/certificate.json", out_dir);
        if let Err(err) = decision.save_to_file(&cert_path) {
            eprintln!("Warning: Failed to save certificate: {}", err);
            // Continue with the in-memory decision
        }
        
        // Load certificate from file
        let loaded_decision = match crate::ns_decision::NSDecision::load_from_file(&cert_path) {
            Ok(d) => d,
            Err(err) => {
                eprintln!("Warning: Failed to load certificate: {}. Using in-memory decision.", err);
                decision
            }
        };
        
        // Verify and return result with timing
        let result = crate::stats::record_certificate_checking_time(|| {
            self.verify_ns_decision(&loaded_decision)
        });
        
        // Print result with consistent formatting
        println!();
        println!(
            "{}",
            "────────────────────────────────────────────────────────────".bright_black()
        );
        println!(
            "{} {}",
            "🔍".yellow(),
            "SERIALIZABILITY ANALYSIS".yellow().bold()
        );
        println!(
            "{}",
            "────────────────────────────────────────────────────────────".bright_black()
        );
        
        // Print the semilinear set for compatibility
        println!();
        println!("Serialized automaton semilinear set:");
        println!("{}", self.serialized_automaton_semilinear());
        
        // Print decision details
        match &loaded_decision {
            crate::ns_decision::NSDecision::Serializable { invariant } => {
                println!();
                println!("✅ PROOF CERTIFICATE FOUND");
                println!();
                invariant.pretty_print_with_verification(self);
            }
            crate::ns_decision::NSDecision::NotSerializable { trace } => {
                println!();
                println!("❌ COUNTEREXAMPLE TRACE FOUND");
                println!();
                trace.pretty_print(self);
            }
            crate::ns_decision::NSDecision::Timeout { message } => {
                println!();
                println!("⏱️ ANALYSIS TIMED OUT");
                println!();
                println!("{}", message);
            }
        }
        
        // Determine the result and stats string based on decision type
        let (result_emoji, result_text, stats_result) = match &loaded_decision {
            crate::ns_decision::NSDecision::Serializable { .. } => ("✅", "SERIALIZABLE".green().bold(), "serializable"),
            crate::ns_decision::NSDecision::NotSerializable { .. } => ("❌", "NOT SERIALIZABLE".red().bold(), "not_serializable"),
            crate::ns_decision::NSDecision::Timeout { .. } => ("⏱️", "TIMEOUT".yellow().bold(), "timeout"),
        };
        
        println!();
        println!(
            "{}",
            "════════════════════════════════════════════════════════════".bright_black()
        );
        println!(
            "{} {}",
            result_emoji,
            format!("RESULT: {}", result_text)
        );
        println!(
            "{}",
            "════════════════════════════════════════════════════════════".bright_black()
        );
        
        // Record result in stats
        crate::stats::set_analysis_result(stats_result);
        
        result
    }

    /// Create a serializability certificate (NSDecision) without full visualization
    pub fn create_certificate(&self, out_dir: &str) -> crate::ns_decision::NSDecision<G, L, Req, Resp>
    where
        G: Clone + Ord + Hash + Display + std::fmt::Debug,
        L: Clone + Ord + Hash + Display + std::fmt::Debug,
        Req: Clone + Ord + Hash + Display + std::fmt::Debug,
        Resp: Clone + Ord + Hash + Display + std::fmt::Debug,
    {
        use crate::ns_to_petri::*;
        use ReqPetriState::*;

        // Initialize debug logger
        let program_name = std::path::Path::new(out_dir)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        crate::reachability::init_debug_logger(
            program_name.clone(),
            format!("Network System: {:?}", self),
        );

        // Convert to Petri net
        let mut places_that_must_be_zero = HashSet::default();
        let petri = ns_to_petri_with_requests(self).rename(|st| match st {
            Response(_, _) => Right(st),
            Global(_) => Left(st),
            Local(_, _) | Request(_) => {
                places_that_must_be_zero.insert(st.clone());
                Left(st)
            }
        });
        let places_that_must_be_zero: Vec<_> = places_that_must_be_zero.into_iter().collect();

        // Create serialized automaton semilinear set
        let ser: SemilinearSet<_> = self.serialized_automaton_kleene(|req, resp| {
            SemilinearSet::singleton(SparseVector::unit(Response(req, resp)))
        });
        
        // Collect Petri net size stats
        let places_count = petri.get_places().len();
        let transitions_count = petri.get_transitions().len();
        crate::stats::set_petri_net_sizes(places_count, transitions_count);
        
        // Collect semilinear set stats
        let semilinear_stats = crate::stats::SemilinearSetStats {
            num_components: ser.components.len(),
            components: ser.components.iter().map(|c| crate::stats::SemilinearComponent {
                periods: c.periods.len(),
            }).collect(),
        };
        crate::stats::set_semilinear_stats(semilinear_stats);

        // Run the proof-based analysis to get Decision
        let result_with_proofs =
            crate::reachability_with_proofs::is_petri_reachability_set_subset_of_semilinear_new(
                petri.clone(),
                &places_that_must_be_zero,
                ser.clone(),
                out_dir,
            );

        // Convert Petri decision to NS decision
        crate::ns_decision::petri_decision_to_ns(result_with_proofs, self)
    }

    /// Verify an NSDecision against this Network System
    /// Returns true if the system is serializable based on the decision
    pub fn verify_ns_decision(&self, decision: &crate::ns_decision::NSDecision<G, L, Req, Resp>) -> bool
    where
        G: Clone + Ord + Hash + Display + std::fmt::Debug,
        L: Clone + Ord + Hash + Display + std::fmt::Debug,
        Req: Clone + Ord + Hash + Display + std::fmt::Debug,
        Resp: Clone + Ord + Hash + Display + std::fmt::Debug,
    {
        match decision {
            crate::ns_decision::NSDecision::Serializable { invariant } => {
                // If we have a valid proof, the system is serializable
                invariant.check_proof(self).is_ok()
            }
            crate::ns_decision::NSDecision::NotSerializable { trace } => {
                // If we have a valid counterexample trace, the system is NOT serializable
                // So we return false (not serializable)
                if self.check_trace(trace).is_ok() {
                    false // Valid counterexample means not serializable
                } else {
                    // Invalid trace - this shouldn't happen, but we can't conclude serializability
                    eprintln!("Warning: Invalid counterexample trace found in certificate");
                    false
                }
            }
            crate::ns_decision::NSDecision::Timeout { .. } => {
                // Timeout means we cannot determine serializability
                eprintln!("Warning: Analysis timed out - cannot determine serializability");
                false
            }
        }
    }
}

fn display_vec<T: Display>(v: &[T]) -> String {
    v.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Prints a counterexample trace step-by-step on the given Petri net.
fn print_counterexample_trace<L, G, Req, Resp>(
    petri: &Petri<PetriPlace<L, G, Req, Resp>>,
    trace: &[PetriTraceStep<L, G, Req, Resp>],
) where
    L: Clone + Eq + PartialEq + Hash + std::fmt::Display,
    G: Clone + Eq + PartialEq + Hash + std::fmt::Display,
    Req: Clone + Eq + PartialEq + Hash + std::fmt::Display,
    Resp: Clone + Eq + PartialEq + Hash + std::fmt::Display,
{
    // Header
    println!("{}", "❌ COUNTEREXAMPLE TRACE FOUND".bold().red());

    if trace.is_empty() {
        // Empty trace
        println!("{}", "(Empty trace – violation at initial state)".yellow());
    } else {
        // Show transition sequence with details
        println!("{}", "Transition sequence:".yellow());
        for (i, (inputs, outputs)) in trace.iter().enumerate() {
            println!(
                "{}",
                format!(
                    "  {}. {} → {}",
                    i + 1,
                    if inputs.is_empty() {
                        "∅".to_string()
                    } else {
                        display_vec(inputs)
                    },
                    if outputs.is_empty() {
                        "∅".to_string()
                    } else {
                        display_vec(outputs)
                    }
                )
                .yellow()
            );
        }
        println!();

        // Replay
        let mut marking = petri.get_initial_marking();
        println!(
            "{}",
            format!("Step 0 – initial marking: {}", display_vec(&marking)).yellow()
        );

        for (i, (inputs, outputs)) in trace.iter().enumerate() {
            // consume inputs
            for p in inputs {
                if let Some(pos) = marking.iter().position(|x| x == p) {
                    marking.remove(pos);
                } else {
                    println!(
                        "{}",
                        format!(
                            "Step {} – transition {}: input {} not in marking",
                            i + 1,
                            i + 1,
                            p
                        )
                        .bold()
                        .red()
                    );
                    println!(
                        "{}",
                        format!("Current marking: {}", display_vec(&marking)).red()
                    );
                    println!("{}", "Note: This may indicate a bug!".red());
                    // Don't panic, just continue to show the rest of the trace
                    return;
                }
            }
            // produce outputs
            marking.extend(outputs.clone());

            println!(
                "{}",
                format!(
                    "Step {} – fired transition: inputs={}, outputs={}, marking={}",
                    i + 1,
                    display_vec(inputs),
                    display_vec(outputs),
                    display_vec(&marking)
                )
                .yellow()
            );
        }

        // Final marking summary
        let total_tokens = marking.len();
        let mut counts = HashMap::default();
        for p in &marking {
            *counts.entry(p).or_insert(0) += 1;
        }
        let unique_places = counts.len();
        println!(
            "{}",
            format!(
                "Final marking has {} token(s) across {} place(s)",
                total_tokens, unique_places
            )
            .yellow()
        );
        println!("{}", "Places with tokens:".yellow());
        for (place, count) in &counts {
            println!("{}", format!("{}: {} token(s)", place, count).yellow());
        }

        // Conclusion
        println!(
            "{}",
            "This trace demonstrates a non-serializable execution, with the following outputs"
                .yellow()
        );
        // cyan separator
        println!(
            "{}",
            "================================================================================
        "
            .cyan()
        );
        print!("{}", "❌ COUNTEREXAMPLE request/responses: ".bold().red());
        // for each place, look for the Debug pattern "Right(Response(...), resp)" and extract
        for (place, &cnt) in &counts {
            use crate::ns_to_petri::ReqPetriState::Response;
            if let Right(Response(req, resp)) = place {
                if cnt == 1 {
                    print!("{req}/{resp} ");
                } else {
                    print!("({req}/{resp})^{cnt} ");
                }
            }
        }
        println!();
    }
}

/// Given something like `"ExprRequest { name: \"foo\" }, 0)"`,
/// returns Some(("foo", 0)) or None.
fn extract_name_and_value(s: &str) -> Option<(String, usize)> {
    // 1) Trim any surrounding quotes:
    let inner = s.trim().trim_matches('"').trim();

    // 2) Turn `\"` into plain `"` so our split-on-quote works:
    let unescaped = inner.replace("\\\"", "\"");

    // 3) Grab the request name between the first pair of real quotes:
    let name = unescaped.split('"').nth(1)?.to_string();

    // 4) Grab the number after the comma and before the `)`:
    let num_part = unescaped.split_once(',')?.1.trim();
    let num = num_part.trim_end_matches(')').parse::<usize>().ok()?;

    Some((name, num))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ns_parse() {
        let input = r#"
            {
                "initial_global": "G0",
                "requests": [["Req1", "L0"], ["Req2", "L1"], ["Req3", "L2"]],
                "responses": [["L0", "RespA"], ["L1", "RespB"], ["L2", "RespC"]],
                "transitions": [
                    ["L0", "G0", "L1", "G1"],
                    ["L1", "G1", "L2", "G2"],
                    ["L2", "G2", "L0", "G3"]
                ]
            }"#;

        let ns: NS<String, String, String, String> = serde_json::from_str(input).unwrap();

        assert_eq!(ns.requests.len(), 3);
        assert_eq!(ns.responses.len(), 3);
        assert_eq!(ns.transitions.len(), 3);

        assert_eq!(ns.requests[0], ("Req1".to_string(), "L0".to_string()));
        assert_eq!(ns.responses[1], ("L1".to_string(), "RespB".to_string()));
        assert_eq!(
            ns.transitions[2],
            (
                "L2".to_string(),
                "G2".to_string(),
                "L0".to_string(),
                "G3".to_string()
            )
        );
    }

    #[test]
    fn test_ns_from_json() {
        let input = r#"
            {
                "initial_global": "G0",
                "requests": [["Req1", "L0"], ["Req2", "L1"]],
                "responses": [["L0", "RespA"], ["L1", "RespB"]],
                "transitions": [
                    ["L0", "G0", "L1", "G1"],
                    ["L1", "G1", "L0", "G0"]
                ]
            }"#;

        let ns = NS::<String, String, String, String>::from_json(input).unwrap();

        assert_eq!(ns.requests.len(), 2);
        assert_eq!(ns.responses.len(), 2);
        assert_eq!(ns.transitions.len(), 2);
    }

    #[test]
    fn test_ns_build_and_serialize() {
        let mut ns = NS::<String, String, String, String>::new("EmptySession".to_string());

        // Add requests
        ns.add_request("Login".to_string(), "Start".to_string());
        ns.add_request("Query".to_string(), "LoggedIn".to_string());

        // Add responses
        ns.add_response("Start".to_string(), "LoginResult".to_string());
        ns.add_response("LoggedIn".to_string(), "QueryResult".to_string());

        // Add transitions
        ns.add_transition(
            "Start".to_string(),
            "EmptySession".to_string(),
            "LoggedIn".to_string(),
            "ActiveSession".to_string(),
        );

        ns.add_transition(
            "LoggedIn".to_string(),
            "ActiveSession".to_string(),
            "Start".to_string(),
            "EmptySession".to_string(),
        );

        // Test serialization
        let json = ns.to_json().unwrap();
        assert!(json.contains("\"requests\""));
        assert!(json.contains("\"responses\""));
        assert!(json.contains("\"transitions\""));

        // Test deserialization roundtrip
        let ns2 = NS::<String, String, String, String>::from_json(&json).unwrap();
        assert_eq!(ns.requests.len(), ns2.requests.len());
        assert_eq!(ns.transitions.len(), ns2.transitions.len());
    }

    #[test]
    fn test_check_trace() {
        use crate::ns_decision::{NSStep, NSTrace};

        // Create a simple NS
        let mut ns = NS::<String, String, String, String>::new("G0".to_string());

        // Add requests
        ns.add_request("Req1".to_string(), "L0".to_string());
        ns.add_request("Req2".to_string(), "L1".to_string());

        // Add transitions
        ns.add_transition(
            "L0".to_string(),
            "G0".to_string(),
            "L2".to_string(),
            "G1".to_string(),
        );
        ns.add_transition(
            "L1".to_string(),
            "G1".to_string(),
            "L3".to_string(),
            "G2".to_string(),
        );

        // Add responses
        ns.add_response("L2".to_string(), "Resp1".to_string());
        ns.add_response("L3".to_string(), "Resp2".to_string());

        // Test 1: Valid trace with two requests completing successfully
        let trace1 = NSTrace {
            steps: vec![
                NSStep::RequestStart {
                    request: "Req1".to_string(),
                    initial_local: "L0".to_string(),
                },
                NSStep::InternalStep {
                    request: "Req1".to_string(),
                    from_local: "L0".to_string(),
                    from_global: "G0".to_string(),
                    to_local: "L2".to_string(),
                    to_global: "G1".to_string(),
                },
                NSStep::RequestStart {
                    request: "Req2".to_string(),
                    initial_local: "L1".to_string(),
                },
                NSStep::RequestComplete {
                    request: "Req1".to_string(),
                    final_local: "L2".to_string(),
                    response: "Resp1".to_string(),
                },
                NSStep::InternalStep {
                    request: "Req2".to_string(),
                    from_local: "L1".to_string(),
                    from_global: "G1".to_string(),
                    to_local: "L3".to_string(),
                    to_global: "G2".to_string(),
                },
                NSStep::RequestComplete {
                    request: "Req2".to_string(),
                    final_local: "L3".to_string(),
                    response: "Resp2".to_string(),
                },
            ],
        };

        let result1 = ns.check_trace(&trace1);
        assert!(result1.is_ok());
        let completed = result1.unwrap();
        assert_eq!(completed.len(), 2);
        assert!(completed.contains(&("Req1".to_string(), "Resp1".to_string())));
        assert!(completed.contains(&("Req2".to_string(), "Resp2".to_string())));

        // Test 2: Invalid trace - request still in flight
        let trace2 = NSTrace {
            steps: vec![
                NSStep::RequestStart {
                    request: "Req1".to_string(),
                    initial_local: "L0".to_string(),
                },
                NSStep::InternalStep {
                    request: "Req1".to_string(),
                    from_local: "L0".to_string(),
                    from_global: "G0".to_string(),
                    to_local: "L2".to_string(),
                    to_global: "G1".to_string(),
                },
                // Missing RequestComplete for Req1
            ],
        };

        let result2 = ns.check_trace(&trace2);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().contains("Requests still in flight"));

        // Test 3: Invalid trace - wrong global state
        let trace3 = NSTrace {
            steps: vec![
                NSStep::RequestStart {
                    request: "Req1".to_string(),
                    initial_local: "L0".to_string(),
                },
                NSStep::InternalStep {
                    request: "Req1".to_string(),
                    from_local: "L0".to_string(),
                    from_global: "G1".to_string(), // Wrong! Should be G0
                    to_local: "L2".to_string(),
                    to_global: "G1".to_string(),
                },
            ],
        };

        let result3 = ns.check_trace(&trace3);
        assert!(result3.is_err());
        assert!(result3.unwrap_err().contains("Global state mismatch"));

        // Test 4: Invalid trace - unknown request
        let trace4 = NSTrace {
            steps: vec![NSStep::RequestStart {
                request: "UnknownReq".to_string(),
                initial_local: "L0".to_string(),
            }],
        };

        let result4 = ns.check_trace(&trace4);
        assert!(result4.is_err());
        assert!(result4.unwrap_err().contains("Unknown request type"));
    }

    #[test]
    fn test_get_local_and_global_states() {
        let mut ns = NS::<String, String, String, String>::new("G1".to_string());

        // Add transitions
        ns.add_transition(
            "L1".to_string(),
            "G1".to_string(),
            "L2".to_string(),
            "G2".to_string(),
        );

        ns.add_transition(
            "L2".to_string(),
            "G2".to_string(),
            "L3".to_string(),
            "G3".to_string(),
        );

        // Check local states
        let local_states = ns.get_local_states();
        assert_eq!(local_states.len(), 3);
        assert!(local_states.iter().any(|&l| l == "L1"));
        assert!(local_states.iter().any(|&l| l == "L2"));
        assert!(local_states.iter().any(|&l| l == "L3"));

        // Check global states
        let globals = ns.get_global_states();
        assert_eq!(globals.len(), 3);
        assert!(globals.iter().any(|&g| g == "G1"));
        assert!(globals.iter().any(|&g| g == "G2"));
        assert!(globals.iter().any(|&g| g == "G3"));
    }
    #[test]
    fn test_serialized_automaton_no_transitions() {
        let mut ns = NS::<String, String, String, String>::new("Initial".to_string());
        // Add a single request and response but no transitions
        ns.add_request("Req1".to_string(), "L0".to_string());
        ns.add_response("L0".to_string(), "RespA".to_string());

        // Without transitions, a request to L0 directly creates a response since L0 has
        // a response RespA. This will produce a tuple (Initial, Req1, RespA, Initial).
        let automaton = ns.serialized_automaton();
        assert_eq!(automaton.len(), 1);
        assert_eq!(
            automaton[0],
            (
                "Initial".to_string(),
                "Req1".to_string(),
                "RespA".to_string(),
                "Initial".to_string()
            )
        );
    }

    #[test]
    fn test_serialized_automaton_single_transition() {
        let mut ns = NS::<String, String, String, String>::new("G0".to_string());

        // Request/Response
        ns.add_request("Req1".to_string(), "L0".to_string());
        ns.add_response("L1".to_string(), "RespA".to_string());

        // One transition from (L0, G0) -> (L1, G1)
        ns.add_transition(
            "L0".to_string(),
            "G0".to_string(),
            "L1".to_string(),
            "G1".to_string(),
        );

        // Now we expect to see if, for initial global state "G0", the request "Req1"
        // can eventually yield a response "RespA" in global state "G1".
        // That should produce exactly one tuple: (G0, Req1, RespA, G1).
        let automaton = ns.serialized_automaton();
        assert_eq!(automaton.len(), 1);
        assert_eq!(
            automaton[0],
            (
                "G0".to_string(),
                "Req1".to_string(),
                "RespA".to_string(),
                "G1".to_string()
            )
        );
    }

    #[test]
    fn test_serialized_automaton_chain_of_transitions() {
        let mut ns = NS::<String, String, String, String>::new("G0".to_string());

        // Requests / Responses
        ns.add_request("Req1".to_string(), "L0".to_string());
        ns.add_response("L2".to_string(), "RespA".to_string());

        // Chain: (L0, G0) -> (L1, G1) -> (L2, G2)
        ns.add_transition(
            "L0".to_string(),
            "G0".to_string(),
            "L1".to_string(),
            "G1".to_string(),
        );
        ns.add_transition(
            "L1".to_string(),
            "G1".to_string(),
            "L2".to_string(),
            "G2".to_string(),
        );

        // We should get (G0, Req1, RespA, G2) because from (L0, G0),
        // we can walk transitions to (L2, G2) which has response "RespA".
        let automaton = ns.serialized_automaton();
        assert_eq!(automaton.len(), 1);
        assert_eq!(
            automaton[0],
            (
                "G0".to_string(),
                "Req1".to_string(),
                "RespA".to_string(),
                "G2".to_string()
            )
        );
    }

    #[test]
    fn test_serialized_automaton_branching_paths() {
        let mut ns = NS::<String, String, String, String>::new("G0".to_string());

        // Requests
        ns.add_request("ReqA".to_string(), "L0".to_string());
        ns.add_request("ReqB".to_string(), "L0".to_string());
        // Responses
        ns.add_response("L1".to_string(), "RespA".to_string());
        ns.add_response("L2".to_string(), "RespB".to_string());

        // Branch: (L0, G0) -> (L1, G1) or (L0, G0) -> (L2, G2)
        ns.add_transition(
            "L0".to_string(),
            "G0".to_string(),
            "L1".to_string(),
            "G1".to_string(),
        );
        ns.add_transition(
            "L0".to_string(),
            "G0".to_string(),
            "L2".to_string(),
            "G2".to_string(),
        );

        // For request "ReqA" or "ReqB" starting from initial global state G0 and local L0:
        //   - We can reach L1, G1 => yields "RespA"
        //   - We can reach L2, G2 => yields "RespB"
        //
        // So we expect:
        //   (G0, ReqA, RespA, G1)
        //   (G0, ReqA, RespB, G2)
        //   (G0, ReqB, RespA, G1)
        //   (G0, ReqB, RespB, G2)
        let mut results = ns.serialized_automaton();
        results.sort(); // sort for consistent assertion

        assert_eq!(results.len(), 4);
        assert_eq!(
            results,
            vec![
                (
                    "G0".to_string(),
                    "ReqA".to_string(),
                    "RespA".to_string(),
                    "G1".to_string()
                ),
                (
                    "G0".to_string(),
                    "ReqA".to_string(),
                    "RespB".to_string(),
                    "G2".to_string()
                ),
                (
                    "G0".to_string(),
                    "ReqB".to_string(),
                    "RespA".to_string(),
                    "G1".to_string()
                ),
                (
                    "G0".to_string(),
                    "ReqB".to_string(),
                    "RespB".to_string(),
                    "G2".to_string()
                ),
            ]
        );
    }

    #[test]
    fn test_serialized_automaton_cycle() {
        let mut ns = NS::<String, String, String, String>::new("G0".to_string());

        // Request -> local state L0
        ns.add_request("Req1".to_string(), "L0".to_string());
        // Response from local state L0
        ns.add_response("L0".to_string(), "RespX".to_string());

        // Cycle: (L0, G0) -> (L0, G0)
        ns.add_transition(
            "L0".to_string(),
            "G0".to_string(),
            "L0".to_string(),
            "G0".to_string(),
        );

        // Because there's a cycle on (L0, G0), we remain in the same local/global pair,
        // which has response "RespX". That means:
        //   from G0, with request Req1 that goes to L0, we can stay in L0, G0 indefinitely.
        // The result is (G0, Req1, RespX, G0).
        let automaton = ns.serialized_automaton();
        assert_eq!(automaton.len(), 1);
        assert_eq!(
            automaton[0],
            (
                "G0".to_string(),
                "Req1".to_string(),
                "RespX".to_string(),
                "G0".to_string()
            )
        );
    }

    #[test]
    fn test_graphviz_output() {
        let mut ns = NS::<String, String, String, String>::new("NoSession".to_string());

        // Add requests and responses
        ns.add_request("Login".to_string(), "Init".to_string());
        ns.add_response("LoggedIn".to_string(), "Success".to_string());

        // Add transition
        ns.add_transition(
            "Init".to_string(),
            "NoSession".to_string(),
            "LoggedIn".to_string(),
            "ActiveSession".to_string(),
        );

        // Generate GraphViz DOT
        let dot = ns.to_graphviz();

        // Basic checks on the output format
        assert!(dot.starts_with("digraph NetworkSystem {"));
        assert!(dot.ends_with("}\n"));

        // Check for local state nodes
        assert!(dot.contains("L_Init [label=\"Init\"]"));
        assert!(dot.contains("L_LoggedIn [label=\"LoggedIn\"]"));

        // Check for request and response nodes
        assert!(dot.contains("REQ_Login [label=\"Login\"]"));
        assert!(dot.contains("RESP_Success [label=\"Success\"]"));

        // Check for connections
        assert!(dot.contains("REQ_Login -> L_Init"));
        assert!(dot.contains("L_LoggedIn -> RESP_Success"));

        // Check for transition
        assert!(dot.contains("L_Init -> L_LoggedIn"));
        assert!(dot.contains("NoSession → ActiveSession"));

        // Check serialized automaton section
        assert!(dot.contains("subgraph cluster_serialized"));
        assert!(dot.contains("G_NoSession"));
        assert!(dot.contains("G_ActiveSession"));
        assert!(dot.contains("G_NoSession -> G_ActiveSession"));
        assert!(dot.contains("Login / Success"));
    }

    // #[test]
    // fn test_save_graphviz() {
    //     // This test is conditional on GraphViz being installed
    //     // We'll only verify the file creation, not the PNG generation

    //     let mut ns = NS::<String, String, String, String>::new("G1".to_string());

    //     // Add a simple system
    //     ns.add_request("Req".to_string(), "L1".to_string());
    //     ns.add_response("L2".to_string(), "Resp".to_string());

    //     ns.add_transition(
    //         "L1".to_string(),
    //         "G1".to_string(),
    //         "L2".to_string(),
    //         "G2".to_string(),
    //     );

    //     // Save to out directory with test prefix, don't open files during testing
    //     let result = ns.save_graphviz("test_graphviz", false);

    //     // Check if saving worked (may fail if GraphViz not installed)
    //     if result.is_ok() {
    //         let files = result.unwrap();

    //         // Check DOT files were created
    //         assert!(files.iter().any(|f| f.contains("network.dot")));

    //         // Check if files exist
    //         assert!(Path::new("out/test_graphviz/network.dot").exists());

    //         // Clean up test files
    //         let _ = fs::remove_dir_all("out/test_graphviz");
    //     }
    //     // Note: We don't assert on error case since GraphViz might not be installed
    // }
}
