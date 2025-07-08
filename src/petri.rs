use crate::deterministic_map::{HashMap, HashSet};
use crate::graphviz;
use crate::utils::string::escape_for_graphviz_id;
use std::hash::Hash;

#[derive(Clone)]
pub struct Petri<Place> {
    initial_marking: Vec<Place>,
    transitions: Vec<(Vec<Place>, Vec<Place>)>,
}

impl<Place> Petri<Place>
where
    Place: Clone + PartialEq + Eq + Hash,
{
    /// Create a new Petri net with initial marking
    pub fn new(initial_marking: Vec<Place>) -> Self {
        Petri {
            initial_marking,
            transitions: Vec::new(),
        }
    }

    /// Add a transition to the Petri net (input places, output places)
    pub fn add_transition(&mut self, input: Vec<Place>, output: Vec<Place>) {
        self.transitions.push((input, output));
    }

    /// Get all unique places in the Petri net
    pub fn get_places(&self) -> Vec<Place> {
        let mut places = HashSet::default();

        // Collect places from initial marking
        for place in &self.initial_marking {
            places.insert(place.clone());
        }

        // Collect places from transitions
        for (input, output) in &self.transitions {
            for place in input {
                places.insert(place.clone());
            }
            for place in output {
                places.insert(place.clone());
            }
        }

        places.into_iter().collect()
    }

    /// Get the initial marking of the Petri net
    pub fn get_initial_marking(&self) -> Vec<Place> {
        self.initial_marking.clone()
    }

    /// Get all transitions in the Petri net
    pub fn get_transitions(&self) -> Vec<(Vec<Place>, Vec<Place>)> {
        self.transitions.clone()
    }
}

impl<Place> Petri<Place>
where
    Place: Clone + PartialEq + Eq + Hash + Ord,
{
    /// Get all unique places in the Petri net, sorted for deterministic ordering
    pub fn get_places_sorted(&self) -> Vec<Place> {
        let mut places = HashSet::default();

        // Collect places from initial marking
        for place in &self.initial_marking {
            places.insert(place.clone());
        }

        // Collect places from transitions
        for (input, output) in &self.transitions {
            for place in input {
                places.insert(place.clone());
            }
            for place in output {
                places.insert(place.clone());
            }
        }

        // Sort places to ensure deterministic ordering
        let mut places_vec: Vec<Place> = places.into_iter().collect();
        places_vec.sort();
        places_vec
    }
}

