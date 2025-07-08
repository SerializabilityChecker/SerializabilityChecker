use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;
use chrono::{DateTime, Utc};
use crate::reachability::BIDIRECTIONAL_PRUNING_ENABLED;
use crate::semilinear::{GENERATE_LESS, REMOVE_REDUNDANT};
use crate::kleene::SMART_ORDER;
use std::sync::atomic::Ordering;

lazy_static::lazy_static! {
    pub static ref STATS_COLLECTOR: Mutex<StatsCollector> = Mutex::new(StatsCollector::new());
    pub static ref CURRENT_DISJUNCT_STATS: Mutex<DisjunctStatsCollector> = Mutex::new(DisjunctStatsCollector::new());
}

pub struct DisjunctStatsCollector {
    disjunct_id: usize,
    initial_places: usize,
    initial_transitions: usize,
    pruning_iterations: usize,
    final_places: Option<usize>,
    final_transitions: Option<usize>,
}

impl DisjunctStatsCollector {
    pub fn new() -> Self {
        DisjunctStatsCollector {
            disjunct_id: 0,
            initial_places: 0,
            initial_transitions: 0,
            pruning_iterations: 0,
            final_places: None,
            final_transitions: None,
        }
    }
    
    pub fn start_disjunct(&mut self, id: usize, places: usize, transitions: usize) {
        self.disjunct_id = id;
        self.initial_places = places;
        self.initial_transitions = transitions;
        self.pruning_iterations = 0;
        self.final_places = None;
        self.final_transitions = None;
    }
    
    pub fn record_pruning_iteration(&mut self) {
        self.pruning_iterations += 1;
    }
    
    pub fn set_final_sizes(&mut self, places: usize, transitions: usize) {
        self.final_places = Some(places);
        self.final_transitions = Some(transitions);
    }
    
