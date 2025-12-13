use crate::config::*;
use crate::structures::{Memory, OrderedSet};
use dashmap::DashMap;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct RecallResult {
    pub memory_id: String,
    pub content: String,
    pub score: f64,
    pub intersection_count: usize,
    pub recency_score: f64,
    pub reinforcement_score: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone)]
pub struct CueMapEngine {
    memories: Arc<DashMap<String, Memory>>,
    cue_index: Arc<DashMap<String, OrderedSet>>,
}

impl CueMapEngine {
    pub fn new() -> Self {
        Self {
            memories: Arc::new(DashMap::new()),
            cue_index: Arc::new(DashMap::new()),
        }
    }
    
    pub fn from_state(
        memories: DashMap<String, Memory>,
        cue_index: DashMap<String, OrderedSet>,
    ) -> Self {
        Self {
            memories: Arc::new(memories),
            cue_index: Arc::new(cue_index),
        }
    }
    
    // Expose internal state for persistence
    pub fn get_memories(&self) -> &Arc<DashMap<String, Memory>> {
        &self.memories
    }
    
    pub fn get_cue_index(&self) -> &Arc<DashMap<String, OrderedSet>> {
        &self.cue_index
    }
    
    pub fn add_memory(
        &self,
        content: String,
        cues: Vec<String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> String {
        let memory = Memory::new(content, metadata);
        let memory_id = memory.id.clone();
        
        // Store memory
        self.memories.insert(memory_id.clone(), memory);
        
        // Index by cues
        for cue in cues {
            let cue_lower = cue.to_lowercase().trim().to_string();
            if !cue_lower.is_empty() {
                self.cue_index
                    .entry(cue_lower)
                    .or_insert_with(OrderedSet::new)
                    .add(memory_id.clone());
            }
        }
        
        memory_id
    }
    
    pub fn reinforce_memory(&self, memory_id: &str, cues: Vec<String>) -> bool {
        // Update last accessed
        if let Some(mut memory) = self.memories.get_mut(memory_id) {
            memory.touch();
        } else {
            return false;
        }
        
        // Move to front for each cue
        for cue in cues {
            let cue_lower = cue.to_lowercase().trim().to_string();
            if !cue_lower.is_empty() {
                let mut entry = self.cue_index
                    .entry(cue_lower)
                    .or_insert_with(OrderedSet::new);
                entry.move_to_front(memory_id);
            }
        }
        
        true
    }
    
    pub fn recall(
        &self,
        query_cues: Vec<String>,
        limit: usize,
        auto_reinforce: bool,
    ) -> Vec<RecallResult> {
        self.recall_with_min_intersection(query_cues, limit, auto_reinforce, None)
    }
    
    pub fn recall_with_min_intersection(
        &self,
        query_cues: Vec<String>,
        limit: usize,
        auto_reinforce: bool,
        min_intersection: Option<usize>,
    ) -> Vec<RecallResult> {
        if query_cues.is_empty() {
            return Vec::new();
        }
        
        // Normalize cues
        let cues: Vec<String> = query_cues
            .iter()
            .map(|c| c.to_lowercase().trim().to_string())
            .filter(|c| !c.is_empty() && self.cue_index.contains_key(c))
            .collect();
        
        if cues.is_empty() {
            return Vec::new();
        }
        
        // Iterative deepening search
        let mut results = self.iterative_search(&cues, limit);
        
        // Filter by minimum intersection if specified
        if let Some(min_int) = min_intersection {
            results.retain(|r| r.intersection_count >= min_int);
        }
        
        // Auto-reinforce if enabled
        if auto_reinforce {
            for result in &results {
                self.reinforce_memory(&result.memory_id, query_cues.clone());
            }
        }
        
        results
    }
    
    fn iterative_search(&self, cues: &[String], limit: usize) -> Vec<RecallResult> {
        let tiers = [
            (0, TIER_1_DEPTH),
            (TIER_1_DEPTH, TIER_2_DEPTH),
            (TIER_2_DEPTH, MAX_SEARCH_DEPTH),
        ];
        
        for (tier_start, tier_end) in tiers {
            let candidates = self.gather_candidates(cues, tier_start, tier_end);
            
            if !candidates.is_empty() {
                let mut scored = self.score_candidates(candidates);
                
                // Filter by intersection threshold (inline for performance)
                scored.retain(|c| c.intersection_count >= 1);
                
                if !scored.is_empty() {
                    // Use unstable sort for better performance (order of equal elements doesn't matter)
                    scored.sort_unstable_by(|a, b| {
                        b.intersection_count
                            .cmp(&a.intersection_count)
                            .then_with(|| {
                                b.recency_score
                                    .partial_cmp(&a.recency_score)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                    });
                    
                    scored.truncate(limit);
                    return scored;
                }
            }
        }
        
        Vec::new()
    }
    
    fn gather_candidates(
        &self,
        cues: &[String],
        start: usize,
        end: usize,
    ) -> HashMap<String, Vec<(usize, usize)>> {
        // Pre-allocate with estimated capacity
        let mut candidates: HashMap<String, Vec<(usize, usize)>> = HashMap::with_capacity(end - start);
        
        for cue in cues {
            if let Some(ordered_set) = self.cue_index.get(cue) {
                // Use zero-copy references
                let memories = ordered_set.get_recent(Some(end));
                let list_len = ordered_set.len();
                
                // Only iterate over the range we care about
                // Clone only when inserting into HashMap (unavoidable)
                for (idx, memory_id) in memories.iter().enumerate().skip(start).take(end - start) {
                    candidates
                        .entry((*memory_id).clone())
                        .or_insert_with(Vec::new)
                        .push((idx, list_len));
                }
            }
        }
        
        candidates
    }
    
    fn score_candidates(&self, candidates: HashMap<String, Vec<(usize, usize)>>) -> Vec<RecallResult> {
        // Constants for continuous gradient weighting
        const MAX_REC_WEIGHT: f64 = 20.0;
        const MAX_FREQ_WEIGHT: f64 = 5.0;
        
        // Pre-allocate result vector
        let mut results = Vec::with_capacity(candidates.len());
        
        for (memory_id, positions_with_len) in candidates {
            if let Some(memory) = self.memories.get(&memory_id) {
                let intersection_count = positions_with_len.len();
                
                // Calculate weighted recency and find best position for gradient calculation
                let mut total_recency = 0.0;
                let mut total_w_rec = 0.0;
                let mut total_w_freq = 0.0;
                
                for &(pos, list_len) in &positions_with_len {
                    let pos_f64 = pos as f64;
                    let list_len_f64 = list_len as f64;
                    
                    // Calculate sigma (characteristic scale)
                    let sigma = list_len_f64.sqrt();
                    
                    // Calculate ratio (normalized depth)
                    let ratio = pos_f64 / sigma;
                    
                    // Dynamic weights using continuous gradient
                    // Recency weight: decays as depth increases
                    let w_rec = MAX_REC_WEIGHT / (ratio + 1.0);
                    
                    // Frequency weight: grows as depth increases
                    let w_freq = 1.0 + (MAX_FREQ_WEIGHT * (1.0 - (1.0 / (ratio + 1.0))));
                    
                    // Calculate recency component
                    let mut recency_component = 1.0 / (pos_f64 + 1.0);
                    
                    // Freshness boost for position 0
                    if pos == 0 {
                        recency_component += 1.0;
                    }
                    
                    total_recency += recency_component;
                    total_w_rec += w_rec;
                    total_w_freq += w_freq;
                }
                
                // Average the scores and weights across all positions
                let count = positions_with_len.len() as f64;
                let recency_score = total_recency / count;
                let avg_w_rec = total_w_rec / count;
                let avg_w_freq = total_w_freq / count;
                
                // Calculate frequency score using log10
                let frequency_score = if memory.reinforcement_count > 0 {
                    (memory.reinforcement_count as f64).log10()
                } else {
                    0.0
                };
                
                // Calculate intersection score
                let intersection_score = (intersection_count as f64) * 100.0;
                
                // Final score calculation with averaged weights
                let score = intersection_score + (recency_score * avg_w_rec) + (frequency_score * avg_w_freq);
                
                results.push(RecallResult {
                    memory_id: memory_id.clone(),
                    content: memory.content.clone(),
                    score,
                    intersection_count,
                    recency_score,
                    reinforcement_score: frequency_score,
                    metadata: memory.metadata.clone(),
                });
            }
        }
        
        results
    }
    
    pub fn get_memory(&self, memory_id: &str) -> Option<Memory> {
        self.memories.get(memory_id).map(|m| m.clone())
    }
    
    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert(
            "total_memories".to_string(),
            serde_json::json!(self.memories.len()),
        );
        stats.insert(
            "total_cues".to_string(),
            serde_json::json!(self.cue_index.len()),
        );
        
        let cues: Vec<String> = self.cue_index.iter().map(|e| e.key().clone()).collect();
        stats.insert("cues".to_string(), serde_json::json!(cues));
        
        stats
    }
}