impl<Place> Petri<Place>
where
    Place: Clone + PartialEq + Eq + Hash + std::fmt::Display,
{
    /// Generate Graphviz DOT format for visualizing the Petri net
    pub fn to_graphviz(&self) -> String {
        let mut dot = String::from("digraph PetriNet {\n");
        dot.push_str("  // Graph settings\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [fontsize=10, fontname=\"Arial\"];\n");
        dot.push_str("  edge [fontsize=10];\n");
        dot.push_str("  bgcolor=white;\n\n");

        // Configure graph attributes for a more traditional look
        dot.push_str("  // Graph properties\n");
        dot.push_str("  graph [nodesep=0.7, ranksep=0.7];\n\n");

        // Define place nodes (circles)
        dot.push_str("  // Place nodes\n");
        dot.push_str("  node [shape=circle, width=0.7, height=0.7, fixedsize=true, style=filled, fillcolor=\"#E5F5FF\", color=\"#2080B0\", penwidth=1.2];\n");

        // Define transition nodes (rectangles/bars)
        dot.push_str("\n  // Transition nodes\n");
        dot.push_str("  node [shape=rect, width=0.5, height=0.2, fixedsize=true, style=filled, fillcolor=\"#404040\", fontcolor=white];\n");

        for (i, _) in self.transitions.iter().enumerate() {
            dot.push_str(&format!("  T_{} [label=\"t{}\", fontcolor=white];\n", i, i));
        }

        let places = self.get_places();
        // Count tokens per place
        let mut initial_count = HashMap::default();
        // Add zero to initial_count for each place
        for place in &places {
            initial_count.insert(place.clone(), 0);
        }
        for place in &self.initial_marking {
            *initial_count.entry(place.clone()).or_insert(0) += 1;
        }

        // Define places with tokens using direct Unicode in the label
        dot.push_str("\n  // Place marking with tokens\n");
        for (place, count) in &initial_count {
            // Using HTML labels for better control of token appearance
            let dots = if *count <= 5 {
                "● ".repeat(*count).trim().to_string()
            } else {
                format!("{}●", count)
            };
            // Escape special characters for GraphViz node ID
            let escaped_place_id = format!("P_{}", escape_for_graphviz_id(&format!("{}", place)));

            // Prepare HTML label with tokens
            let token_html = if *count > 0 {
                format!(
                    "<<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"0\"><TR><TD>{}</TD></TR><TR><TD><FONT POINT-SIZE=\"14\">{}</FONT></TD></TR></TABLE>>",
                    place, dots
                )
            } else {
                format!("\"{}\"", place)
            };

            dot.push_str(&format!(
                "  {} [label={}, fillcolor=\"#D0F0FF\", fontcolor=\"#000000\", fixedsize=false, style=\"filled,rounded\"];\n",
                escaped_place_id, token_html
            ));
        }

        // Count inputs and outputs for each place and transition
        // to determine if we need to show weights
        let mut input_counts = HashMap::default();
        let mut output_counts = HashMap::default();

        for (i, (input, output)) in self.transitions.iter().enumerate() {
            // Count inputs from each place to this transition
            for place in input {
                let key = (place.clone(), i);
                *input_counts.entry(key).or_insert(0) += 1;
            }

            // Count outputs from this transition to each place
            for place in output {
                let key = (i, place.clone());
                *output_counts.entry(key).or_insert(0) += 1;
            }
        }

        // Define transition edges with weights
        dot.push_str("\n  // Transition edges\n");
        for (i, (input, output)) in self.transitions.iter().enumerate() {
            // Process unique input places
            let mut unique_inputs = HashMap::default();
            for place in input {
                *unique_inputs.entry(place).or_insert(0) += 1;
            }

            // Connect input places to transition with weights if needed
            for (place, count) in unique_inputs {
                let escaped_place_id =
                    format!("P_{}", escape_for_graphviz_id(&format!("{}", place)));

                if count == 1 {
                    dot.push_str(&format!(
                        "  {} -> T_{} [arrowhead=normal, color=\"#404040\", penwidth=1.2];\n",
                        escaped_place_id, i
                    ));
                } else {
                    // Add weight label for multiple arcs
                    dot.push_str(&format!(
                        "  {} -> T_{} [label=\" {}\", fontsize=12, arrowhead=normal, color=\"#404040\", penwidth=1.2];\n",
                        escaped_place_id, i, count
                    ));
                }
            }

            // Process unique output places
            let mut unique_outputs = HashMap::default();
            for place in output {
                *unique_outputs.entry(place).or_insert(0) += 1;
            }

            // Connect transition to output places with weights if needed
            for (place, count) in unique_outputs {
                let escaped_place_id =
                    format!("P_{}", escape_for_graphviz_id(&format!("{}", place)));

                if count == 1 {
                    dot.push_str(&format!(
                        "  T_{} -> {} [arrowhead=normal, color=\"#404040\", penwidth=1.2];\n",
                        i, escaped_place_id
                    ));
                } else {
                    // Add weight label for multiple arcs
                    dot.push_str(&format!(
                        "  T_{} -> {} [label=\" {}\", fontsize=12, arrowhead=normal, color=\"#404040\", penwidth=1.2];\n",
                        i, escaped_place_id, count
                    ));
                }
            }
        }

        // Close the graph
        dot.push_str("}\n");

        dot
    }

    /// Save GraphViz DOT representation of the Petri net
    pub fn save_graphviz(&self, name: &str, open_files: bool) -> Result<Vec<String>, String> {
        let dot_content = self.to_graphviz();
        graphviz::save_graphviz(&dot_content, name, "petri", open_files)
    }
}

impl<P> Petri<P> {
    /// Run an operation on each place
    pub fn for_each_place(&self, mut f: impl for<'a> FnMut(&'a P)) {
        self.initial_marking.iter().for_each(&mut f);
        for (from, to) in &self.transitions {
            from.iter().for_each(&mut f);
            to.iter().for_each(&mut f);
        }
    }

    /// Rename all the places
    pub fn rename<Q>(self, mut f: impl FnMut(P) -> Q) -> Petri<Q> {
        Petri {
            initial_marking: self.initial_marking.into_iter().map(&mut f).collect(),
            transitions: self
                .transitions
                .into_iter()
                .map(|(from, to)| {
                    (
                        from.into_iter().map(&mut f).collect(),
                        to.into_iter().map(&mut f).collect(),
                    )
                })
                .collect(),
        }
    }
}

impl<P: Clone + PartialEq + Eq + Hash> Petri<P> {
    /// Add transitions to make arbitrarily many markings in the given place
    pub fn add_existential_place(&mut self, place: P) {
        self.add_transition(vec![], vec![place]);
    }
}