    pub fn to_disjunct_stats(&self) -> DisjunctStats {
        let final_places = self.final_places.unwrap_or(self.initial_places);
        let final_transitions = self.final_transitions.unwrap_or(self.initial_transitions);
        
        DisjunctStats {
            id: self.disjunct_id,
            places_after: final_places,
            transitions_after: final_transitions,
            pruning_iterations: self.pruning_iterations,
            removed_places: self.initial_places.saturating_sub(final_places),
            removed_transitions: self.initial_transitions.saturating_sub(final_transitions),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializabilityStats {
    pub timestamp: DateTime<Utc>,
    pub example: String,
    pub options: OptimizationOptions,
    pub result: String, // "serializable", "not_serializable", "error", "timeout"
    pub certificate_creation_time_ms: Option<u64>,
    pub certificate_checking_time_ms: Option<u64>,
    pub num_disjuncts: usize,
    pub semilinear_set: SemilinearSetStats,
    pub petri_net: PetriNetStats,
    pub total_time_ms: u64,
    pub smpt_calls: usize,
    pub smpt_timeouts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationOptions {
    pub bidirectional_pruning: bool,
    pub remove_redundant: bool,
    pub generate_less: bool,
    pub smart_kleene_order: bool,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemilinearSetStats {
    pub num_components: usize,
    pub components: Vec<SemilinearComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemilinearComponent {
    pub periods: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetriNetStats {
    pub places_before: usize,
    pub transitions_before: usize,
    pub disjuncts: Vec<DisjunctStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisjunctStats {
    pub id: usize,
    pub places_after: usize,
    pub transitions_after: usize,
    pub pruning_iterations: usize,
    pub removed_places: usize,
    pub removed_transitions: usize,
}

pub struct StatsCollector {
    current_stats: Option<SerializabilityStats>,
    start_time: Option<Instant>,
    certificate_creation_start: Option<Instant>,
    certificate_checking_start: Option<Instant>,
    was_saved: bool,
}

impl StatsCollector {
    pub fn new() -> Self {
        StatsCollector {
            current_stats: None,
            start_time: None,
            certificate_creation_start: None,
            certificate_checking_start: None,
            was_saved: false,
        }
    }

    pub fn start_new_analysis(&mut self, example: String) {
        self.was_saved = false;  // Reset for new analysis
        self.start_time = Some(Instant::now());
        self.current_stats = Some(SerializabilityStats {
            timestamp: Utc::now(),
            example,
            options: OptimizationOptions {
                bidirectional_pruning: BIDIRECTIONAL_PRUNING_ENABLED.load(Ordering::Relaxed),
                remove_redundant: REMOVE_REDUNDANT.load(Ordering::Relaxed),
                generate_less: GENERATE_LESS.load(Ordering::Relaxed),
                smart_kleene_order: SMART_ORDER.load(Ordering::Relaxed),
                timeout: crate::smpt::get_smpt_timeout(),
            },
            result: "unknown".to_string(),
            certificate_creation_time_ms: None,
            certificate_checking_time_ms: None,
            num_disjuncts: 0,
            semilinear_set: SemilinearSetStats {
                num_components: 0,
                components: vec![],
            },
            petri_net: PetriNetStats {
                places_before: 0,
                transitions_before: 0,
                disjuncts: vec![],
            },
            total_time_ms: 0,
            smpt_calls: 0,
            smpt_timeouts: 0,
        });
    }

    pub fn start_certificate_creation(&mut self) {
        self.certificate_creation_start = Some(Instant::now());
    }

    pub fn end_certificate_creation(&mut self) {
        if let Some(start) = self.certificate_creation_start.take() {
            if let Some(stats) = &mut self.current_stats {
                stats.certificate_creation_time_ms = Some(start.elapsed().as_millis() as u64);
            }
        }
    }

    pub fn start_certificate_checking(&mut self) {
        self.certificate_checking_start = Some(Instant::now());
    }

    pub fn end_certificate_checking(&mut self) {
        if let Some(start) = self.certificate_checking_start.take() {
            if let Some(stats) = &mut self.current_stats {
                stats.certificate_checking_time_ms = Some(start.elapsed().as_millis() as u64);
            }
        }
    }

    pub fn set_result(&mut self, result: &str) {
        if let Some(stats) = &mut self.current_stats {
            stats.result = result.to_string();
        }
    }

    pub fn set_petri_net_sizes(&mut self, places: usize, transitions: usize) {
        if let Some(stats) = &mut self.current_stats {
            stats.petri_net.places_before = places;
            stats.petri_net.transitions_before = transitions;
        }
    }

    pub fn add_disjunct_stats(&mut self, disjunct: DisjunctStats) {
        if let Some(stats) = &mut self.current_stats {
            stats.petri_net.disjuncts.push(disjunct);
            stats.num_disjuncts = stats.petri_net.disjuncts.len();
        }
    }

    pub fn set_semilinear_stats(&mut self, semilinear: SemilinearSetStats) {
        if let Some(stats) = &mut self.current_stats {
            stats.semilinear_set = semilinear;
        }
    }

    pub fn increment_smpt_calls(&mut self) {
        if let Some(stats) = &mut self.current_stats {
            stats.smpt_calls += 1;
        }
    }

    pub fn increment_smpt_timeouts(&mut self) {
        if let Some(stats) = &mut self.current_stats {
            stats.smpt_timeouts += 1;
        }
    }

    pub fn finalize_and_save(&mut self) {
        if self.was_saved {
            return;
        }
        self.was_saved = true;

        if let (Some(start), Some(mut stats)) = (self.start_time.take(), self.current_stats.take()) {
            stats.total_time_ms = start.elapsed().as_millis() as u64;
            
            // Save to JSONL file
            if let Err(e) = append_stats_to_file(&stats) {
                eprintln!("Failed to save statistics: {}", e);
            }
        }
    }
}

fn append_stats_to_file(stats: &SerializabilityStats) -> std::io::Result<()> {
    // Ensure out directory exists
    std::fs::create_dir_all("out")?;
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("out/serializability_stats.jsonl")?;
    
    let json = serde_json::to_string(stats)?;
    writeln!(file, "{}", json)?;
    
    Ok(())
}

// Helper functions to be called from various parts of the codebase
pub fn start_analysis(example: String) {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.start_new_analysis(example);
    }
}

pub fn record_certificate_creation_time<F, R>(f: F) -> R 
where 
    F: FnOnce() -> R
{
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.start_certificate_creation();
    }
    let result = f();
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.end_certificate_creation();
    }
    result
}

pub fn record_certificate_checking_time<F, R>(f: F) -> R 
where 
    F: FnOnce() -> R
{
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.start_certificate_checking();
    }
    let result = f();
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.end_certificate_checking();
    }
    result
}

pub fn set_analysis_result(result: &str) {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.set_result(result);
    }
}

pub fn set_petri_net_sizes(places: usize, transitions: usize) {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.set_petri_net_sizes(places, transitions);
    }
}

pub fn add_disjunct_stats(stats: DisjunctStats) {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.add_disjunct_stats(stats);
    }
}

pub fn set_semilinear_stats(stats: SemilinearSetStats) {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.set_semilinear_stats(stats);
    }
}

pub fn increment_smpt_calls() {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.increment_smpt_calls();
    }
}

pub fn increment_smpt_timeouts() {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.increment_smpt_timeouts();
    }
}

pub fn finalize_stats() {
    if let Ok(mut collector) = STATS_COLLECTOR.lock() {
        collector.finalize_and_save();
    }
}

// Disjunct-specific helper functions
pub fn start_disjunct_analysis(id: usize, places: usize, transitions: usize) {
    if let Ok(mut collector) = CURRENT_DISJUNCT_STATS.lock() {
        collector.start_disjunct(id, places, transitions);
    }
}

pub fn record_pruning_iteration() {
    if let Ok(mut collector) = CURRENT_DISJUNCT_STATS.lock() {
        collector.record_pruning_iteration();
    }
}

pub fn finalize_disjunct(final_places: usize, final_transitions: usize) {
    if let Ok(mut disjunct_collector) = CURRENT_DISJUNCT_STATS.lock() {
        disjunct_collector.set_final_sizes(final_places, final_transitions);
        let stats = disjunct_collector.to_disjunct_stats();
        
        // Add to main stats collector
        if let Ok(mut main_collector) = STATS_COLLECTOR.lock() {
            main_collector.add_disjunct_stats(stats);
        }
    }
}