//! HTML debugging report generation for serializability analysis

use crate::presburger::Constraint;
use crate::size_logger::{SemilinearStats, log_semilinear_size_csv};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SmptCall {
    pub disjunct_id: usize,
    pub petri_net_content: String,
    pub xml_content: String,
    pub result: String,
    pub execution_time_ms: Option<u64>,
    pub constraints_description: String,
}

#[derive(Debug, Clone)]
pub struct AlgorithmStep {
    pub step_name: String,
    pub description: String,
    pub details: String,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct DebugReport {
    pub program_name: String,
    pub program_content: String,
    pub algorithm_steps: Vec<AlgorithmStep>,
    pub smpt_calls: Vec<SmptCall>,
    pub final_result: String,
    pub total_execution_time_ms: u64,
}

impl DebugReport {
    pub fn new(program_name: String, program_content: String) -> Self {
        Self {
            program_name,
            program_content,
            algorithm_steps: Vec::new(),
            smpt_calls: Vec::new(),
            final_result: String::new(),
            total_execution_time_ms: 0,
        }
    }

    pub fn add_step(&mut self, step_name: String, description: String, details: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        self.algorithm_steps.push(AlgorithmStep {
            step_name,
            description,
            details,
            timestamp,
        });
    }

    pub fn add_smpt_call(&mut self, call: SmptCall) {
        self.smpt_calls.push(call);
    }

    pub fn set_final_result(&mut self, result: String, total_time_ms: u64) {
        self.final_result = result;
        self.total_execution_time_ms = total_time_ms;
    }

    pub fn generate_html(&self, output_path: &str) -> Result<(), std::io::Error> {
        let html = self.render_html();
        std::fs::write(output_path, html)?;
        println!("Debug report generated: {}", output_path);
        Ok(())
    }