impl<Place> Petri<Place>
where
    Place: Clone + PartialEq + Eq + Hash + std::fmt::Debug,
{
    /// Remove transitions where input places are exactly the same as output places
    pub fn remove_identity_transitions(&mut self) {
        self.transitions.retain(|(input, output)| input != output);
    }

    /// Filter the Petri net to keep only reachable transitions using a worklist algorithm
    ///
    /// Takes a list of initially reachable places and modifies the Petri net by:
    /// - Finding all transitions that can fire (all preconditions are reachable)
    /// - Removing all unreachable transitions from self.transitions
    /// - Removing unreachable places from self.initial_marking
    ///
    /// A transition is reachable (can fire) if all its precondition places are reachable.
    /// When a transition can fire, all its postcondition places become reachable.
    ///
    /// Returns a list of places that were removed (initial list of places minus final list of places).
    pub fn filter_reachable(&mut self, initial_places: &[Place]) -> Vec<Place> {
        // Get all places that appear in the Petri net before filtering
        let all_places_before: HashSet<Place> = self.get_places().into_iter().collect();

        self.remove_identity_transitions();

        let mut reachable_places: HashSet<Place> = initial_places.iter().cloned().collect();
        let mut reachable_transitions: HashSet<usize> = HashSet::default();
        let mut worklist: Vec<Place> = initial_places.to_vec();

        while let Some(_current_place) = worklist.pop() {
            // Check all transitions to see if any can now fire
            for (transition_idx, (inputs, outputs)) in self.transitions.iter().enumerate() {
                // Skip if we've already processed this transition
                if reachable_transitions.contains(&transition_idx) {
                    continue;
                }

                // Check if all input places of this transition are reachable
                let can_fire = inputs
                    .iter()
                    .all(|input_place| reachable_places.contains(input_place));

                if can_fire {
                    // Mark this transition as reachable (can fire)
                    reachable_transitions.insert(transition_idx);

                    // Add all output places to reachable set and worklist
                    for output_place in outputs {
                        if !reachable_places.contains(output_place) {
                            reachable_places.insert(output_place.clone());
                            worklist.push(output_place.clone());
                        }
                    }
                }
            }
        }

        // Filter transitions to keep only reachable ones
        self.transitions = self
            .transitions
            .iter()
            .enumerate()
            .filter(|(idx, _)| reachable_transitions.contains(idx))
            .map(|(_, transition)| transition.clone())
            .collect();

        // Filter initial marking to keep only places that still exist in the net
        self.initial_marking
            .retain(|place| reachable_places.contains(place));

        // Get all places that remain after filtering transitions
        let all_places_after: HashSet<Place> = self.get_places().into_iter().collect();

        let all_places_after_plus_initial: HashSet<Place> = all_places_after
            .clone()
            .into_iter()
            .chain(initial_places.iter().cloned())
            .collect();

        assert_eq!(reachable_places, all_places_after_plus_initial);

        // Calculate removed places: places that were in the net before but not after
        let removed_places: Vec<Place> = all_places_before
            .difference(&all_places_after)
            .cloned()
            .collect();

        removed_places
    }

    /// Filter the Petri net to keep only transitions reachable from the initial marking
    pub fn filter_reachable_from_initial(&mut self) -> Vec<Place> {
        let initial_marking = self.initial_marking.clone();
        self.filter_reachable(&initial_marking)
    }

    /// Flip the Petri net by reversing all transitions (input becomes output, output becomes input)
    /// This is useful for backwards reachability analysis
    pub fn flip(&mut self) {
        for (inputs, outputs) in &mut self.transitions {
            std::mem::swap(inputs, outputs);
        }
    }

    /// Filter the Petri net to keep only transitions that can reach the given target places
    /// This performs backwards reachability analysis by:
    /// 1. Flipping the net (reversing all transitions)
    /// 2. Running forward reachability from target places
    /// 3. Flipping back to original orientation
    ///
    /// Iteratively filter the Petri net using alternating forward and backward reachability
    /// Returns two vectors of transitions:
    /// 1. `removed_forward` — transitions deleted in the forward filtering steps
    /// 2. `removed_backward` — transitions deleted in the backward filtering steps
    pub fn filter_backwards_reachable(&mut self, target_places: &[Place]) -> Vec<Place> {
        // Step 1: Flip the net
        self.flip();

        // Step 2: Run forward reachability from target places
        let places = self.filter_reachable(target_places);

        // Step 3: Flip back to original orientation
        self.flip();

        places
    }

    /// Iteratively filter the Petri net using alternating forward and backward reachability
    /// until a fixed point is reached.
    ///
    /// This finds the minimal set of transitions that are:
    /// 1. Reachable from the initial marking (forward reachability)
    /// 2. Can reach the target places (backward reachability)
    ///
    /// The algorithm alternates between these filters until no more transitions are removed.
    pub fn filter_bidirectional_reachable(
        &mut self,
        target_places: &[Place],
    ) -> (Vec<(Vec<Place>, Vec<Place>)>, Vec<(Vec<Place>, Vec<Place>)>) {
        // If the user passed --without-bidirectional, skip the entire pruning step
        if !crate::reachability::optimize_enabled() {
            return (Vec::new(), Vec::new());
        }

        // track which transitions each pass deletes
        let mut removed_forward = Vec::new();
        let mut removed_backward = Vec::new();

        let initial_places = self.initial_marking.clone();
        let mut previous_count = self.transitions.len();
        let mut iteration = 0;

        loop {
            iteration += 1;

            // Step 1: Filter forward from initial marking
            let before_forward = self.transitions.clone();
            self.filter_reachable(&initial_places);
            for tr in before_forward
                .into_iter()
                .filter(|tr| !self.transitions.contains(tr))
            {
                removed_forward.push(tr);
            }

            // Step 2: Filter backward from target places
            let before_backward = self.transitions.clone();
            self.filter_backwards_reachable(target_places);
            for tr in before_backward
                .into_iter()
                .filter(|tr| !self.transitions.contains(tr))
            {
                removed_backward.push(tr);
            }
            let after_backward = self.transitions.len();

            // Check if we've reached a fixed point (no changes)
            if after_backward == previous_count {
                // No transitions were removed in this iteration
                break;
            }

            previous_count = after_backward;

            // Safety check to prevent infinite loops (shouldn't be needed)
            if iteration > 100 {
                eprintln!("Warning: Bidirectional filtering exceeded 100 iterations");
                break;
            }
        }
        (removed_forward, removed_backward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_reachable() {
        // Create a simple Petri net: P0 -> P1 -> P2, with P3 isolated
        let mut petri = Petri::new(vec!["P0"]);
        petri.add_transition(vec!["P0"], vec!["P1"]); // t0: P0 -> P1
        petri.add_transition(vec!["P1"], vec!["P2"]); // t1: P1 -> P2
        petri.add_transition(vec!["P3"], vec!["P4"]); // t2: P3 -> P4 (unreachable)

        // Before filtering: should have 3 transitions
        assert_eq!(petri.transitions.len(), 3);

        petri.filter_reachable_from_initial();

        // After filtering: should have only 2 reachable transitions (t0 and t1)
        assert_eq!(petri.transitions.len(), 2);
        assert_eq!(petri.transitions[0], (vec!["P0"], vec!["P1"])); // t0
        assert_eq!(petri.transitions[1], (vec!["P1"], vec!["P2"])); // t1
        // t2 (P3 -> P4) should be removed
    }

    #[test]
    fn test_filter_reachable_complex() {
        // More complex net: requires multiple places to fire transition
        let mut petri = Petri::new(vec!["A", "B"]);
        petri.add_transition(vec!["A"], vec!["C"]); // t0: A -> C
        petri.add_transition(vec!["B"], vec!["D"]); // t1: B -> D  
        petri.add_transition(vec!["C", "D"], vec!["E"]); // t2: C+D -> E (needs both C and D)
        petri.add_transition(vec!["F"], vec!["G"]); // t3: F -> G (unreachable, F not reachable)

        // Before filtering: should have 4 transitions
        assert_eq!(petri.transitions.len(), 4);

        petri.filter_reachable_from_initial();

        // After filtering: should have only 3 reachable transitions (t0, t1, t2)
        assert_eq!(petri.transitions.len(), 3);
        assert_eq!(petri.transitions[0], (vec!["A"], vec!["C"])); // t0
        assert_eq!(petri.transitions[1], (vec!["B"], vec!["D"])); // t1
        assert_eq!(petri.transitions[2], (vec!["C", "D"], vec!["E"])); // t2
        // t3 (F -> G) should be removed
    }

    #[test]
    fn test_filter_reachable_with_custom_initial() {
        // Test with custom initial places instead of initial marking
        let mut petri = Petri::new(vec!["P0"]); // P0 in initial marking
        petri.add_transition(vec!["P0"], vec!["P1"]); // t0: P0 -> P1
        petri.add_transition(vec!["P2"], vec!["P3"]); // t1: P2 -> P3

        // Before filtering: should have 2 transitions
        assert_eq!(petri.transitions.len(), 2);

        // Filter reachable from custom set including P2 (not in initial marking)
        petri.filter_reachable(&["P2"]);

        // After filtering: should have only 1 reachable transition (t1)
        assert_eq!(petri.transitions.len(), 1);
        assert_eq!(petri.transitions[0], (vec!["P2"], vec!["P3"])); // t1
        // t0 (P0 -> P1) should be removed since P0 is not in custom initial set
    }

    #[test]
    fn test_filter_reachable_usage_example() {
        // Example showing how to use reachability filtering to remove unreachable parts
        let mut petri = Petri::new(vec!["Start", "Resource"]);
        petri.add_transition(vec!["Start"], vec!["Process1"]); // t0
        petri.add_transition(vec!["Process1", "Resource"], vec!["Process2"]); // t1  
        petri.add_transition(vec!["Process2"], vec!["End", "Resource"]); // t2
        petri.add_transition(vec!["Unreachable"], vec!["AlsoUnreachable"]); // t3

        // Before filtering: 4 transitions
        assert_eq!(petri.transitions.len(), 4);

        petri.filter_reachable_from_initial();

        // After filtering: only 3 reachable transitions remain (t0, t1, t2)
        assert_eq!(petri.transitions.len(), 3);
        assert_eq!(petri.transitions[0], (vec!["Start"], vec!["Process1"]));
        assert_eq!(
            petri.transitions[1],
            (vec!["Process1", "Resource"], vec!["Process2"])
        );
        assert_eq!(
            petri.transitions[2],
            (vec!["Process2"], vec!["End", "Resource"])
        );
        // t3 (Unreachable -> AlsoUnreachable) should be removed
    }

    #[test]
    fn test_flip() {
        let mut petri = Petri::new(vec!["P0"]);
        petri.add_transition(vec!["P0"], vec!["P1"]); // t0: P0 -> P1
        petri.add_transition(vec!["P1", "P2"], vec!["P3"]); // t1: P1+P2 -> P3

        // Before flip
        assert_eq!(petri.transitions[0], (vec!["P0"], vec!["P1"]));
        assert_eq!(petri.transitions[1], (vec!["P1", "P2"], vec!["P3"]));

        petri.flip();

        // After flip: inputs and outputs should be swapped
        assert_eq!(petri.transitions[0], (vec!["P1"], vec!["P0"])); // was P0 -> P1, now P1 -> P0
        assert_eq!(petri.transitions[1], (vec!["P3"], vec!["P1", "P2"])); // was P1+P2 -> P3, now P3 -> P1+P2

        // Flip back
        petri.flip();

        // Should be back to original
        assert_eq!(petri.transitions[0], (vec!["P0"], vec!["P1"]));
        assert_eq!(petri.transitions[1], (vec!["P1", "P2"], vec!["P3"]));
    }

    #[test]
    fn test_filter_backwards_reachable() {
        // Create a linear chain: P0 -> P1 -> P2 -> P3, with unreachable branch P4 -> P5
        let mut petri = Petri::new(vec!["P0"]);
        petri.add_transition(vec!["P0"], vec!["P1"]); // t0: P0 -> P1
        petri.add_transition(vec!["P1"], vec!["P2"]); // t1: P1 -> P2
        petri.add_transition(vec!["P2"], vec!["P3"]); // t2: P2 -> P3
        petri.add_transition(vec!["P4"], vec!["P5"]); // t3: P4 -> P5 (unreachable from target)

        // Before filtering: 4 transitions
        assert_eq!(petri.transitions.len(), 4);

        // Filter backwards reachable to P2 (only transitions that can lead to P2)
        petri.filter_backwards_reachable(&["P2"]);

        // After filtering: should keep t0 and t1 (can reach P2), remove t2 and t3
        assert_eq!(petri.transitions.len(), 2);
        assert_eq!(petri.transitions[0], (vec!["P0"], vec!["P1"])); // t0: P0 -> P1 (can reach P2)
        assert_eq!(petri.transitions[1], (vec!["P1"], vec!["P2"])); // t1: P1 -> P2 (can reach P2)
        // t2 (P2 -> P3) removed because it doesn't lead TO P2
        // t3 (P4 -> P5) removed because it can't reach P2
    }

    #[test]
    fn test_filter_backwards_reachable_complex() {
        // More complex backwards reachability: multiple paths to target
        let mut petri = Petri::new(vec!["A", "B"]);
        petri.add_transition(vec!["A"], vec!["C"]); // t0: A -> C (leads to target E)
        petri.add_transition(vec!["B"], vec!["D"]); // t1: B -> D (leads to target E)
        petri.add_transition(vec!["C", "D"], vec!["E"]); // t2: C+D -> E (creates target E)
        petri.add_transition(vec!["E"], vec!["F"]); // t3: E -> F (doesn't lead TO E)
        petri.add_transition(vec!["X"], vec!["Y"]); // t4: X -> Y (unconnected, can't reach E)

        // Before filtering: 5 transitions
        assert_eq!(petri.transitions.len(), 5);

        // Filter backwards reachable to E (transitions that can lead to E)
        petri.filter_backwards_reachable(&["E"]);

        // Should keep t0, t1, t2 (all can lead to E), remove t3, t4
        assert_eq!(petri.transitions.len(), 3);
        assert_eq!(petri.transitions[0], (vec!["A"], vec!["C"])); // t0: can contribute to E
        assert_eq!(petri.transitions[1], (vec!["B"], vec!["D"])); // t1: can contribute to E  
        assert_eq!(petri.transitions[2], (vec!["C", "D"], vec!["E"])); // t2: directly creates E
        // t3 (E -> F) removed: doesn't lead TO E
        // t4 (X -> Y) removed: unconnected
    }

    #[test]
    fn test_backwards_vs_forwards_filtering_example() {
        // Demonstrate the difference between forward and backward reachability
        let mut petri_forward = Petri::new(vec!["Start"]);
        petri_forward.add_transition(vec!["Start"], vec!["Middle"]); // t0: Start -> Middle
        petri_forward.add_transition(vec!["Middle"], vec!["End"]); // t1: Middle -> End  
        petri_forward.add_transition(vec!["End"], vec!["Cleanup"]); // t2: End -> Cleanup
        petri_forward.add_transition(vec!["Isolated"], vec!["Nowhere"]); // t3: Isolated -> Nowhere

        let mut petri_backward = petri_forward.clone();

        println!("Original net: Start->Middle->End->Cleanup, Isolated->Nowhere");
        assert_eq!(petri_forward.transitions.len(), 4);

        // Forward reachability from "Start"
        petri_forward.filter_reachable(&["Start"]);
        println!(
            "Forward from 'Start': {} transitions remain",
            petri_forward.transitions.len()
        );
        assert_eq!(petri_forward.transitions.len(), 3); // Keep t0, t1, t2; remove t3

        // Backward reachability to "End"
        petri_backward.filter_backwards_reachable(&["End"]);
        println!(
            "Backward to 'End': {} transitions remain",
            petri_backward.transitions.len()
        );
        assert_eq!(petri_backward.transitions.len(), 2); // Keep t0, t1; remove t2, t3

        // Forward: transitions reachable FROM start
        assert_eq!(
            petri_forward.transitions[0],
            (vec!["Start"], vec!["Middle"])
        );
        assert_eq!(petri_forward.transitions[1], (vec!["Middle"], vec!["End"]));
        assert_eq!(petri_forward.transitions[2], (vec!["End"], vec!["Cleanup"]));

        // Backward: transitions that can reach TO end
        assert_eq!(
            petri_backward.transitions[0],
            (vec!["Start"], vec!["Middle"])
        );
        assert_eq!(petri_backward.transitions[1], (vec!["Middle"], vec!["End"]));
        // End->Cleanup removed because it doesn't lead TO End
    }

    #[test]
    fn test_filter_bidirectional_reachable_simple() {
        // Simple case: linear chain with extra branches
        let mut petri = Petri::new(vec!["Start"]);
        petri.add_transition(vec!["Start"], vec!["A"]); // t0: Start -> A (needed)
        petri.add_transition(vec!["A"], vec!["Target"]); // t1: A -> Target (needed)
        petri.add_transition(vec!["Target"], vec!["After"]); // t2: Target -> After (not needed for Target)
        petri.add_transition(vec!["Isolated"], vec!["B"]); // t3: Isolated -> B (unreachable from Start)

        // Before filtering: 4 transitions
        assert_eq!(petri.transitions.len(), 4);

        // Bidirectional filter to Target
        petri.filter_bidirectional_reachable(&["Target"]);

        // Should keep only t0 and t1 (path from Start to Target)
        assert_eq!(petri.transitions.len(), 2);
        assert_eq!(petri.transitions[0], (vec!["Start"], vec!["A"])); // t0: needed for path
        assert_eq!(petri.transitions[1], (vec!["A"], vec!["Target"])); // t1: needed for path
        // t2 removed: Target->After doesn't help reach Target
        // t3 removed: Isolated->B unreachable from Start
    }

    #[test]
    fn test_filter_bidirectional_reachable_complex() {
        // More complex case requiring multiple iterations
        let mut petri = Petri::new(vec!["Start"]);
        petri.add_transition(vec!["Start"], vec!["A"]); // t0: Start -> A (needed)
        petri.add_transition(vec!["A"], vec!["B"]); // t1: A -> B (needed)
        petri.add_transition(vec!["B"], vec!["Target"]); // t2: B -> Target (needed)
        petri.add_transition(vec!["A"], vec!["C"]); // t3: A -> C (dead end, not needed)
        petri.add_transition(vec!["C"], vec!["D"]); // t4: C -> D (dead end, not needed)
        petri.add_transition(vec!["Target"], vec!["E"]); // t5: Target -> E (not needed for Target)
        petri.add_transition(vec!["Unreachable"], vec!["F"]); // t6: Unreachable -> F (isolated)

        // Before filtering: 7 transitions
        assert_eq!(petri.transitions.len(), 7);

        // Bidirectional filter to Target
        petri.filter_bidirectional_reachable(&["Target"]);

        // Should keep only the direct path: Start -> A -> B -> Target
        assert_eq!(petri.transitions.len(), 3);
        assert_eq!(petri.transitions[0], (vec!["Start"], vec!["A"])); // t0: needed
        assert_eq!(petri.transitions[1], (vec!["A"], vec!["B"])); // t1: needed
        assert_eq!(petri.transitions[2], (vec!["B"], vec!["Target"])); // t2: needed
        // t3, t4 removed: A->C->D is a dead end that doesn't reach Target
        // t5 removed: Target->E doesn't help reach Target
        // t6 removed: Unreachable->F is isolated from Start
    }

    #[test]
    fn test_filter_bidirectional_reachable_multiple_targets() {
        // Test with multiple target places
        let mut petri = Petri::new(vec!["Start"]);
        petri.add_transition(vec!["Start"], vec!["A"]); // t0: Start -> A (needed for both targets)
        petri.add_transition(vec!["A"], vec!["Target1"]); // t1: A -> Target1 (needed)
        petri.add_transition(vec!["A"], vec!["Target2"]); // t2: A -> Target2 (needed)
        petri.add_transition(vec!["Target1"], vec!["B"]); // t3: Target1 -> B (not needed)
        petri.add_transition(vec!["Target2"], vec!["C"]); // t4: Target2 -> C (not needed)
        petri.add_transition(vec!["Isolated"], vec!["D"]); // t5: Isolated -> D (unreachable)

        // Before filtering: 6 transitions
        assert_eq!(petri.transitions.len(), 6);

        // Bidirectional filter to both targets
        petri.filter_bidirectional_reachable(&["Target1", "Target2"]);

        // Should keep path to both targets: Start -> A -> {Target1, Target2}
        assert_eq!(petri.transitions.len(), 3);
        assert_eq!(petri.transitions[0], (vec!["Start"], vec!["A"])); // t0: needed for both
        assert_eq!(petri.transitions[1], (vec!["A"], vec!["Target1"])); // t1: needed for Target1
        assert_eq!(petri.transitions[2], (vec!["A"], vec!["Target2"])); // t2: needed for Target2
        // t3, t4 removed: don't help reach the targets
        // t5 removed: isolated from Start
    }

    #[test]
    fn test_filter_bidirectional_reachable_convergence() {
        // Test case that requires multiple iterations to converge
        let mut petri = Petri::new(vec!["Start"]);
        petri.add_transition(vec!["Start"], vec!["A"]); // t0: Start -> A (needed)
        petri.add_transition(vec!["A"], vec!["B"]); // t1: A -> B (needed)
        petri.add_transition(vec!["B"], vec!["C"]); // t2: B -> C (needed)
        petri.add_transition(vec!["C"], vec!["Target"]); // t3: C -> Target (needed)
        petri.add_transition(vec!["A"], vec!["X"]); // t4: A -> X (initially seems reachable)
        petri.add_transition(vec!["X"], vec!["Y"]); // t5: X -> Y (depends on t4)
        petri.add_transition(vec!["Y"], vec!["Z"]); // t6: Y -> Z (depends on t5, doesn't reach Target)

        // Before filtering: 7 transitions
        assert_eq!(petri.transitions.len(), 7);

        // Bidirectional filter to Target
        petri.filter_bidirectional_reachable(&["Target"]);

        // Should eliminate the X->Y->Z branch since it doesn't reach Target
        assert_eq!(petri.transitions.len(), 4);
        assert_eq!(petri.transitions[0], (vec!["Start"], vec!["A"])); // t0: needed
        assert_eq!(petri.transitions[1], (vec!["A"], vec!["B"])); // t1: needed  
        assert_eq!(petri.transitions[2], (vec!["B"], vec!["C"])); // t2: needed
        assert_eq!(petri.transitions[3], (vec!["C"], vec!["Target"])); // t3: needed
        // t4, t5, t6 removed: A->X->Y->Z doesn't contribute to reaching Target
    }

    #[test]
    fn test_bidirectional_vs_individual_filtering_demo() {
        // Demonstrate the power of bidirectional filtering vs individual approaches
        println!("\n=== Bidirectional vs Individual Filtering Demo ===");

        // Create a complex network with multiple branches
        let mut petri_original = Petri::new(vec!["Start"]);
        petri_original.add_transition(vec!["Start"], vec!["A"]); // t0: Start -> A (essential)
        petri_original.add_transition(vec!["A"], vec!["B"]); // t1: A -> B (essential)  
        petri_original.add_transition(vec!["B"], vec!["Target"]); // t2: B -> Target (essential)
        petri_original.add_transition(vec!["A"], vec!["DeadEnd"]); // t3: A -> DeadEnd (forward reachable, but useless)
        petri_original.add_transition(vec!["Target"], vec!["After"]); // t4: Target -> After (backward unreachable)
        petri_original.add_transition(vec!["Isolated"], vec!["Nowhere"]); // t5: Isolated -> Nowhere (completely unreachable)

        println!(
            "Original net has {} transitions",
            petri_original.transitions.len()
        );
        assert_eq!(petri_original.transitions.len(), 6);

        // Test 1: Forward-only filtering
        let mut petri_forward = petri_original.clone();
        let _removed_places = petri_forward.filter_reachable_from_initial();
        println!(
            "Forward-only filtering: {} transitions remain",
            petri_forward.transitions.len()
        );
        assert_eq!(petri_forward.transitions.len(), 5); // Removes only t5 (isolated), keeps everything else

        // Test 2: Backward-only filtering
        let mut petri_backward = petri_original.clone();
        petri_backward.filter_backwards_reachable(&["Target"]);
        println!(
            "Backward-only filtering: {} transitions remain",
            petri_backward.transitions.len()
        );
        assert_eq!(petri_backward.transitions.len(), 3); // Removes t4, t5; keeps t3

        // Test 3: Bidirectional filtering (the optimal result)
        let mut petri_bidirectional = petri_original.clone();
        petri_bidirectional.filter_bidirectional_reachable(&["Target"]);
        println!(
            "Bidirectional filtering: {} transitions remain",
            petri_bidirectional.transitions.len()
        );
        assert_eq!(petri_bidirectional.transitions.len(), 3); // Keeps only essential path: t0, t1, t2

        // Verify the bidirectional result is optimal
        assert_eq!(
            petri_bidirectional.transitions[0],
            (vec!["Start"], vec!["A"])
        );
        assert_eq!(petri_bidirectional.transitions[1], (vec!["A"], vec!["B"]));
        assert_eq!(
            petri_bidirectional.transitions[2],
            (vec!["B"], vec!["Target"])
        );

        println!("✓ Bidirectional filtering found the minimal essential path!");
        println!("  Forward-only: kept dead-end branches reachable from Start");
        println!("  Backward-only: kept irrelevant transitions after Target");
        println!("  Bidirectional: kept only Start → A → B → Target");
    }

    #[test]
    fn test_bidirectional_pruning_print_names() {
        use crate::petri::Petri;

        // 1) Build the net with initial marking only on "P1"
        let mut petri = Petri::new(vec!["P1","P2"]);

        // 2) Define our transitions along with their names
        let named: Vec<(&str, Vec<&str>, Vec<&str>)> = vec![
            ("t0", vec![],                          vec!["P0"]),
            ("t1", vec!["P0", "P1", "P6"],          vec!["P5"]),
            ("t2", vec!["P1", "P2"],                vec!["P3"]),
            ("t3", vec!["P5"],                 vec!["P6", "P7"]),
            ("t4", vec!["P3"],                      vec!["P4", "P6"]),
            ("t5", vec!["P4", "P8", "P9"],          vec!["P8"]),
        ];

        // 3) Add them in order
        for (_name, input, output) in &named {
            petri.add_transition(input.clone(), output.clone());
        }

        let initial = petri.get_initial_marking(); // {"P1"}
        // let targets = petri.get_places();          // all places
        let targets: Vec<&str> = vec!["P0", "P5"];             // we only care about reaching P3

        let mut iteration = 0;
        loop {
            iteration += 1;

            // --- forward pruning ---
            let before_tr = petri.get_transitions();
            let before_pl = petri.get_places();
            petri.filter_reachable(&initial);
            let after_tr  = petri.get_transitions();
            let after_pl  = petri.get_places();

            // compute which transitions fell out
            let removed_tr_f: Vec<&str> = before_tr.iter()
                .filter(|tr| !after_tr.contains(tr))
                .filter_map(|tr| {
                    named.iter()
                        .find(|(_, inp, out)| &tr.0 == inp && &tr.1 == out)
                        .map(|(name, _, _)| *name)
                })
                .collect();

            // compute which places fell out
            let removed_pl_f: Vec<&str> = before_pl.iter()
                .filter(|p| !after_pl.contains(*p))
                .cloned()
                .collect();

            // build owned strings for printing
            let f_tr_str = if removed_tr_f.is_empty() {
                "(none)".to_string()
            } else {
                removed_tr_f.join(", ")
            };
            let f_pl_str = if removed_pl_f.is_empty() {
                "(none)".to_string()
            } else {
                removed_pl_f.join(", ")
            };

            println!(
                "Iteration {} forward removed:\n  transitions: {}\n  places:      {}",
                iteration,
                f_tr_str,
                f_pl_str
            );

            // --- backward pruning ---
            let before_tr = petri.get_transitions();
            let before_pl = petri.get_places();
            petri.filter_backwards_reachable(&targets);
            let after_tr  = petri.get_transitions();
            let after_pl  = petri.get_places();

            let removed_tr_b: Vec<&str> = before_tr.iter()
                .filter(|tr| !after_tr.contains(tr))
                .filter_map(|tr| {
                    named.iter()
                        .find(|(_, inp, out)| &tr.0 == inp && &tr.1 == out)
                        .map(|(name, _, _)| *name)
                })
                .collect();

            let removed_pl_b: Vec<&str> = before_pl.iter()
                .filter(|p| !after_pl.contains(*p))
                .cloned()
                .collect();

            let b_tr_str = if removed_tr_b.is_empty() {
                "(none)".to_string()
            } else {
                removed_tr_b.join(", ")
            };
            let b_pl_str = if removed_pl_b.is_empty() {
                "(none)".to_string()
            } else {
                removed_pl_b.join(", ")
            };

            println!(
                "Iteration {} backward removed:\n  transitions: {}\n  places:      {}",
                iteration,
                b_tr_str,
                b_pl_str
            );

            // fixed‐point check
            if removed_tr_f.is_empty() && removed_pl_f.is_empty()
                && removed_tr_b.is_empty() && removed_pl_b.is_empty() {
                break;
            }
        }

        // show what remains
        let remaining: Vec<&str> = petri.get_transitions().iter()
            .filter_map(|tr| {
                named.iter()
                    .find(|(_, inp, out)| &tr.0 == inp && &tr.1 == out)
                    .map(|(name, _, _)| *name)
            })
            .collect();
        println!("Final (pruned) transitions: {}", remaining.join(", "));
    }



}