    fn render_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Serializability Analysis Debug Report - {}</title>
    <style>
        body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 20px; background-color: #f5f5f5; }}
        .container {{ max-width: 1200px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        h1, h2, h3 {{ color: #333; }}
        .summary {{ background-color: #e8f4fd; padding: 15px; border-radius: 5px; margin-bottom: 20px; border-left: 4px solid #2196F3; }}
        .step {{ background-color: #f9f9f9; margin: 10px 0; padding: 15px; border-radius: 5px; border-left: 3px solid #4CAF50; }}
        .smpt-call {{ background-color: #fff3cd; margin: 15px 0; padding: 15px; border-radius: 5px; border-left: 3px solid #ffc107; }}
        .code-block {{ background-color: #f4f4f4; padding: 10px; border-radius: 3px; font-family: 'Courier New', monospace; white-space: pre-wrap; overflow-x: auto; margin: 10px 0; }}
        .xml-content {{ background-color: #f0f8ff; border: 1px solid #b0d4f1; }}
        .petri-content {{ background-color: #f0fff0; border: 1px solid #90ee90; }}
        .constraint-content {{ background-color: #fff5ee; border: 1px solid #ffa500; }}
        .result-success {{ color: #28a745; font-weight: bold; }}
        .result-failure {{ color: #dc3545; font-weight: bold; }}
        .result-reachable {{ color: #ffc107; font-weight: bold; }}
        .timestamp {{ color: #666; font-size: 0.9em; }}
        .execution-time {{ color: #007bff; font-weight: bold; }}
        .section {{ margin: 20px 0; }}
        table {{ width: 100%; border-collapse: collapse; margin: 10px 0; }}
        th, td {{ padding: 8px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #f2f2f2; }}
        .stats {{ display: flex; justify-content: space-around; margin: 20px 0; }}
        .stat-box {{ background-color: #e3f2fd; padding: 15px; border-radius: 5px; text-align: center; min-width: 120px; }}
        .collapsible {{ background-color: #777; color: white; cursor: pointer; padding: 10px; width: 100%; border: none; text-align: left; outline: none; font-size: 15px; margin: 5px 0; }}
        .collapsible:hover {{ background-color: #555; }}
        .content {{ padding: 0 15px; display: none; overflow: hidden; background-color: #f1f1f1; }}
        .content.show {{ display: block; }}
    </style>
    <script>
        function toggleContent(button) {{
            button.classList.toggle("active");
            var content = button.nextElementSibling;
            if (content.style.display === "block") {{
                content.style.display = "none";
            }} else {{
                content.style.display = "block";
            }}
        }}
    </script>
</head>
<body>
    <div class="container">
        <h1>🔍 Serializability Analysis Debug Report</h1>
        
        <div class="summary">
            <h2>📋 Analysis Summary</h2>
            <p><strong>Program:</strong> {}</p>
            <p><strong>Final Result:</strong> <span class="{}">{}</span></p>
            <p><strong>Total Execution Time:</strong> <span class="execution-time">{} ms</span></p>
            <p><strong>SMPT Calls Made:</strong> {}</p>
            <p><strong>Algorithm Steps:</strong> {}</p>
        </div>

        <div class="stats">
            <div class="stat-box">
                <h3>📁 Program</h3>
                <p>{}</p>
            </div>
            <div class="stat-box">
                <h3>⏱️ Time</h3>
                <p>{} ms</p>
            </div>
            <div class="stat-box">
                <h3>🔧 SMPT Calls</h3>
                <p>{}</p>
            </div>
            <div class="stat-box">
                <h3>📊 Steps</h3>
                <p>{}</p>
            </div>
        </div>

        <div class="section">
            <h2>📄 Program Source</h2>
            <div class="code-block">{}</div>
        </div>

        <div class="section">
            <h2>🔄 Algorithm Execution Steps</h2>
            {}
        </div>

        <div class="section">
            <h2>🤖 SMPT Verification Calls</h2>
            {}
        </div>

        <div class="section">
            <h2>📈 Analysis Timeline</h2>
            <table>
                <tr><th>Time</th><th>Event</th><th>Description</th></tr>
                {}
            </table>
        </div>
    </div>
</body>
</html>"#,
            self.program_name,
            self.program_name,
            if self.final_result.contains("serializable") && !self.final_result.contains("Not") {
                "result-success"
            } else {
                "result-failure"
            },
            self.final_result,
            self.total_execution_time_ms,
            self.smpt_calls.len(),
            self.algorithm_steps.len(),
            self.program_name,
            self.total_execution_time_ms,
            self.smpt_calls.len(),
            self.algorithm_steps.len(),
            html_escape(&self.program_content),
            self.render_algorithm_steps(),
            self.render_smpt_calls(),
            self.render_timeline()
        )
    }

    fn render_algorithm_steps(&self) -> String {
        self.algorithm_steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                format!(
                    r#"<div class="step">
                        <h3>Step {}: {} <span class="timestamp">[{}]</span></h3>
                        <p><strong>Description:</strong> {}</p>
                        <div class="code-block">{}</div>
                    </div>"#,
                    i + 1,
                    html_escape(&step.step_name),
                    step.timestamp,
                    html_escape(&step.description),
                    html_escape(&step.details)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_smpt_calls(&self) -> String {
        self.smpt_calls
            .iter()
            .enumerate()
            .map(|(i, call)| {
                let result_class = match call.result.as_str() {
                    "REACHABLE" => "result-reachable",
                    "UNREACHABLE" => "result-success",
                    _ => "result-failure",
                };

                let time_info = call
                    .execution_time_ms
                    .map(|t| format!(" <span class=\"execution-time\">({} ms)</span>", t))
                    .unwrap_or_default();

                format!(
                    r#"<div class="smpt-call">
                        <h3>🤖 SMPT Call #{} - Disjunct {} <span class="{}">{}</span>{}</h3>
                        <p><strong>Constraints:</strong> {}</p>
                        
                        <h4>📋 Petri Net:</h4>
                        <div class="code-block petri-content">{}</div>
                        
                        <h4>🔧 XML Constraints:</h4>
                        <div class="code-block xml-content">{}</div>
                    </div>"#,
                    i + 1,
                    call.disjunct_id,
                    result_class,
                    call.result,
                    time_info,
                    html_escape(&call.constraints_description),
                    html_escape(&call.petri_net_content),
                    html_escape(&call.xml_content)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_timeline(&self) -> String {
        let mut timeline = Vec::new();

        // Add algorithm steps
        for step in &self.algorithm_steps {
            timeline.push(format!(
                "<tr><td>{}</td><td>📋 Algorithm Step</td><td>{}</td></tr>",
                step.timestamp,
                html_escape(&format!("{}: {}", step.step_name, step.description))
            ));
        }

        // Add SMPT calls (assuming they happen after algorithm steps)
        for (i, call) in self.smpt_calls.iter().enumerate() {
            timeline.push(format!(
                "<tr><td>--:--:--.---</td><td>🤖 SMPT Call #{}</td><td>Disjunct {} → {}</td></tr>",
                i + 1,
                call.disjunct_id,
                call.result
            ));
        }

        timeline.join("\n")
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Debug logger that can be passed through function calls
#[derive(Clone)]
pub struct DebugLogger {
    report: std::sync::Arc<std::sync::Mutex<DebugReport>>,
}

impl DebugLogger {
    pub fn new(program_name: String, program_content: String) -> Self {
        let report = DebugReport::new(program_name, program_content);
        Self {
            report: std::sync::Arc::new(std::sync::Mutex::new(report)),
        }
    }

    pub fn step(&self, step_name: &str, description: &str, details: &str) {
        if let Ok(mut report) = self.report.lock() {
            report.add_step(
                step_name.to_string(),
                description.to_string(),
                details.to_string(),
            );
        }
    }

    pub fn smpt_call(&self, call: SmptCall) {
        if let Ok(mut report) = self.report.lock() {
            report.add_smpt_call(call);
        }
    }

    pub fn finalize(
        &self,
        result: String,
        total_time_ms: u64,
        output_dir: &str,
    ) -> Result<(), std::io::Error> {
        if let Ok(mut report) = self.report.lock() {
            report.set_final_result(result, total_time_ms);
            let output_path = format!("{}/debug_report.html", output_dir);
            report.generate_html(&output_path)?;
        }
        Ok(())
    }

    // Helper methods for common debugging tasks
    pub fn log_petri_net<P: Clone + PartialEq + Eq + Hash + Display + Debug>(
        &self,
        name: &str,
        description: &str,
        petri: &crate::petri::Petri<P>,
    ) {
        let places = petri.get_places();
        let transitions = petri.get_transitions();
        let initial_marking = petri.get_initial_marking();

        // Create a more readable representation
        let mut details = String::new();

        // Places section
        details.push_str(&format!("📍 PLACES ({}):\n", places.len()));
        for (i, place) in places.iter().enumerate() {
            details.push_str(&format!("  {}. {}\n", i + 1, place));
        }

        // Initial marking section
        details.push_str(&format!(
            "\n🏁 INITIAL MARKING ({} tokens):\n",
            initial_marking.len()
        ));
        let mut token_counts = std::collections::HashMap::new();
        for place in &initial_marking {
            *token_counts.entry(format!("{}", place)).or_insert(0) += 1;
        }
        for (place, count) in token_counts {
            details.push_str(&format!(
                "  {} ← {} token{}\n",
                place,
                count,
                if count > 1 { "s" } else { "" }
            ));
        }

        // Transitions section
        details.push_str(&format!("\n🔄 TRANSITIONS ({}):\n", transitions.len()));
        for (i, (inputs, outputs)) in transitions.iter().enumerate() {
            let input_str = if inputs.is_empty() {
                "∅".to_string()
            } else {
                inputs
                    .iter()
                    .map(|p| format!("{}", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let output_str = if outputs.is_empty() {
                "∅".to_string()
            } else {
                outputs
                    .iter()
                    .map(|p| format!("{}", p))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            details.push_str(&format!("  t{}: [{}] → [{}]\n", i, input_str, output_str));
        }

        // Add summary
        details.push_str("\n📊 SUMMARY:\n");
        details.push_str(&format!(
            "  • {} places, {} transitions\n",
            places.len(),
            transitions.len()
        ));
        details.push_str(&format!("  • {} initial tokens\n", initial_marking.len()));
        let reachable_from_initial = transitions
            .iter()
            .filter(|(inputs, _)| inputs.is_empty())
            .count();
        details.push_str(&format!(
            "  • {} transition(s) enabled initially\n",
            reachable_from_initial
        ));

        self.step(name, description, &details);
    }

    pub fn log_semilinear_set<T: Eq + Hash + Clone + Ord + Debug + Display>(
        &self,
        name: &str,
        description: &str,
        set: &crate::semilinear::SemilinearSet<T>,
    ) {
        let mut details = String::new();
        details.push_str("🔢 SEMILINEAR SET:\n\n");

        // Use the existing pretty Display implementation
        details.push_str(&format!("📚 MATHEMATICAL FORM:\n{}\n\n", set));

        // Add some structural analysis
        let components = &set.components;
        details.push_str("📊 STRUCTURE ANALYSIS:\n");
        details.push_str(&format!(
            "  • {} linear set component{}\n",
            components.len(),
            if components.len() > 1 { "s" } else { "" }
        ));

        for (i, linear_set) in components.iter().enumerate() {
            details.push_str(&format!(
                "  • Component {}: {} period vector{}, base vector: {}\n",
                i + 1,
                linear_set.periods.len(),
                if linear_set.periods.len() != 1 {
                    "s"
                } else {
                    ""
                },
                if linear_set.base.values.is_empty() {
                    "∅"
                } else {
                    "non-empty"
                }
            ));
        }

        self.step(name, description, &details);
    }

    pub fn log_semilinear_set_for_optimization_comparison<
        T: Eq + Hash + Clone + Ord + Debug + Display,
    >(
        &self,
        program_name: String,
        name: &str,
        description: &str,
        set: &crate::semilinear::SemilinearSet<T>,
        out_dir: &Path,
    ) {
        let mut details = String::new();
        details.push_str("🔢 SEMILINEAR SET:\n\n");

        // Use the existing pretty Display implementation
        details.push_str(&format!("📚 MATHEMATICAL FORM:\n{}\n\n", set));

        // Add some structural analysis
        let components = &set.components;
        details.push_str("📊 STRUCTURE ANALYSIS:\n");
        details.push_str(&format!(
            "  • {} linear set component{}\n",
            components.len(),
            if components.len() > 1 { "s" } else { "" }
        ));

        for (i, linear_set) in components.iter().enumerate() {
            details.push_str(&format!(
                "  • Component {}: {} period vector{}, base vector: {}\n",
                i + 1,
                linear_set.periods.len(),
                if linear_set.periods.len() != 1 {
                    "s"
                } else {
                    ""
                },
                if linear_set.base.values.is_empty() {
                    "∅"
                } else {
                    "non-empty"
                }
            ));
        }

        // ##### CSV LOGGING START #####
        let stats = SemilinearStats {
            program_name,
            num_components: set.components.len(),
            periods_per_component: set.components.iter().map(|ls| ls.periods.len()).collect(),
        };
        // Write to <out_dir>/semilinear_size_stats.csv
        let csv_path = out_dir.join("semilinear_size_stats.csv"); // #### UPDATE: build path from out_dir
        log_semilinear_size_csv(&csv_path, &stats).expect("Failed to write semilinear_size_stats.csv"); // #### UPDATE: write stats

        // ##### CSV LOGGING END #####

        self.step(name, description, &details);
    }

    pub fn log_constraints<P: Display + Debug>(
        &self,
        name: &str,
        description: &str,
        constraints: &[crate::presburger::Constraint<P>],
    ) {
        let mut details = String::new();
        details.push_str(&format!("⚖️ CONSTRAINTS ({}):\n\n", constraints.len()));

        if constraints.is_empty() {
            details.push_str("  (No constraints - always satisfiable)\n");
        } else {
            // Use the existing Display implementation for each constraint
            details.push_str("📐 MATHEMATICAL FORM:\n");
            for (i, constraint) in constraints.iter().enumerate() {
                details.push_str(&format!("  {}. {}\n", i + 1, constraint));
            }

            // Analysis
            details.push_str("\n🔍 ANALYSIS:\n");
            let equality_count = constraints
                .iter()
                .filter(|c| {
                    matches!(
                        c.constraint_type(),
                        crate::presburger::ConstraintType::EqualToZero
                    )
                })
                .count();
            let inequality_count = constraints.len() - equality_count;

            details.push_str(&format!(
                "  • {} equality constraint{}\n",
                equality_count,
                if equality_count != 1 { "s" } else { "" }
            ));
            details.push_str(&format!(
                "  • {} inequality constraint{}\n",
                inequality_count,
                if inequality_count != 1 { "s" } else { "" }
            ));

            // Check for obvious contradictions
            let mut has_contradiction = false;
            for constraint in constraints {
                if constraint.linear_combination().is_empty() {
                    let rhs = -constraint.constant_term();
                    match constraint.constraint_type() {
                        crate::presburger::ConstraintType::EqualToZero => {
                            if rhs != 0 {
                                details.push_str(&format!(
                                    "  ⚠️ CONTRADICTION: 0 = {} (impossible!)\n",
                                    rhs
                                ));
                                has_contradiction = true;
                            }
                        }
                        crate::presburger::ConstraintType::NonNegative => {
                            if rhs < 0 {
                                details.push_str(&format!(
                                    "  ⚠️ CONTRADICTION: 0 ≥ {} (impossible!)\n",
                                    rhs
                                ));
                                has_contradiction = true;
                            }
                        }
                    }
                }
            }

            if !has_contradiction {
                details.push_str("  ✓ No obvious contradictions detected\n");
            }
        }

        self.step(name, description, &details);
    }

    pub fn log_disjunct_start<T: Clone + Display + Debug>(
        &self,
        disjunct_id: usize,
        quantified_set: &crate::presburger::QuantifiedSet<T>,
    ) {
        let mut details = String::new();
        details.push_str(&format!("🎯 DISJUNCT {} ANALYSIS:\n\n", disjunct_id));

        // Use the existing Display implementation
        details.push_str(&format!("📐 MATHEMATICAL FORM:\n{}\n\n", quantified_set));

        // Add structural analysis
        details.push_str("📊 STRUCTURE:\n");
        details.push_str(&format!(
            "  • {} constraint{}\n",
            quantified_set.constraints().len(),
            if quantified_set.constraints().len() != 1 {
                "s"
            } else {
                ""
            }
        ));

        // Check for existential variables
        let has_existentials = quantified_set.constraints().iter().any(|c| {
            c.linear_combination()
                .iter()
                .any(|(_, var)| format!("{:?}", var).contains("Existential"))
        });

        if has_existentials {
            details.push_str("  • Contains existential variables (∃)\n");
        } else {
            details.push_str("  • No existential variables\n");
        }

        self.step(
            &format!("Disjunct {} Analysis", disjunct_id),
            &format!("Starting analysis of disjunct {}", disjunct_id),
            &details,
        );
    }

    /// Log PresburgerSet using its Display implementation
    pub fn log_presburger_set<T: Clone + Display + Debug>(
        &self,
        name: &str,
        description: &str,
        pset: &crate::presburger::PresburgerSet<T>,
    ) {
        let mut details = String::new();
        details.push_str("🔢 PRESBURGER SET:\n\n");

        // Use the existing Display implementation
        details.push_str(&format!("📚 MATHEMATICAL FORM:\n{}\n\n", pset));

        // Add structural analysis
        details.push_str("📊 STRUCTURE:\n");
        details.push_str("  • ISL-based representation with variable mappings\n");

        self.step(name, description, &details);
    }

    /// Log QuantifiedSet using its Display implementation  
    pub fn log_quantified_set<T: Clone + Display + Debug>(
        &self,
        name: &str,
        description: &str,
        qset: &crate::presburger::QuantifiedSet<T>,
    ) {
        let mut details = String::new();
        details.push_str("⚖️ QUANTIFIED SET:\n\n");

        // Use the existing Display implementation
        details.push_str(&format!("📐 MATHEMATICAL FORM:\n{}\n\n", qset));

        // Add structural analysis
        details.push_str("📊 STRUCTURE:\n");
        details.push_str(&format!(
            "  • {} constraint{}\n",
            qset.constraints().len(),
            if qset.constraints().len() != 1 {
                "s"
            } else {
                ""
            }
        ));

        self.step(name, description, &details);
    }
}

// Global debug report instance for backward compatibility
use std::sync::Mutex;
use std::sync::OnceLock;

static DEBUG_REPORT: OnceLock<Mutex<Option<DebugReport>>> = OnceLock::new();

pub fn init_debug_report(program_name: String, program_content: String) {
    let report = DebugReport::new(program_name, program_content);
    DEBUG_REPORT.set(Mutex::new(Some(report))).unwrap();
}

pub fn add_debug_step(step_name: String, description: String, details: String) {
    if let Some(mutex) = DEBUG_REPORT.get() {
        if let Ok(mut report_opt) = mutex.lock() {
            if let Some(report) = report_opt.as_mut() {
                report.add_step(step_name, description, details);
            }
        }
    }
}

pub fn add_debug_smpt_call(call: SmptCall) {
    if let Some(mutex) = DEBUG_REPORT.get() {
        if let Ok(mut report_opt) = mutex.lock() {
            if let Some(report) = report_opt.as_mut() {
                report.add_smpt_call(call);
            }
        }
    }
}

pub fn finalize_debug_report(
    result: String,
    total_time_ms: u64,
    output_dir: &str,
) -> Result<(), std::io::Error> {
    if let Some(mutex) = DEBUG_REPORT.get() {
        if let Ok(mut report_opt) = mutex.lock() {
            if let Some(report) = report_opt.as_mut() {
                report.set_final_result(result, total_time_ms);
                let output_path = format!("{}/debug_report.html", output_dir);
                report.generate_html(&output_path)?;
            }
        }
    }
    Ok(())
}

pub fn format_constraints_description<P: Display>(constraints: &[Constraint<P>]) -> String {
    if constraints.is_empty() {
        return "No constraints".to_string();
    }

    constraints
        .iter()
        .enumerate()
        .map(|(i, constraint)| {
            let terms: Vec<String> = constraint
                .linear_combination()
                .iter()
                .map(|(coeff, var)| {
                    if *coeff == 1 {
                        format!("{}", var)
                    } else if *coeff == -1 {
                        format!("-{}", var)
                    } else {
                        format!("{}*{}", coeff, var)
                    }
                })
                .collect();

            let lhs = if terms.is_empty() {
                "0".to_string()
            } else {
                terms.join(" + ").replace(" + -", " - ")
            };

            let rhs = -constraint.constant_term();

            let op = match constraint.constraint_type() {
                crate::presburger::ConstraintType::NonNegative => "≥",
                crate::presburger::ConstraintType::EqualToZero => "=",
            };

            format!("{}. {} {} {}", i + 1, lhs, op, rhs)
        })
        .collect::<Vec<_>>()
        .join("; ")
}
