use crate::config::*;
use crate::structures::{Memory, OrderedSet};
use dashmap::DashMap;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<serde_json::Value>,
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
        let mut memory = Memory::new(content, metadata);
        let memory_id = memory.id.clone();
        
        // Store cues in memory
        memory.cues = cues.clone();
        
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

    pub fn delete_memory(&self, memory_id: &str) -> bool {
        if let Some((_, memory)) = self.memories.remove(memory_id) {
            // Remove from cue index
            for cue in memory.cues {
                 let cue_lower = cue.to_lowercase().trim().to_string();
                 if let Some(mut entry) = self.cue_index.get_mut(&cue_lower) {
                     entry.remove(memory_id);
                     // If set becomes empty, we might want to remove the cue entry entirely
                     // But OrderedSet might not expose "is_empty" or we might want to keep the cue
                     // For now, simple removal is enough.
                 }
            }
            true
        } else {
            false
        }
    }

    pub fn upsert_memory_with_id(
        &self,
        id: String,
        content: String,
        cues: Vec<String>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        reinforce: bool,
    ) -> String {
        // If exists: attach cues + optionally touch
        if self.memories.contains_key(&id) {
            self.attach_cues(&id, cues.clone());
            if reinforce {
                self.reinforce_memory(&id, cues);
            }
            return id;
        }
        
        // Insert new
        let mut memory = Memory::new(content, metadata);
        memory.id = id.clone();
        memory.cues = cues.clone();
        
        self.memories.insert(id.clone(), memory);
        
        // Index by cues
        for cue in cues {
            let cue_lower = cue.to_lowercase().trim().to_string();
            if !cue_lower.is_empty() {
                self.cue_index
                    .entry(cue_lower)
                    .or_insert_with(OrderedSet::new)
                    .add(id.clone());
            }
        }
        
        id
    }

    pub fn attach_cues(&self, memory_id: &str, cues: Vec<String>) -> bool {
        // 1. Get memory and check if it exists
        if let Some(mut memory) = self.memories.get_mut(memory_id) {
            // 2. Identify new cues (deduplication)
            let mut new_cues = Vec::new();
            for cue in cues {
                if !memory.cues.contains(&cue) {
                    new_cues.push(cue);
                }
            }

            if new_cues.is_empty() {
                return false;
            }

            // 3. Update memory.cues
            memory.cues.extend(new_cues.clone());

            // 4. Update index for new cues
            for cue in new_cues {
                let cue_lower = cue.to_lowercase().trim().to_string();
                if !cue_lower.is_empty() {
                    self.cue_index
                        .entry(cue_lower)
                        .or_insert_with(OrderedSet::new)
                        .add(memory_id.to_string());
                }
            }
            
            true
        } else {
            false
        }
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
        
        // Default weight of 1.0 for standard recall
        let weighted_cues: Vec<(String, f64)> = query_cues
            .into_iter()
            .map(|c| (c, 1.0))
            .collect();
            
        self.recall_weighted(weighted_cues, limit, auto_reinforce, min_intersection, false)
    }

    pub fn recall_weighted(
        &self,
        query_cues: Vec<(String, f64)>,
        limit: usize,
        auto_reinforce: bool,
        min_intersection: Option<usize>,
        explain: bool,
    ) -> Vec<RecallResult> {
        if query_cues.is_empty() {
            return Vec::new();
        }
        
        // Normalize cues
        let cues: Vec<(String, f64)> = query_cues
            .into_iter()
            .map(|(c, w)| (c.to_lowercase().trim().to_string(), w))
            .filter(|(c, _)| !c.is_empty() && self.cue_index.contains_key(c))
            .collect();
        
        if cues.is_empty() {
            return Vec::new();
        }
        
        // Consolidated search using Selective Set Intersection
        let mut results = self.consolidated_search(&cues, limit, explain);
        
        // Filter by minimum intersection if specified
        if let Some(min_int) = min_intersection {
            results.retain(|r| r.intersection_count >= min_int);
        }
        
        // Auto-reinforce if enabled
        if auto_reinforce {
            let just_cues: Vec<String> = cues.iter().map(|(c, _)| c.clone()).collect();
            for result in &results {
                self.reinforce_memory(&result.memory_id, just_cues.clone());
            }
        }

        // Global sort by score
        results.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);
        
        results
    }
    
    fn consolidated_search(&self, query_cues: &[(String, f64)], _limit: usize, explain: bool) -> Vec<RecallResult> {
        if query_cues.is_empty() {
            return Vec::new();
        }

        // 1. Gather cue data
        let mut cue_data = Vec::with_capacity(query_cues.len());
        for (cue, weight) in query_cues {
            if let Some(ordered_set) = self.cue_index.get(cue) {
                cue_data.push((cue.clone(), *weight, ordered_set));
            }
        }

        if cue_data.is_empty() {
            return Vec::new();
        }

        // 2. Perform Union-based search with O(1) Probing
        // We iterate through EVERY cue's list up to MAX_DRIVER_SCAN to ensure partial matches are found.
        let mut candidates = Vec::new();
        let mut seen_memories = HashSet::new();

        for (cue_idx, (_cue, _weight, set)) in cue_data.iter().enumerate() {
            let scan_limit = std::cmp::min(set.len(), MAX_DRIVER_SCAN);
            let items = set.get_recent(Some(scan_limit));

            for (pos_rev, memory_id) in items.iter().enumerate() {
                // If we've already processed this memory from a previous (likely more selective or relevant) cue, skip it
                if seen_memories.contains(*memory_id) {
                    continue;
                }
                seen_memories.insert((*memory_id).clone());

                let mut total_weight = 0.0;
                let mut positions_info = Vec::with_capacity(cue_data.len());

                // 3. For each NEW candidate, probe ALL query cue lists to get full intersection data
                for (other_idx, (_other_cue, other_weight, other_set)) in cue_data.iter().enumerate() {
                    // Optimization: if it's the current set we're iterating, we know it's there
                    if other_idx == cue_idx {
                        total_weight += *other_weight;
                        positions_info.push((pos_rev, other_set.len(), *other_weight));
                        continue;
                    }

                    // O(1) probe into other sets
                    if let Some(oldest_idx) = other_set.get_index_of(memory_id) {
                        total_weight += *other_weight;
                        let recency_pos = (other_set.len() - 1) - oldest_idx;
                        positions_info.push((recency_pos, other_set.len(), *other_weight));
                    }
                }

                // 4. Collect candidate
                candidates.push(((*memory_id).clone(), positions_info, total_weight));
            }
        }

        // 5. Score candidates
        self.score_consolidated_candidates(candidates, explain)
    }

    fn score_consolidated_candidates(&self, candidates: Vec<(String, Vec<(usize, usize, f64)>, f64)>, explain: bool) -> Vec<RecallResult> {
        const MAX_REC_WEIGHT: f64 = 20.0;
        const MAX_FREQ_WEIGHT: f64 = 5.0;
        
        let mut results = Vec::with_capacity(candidates.len());
        
        for (memory_id, positions_info, total_weight) in candidates {
            if let Some(memory) = self.memories.get(&memory_id) {
                let mut total_recency = 0.0;
                let mut total_w_rec = 0.0;
                let mut total_w_freq = 0.0;
                
                let match_count = positions_info.len() as f64;

                for (pos, list_len, weight) in positions_info {
                    let pos_f64 = pos as f64;
                    let list_len_f64 = list_len as f64;
                    let sigma = list_len_f64.sqrt();
                    let ratio = pos_f64 / sigma;
                    
                    let w_rec = MAX_REC_WEIGHT / (ratio + 1.0);
                    let w_freq = 1.0 + (MAX_FREQ_WEIGHT * (1.0 - (1.0 / (ratio + 1.0))));
                    
                    let mut recency_component = 1.0 / (pos_f64 + 1.0);
                    if pos == 0 {
                        recency_component += 1.0;
                    }
                    
                    total_recency += recency_component * weight; // Weigh the recency contribution
                    total_w_rec += w_rec;
                    total_w_freq += w_freq;
                }
                
                let avg_w_rec = total_w_rec / match_count;
                let avg_w_freq = total_w_freq / match_count;
                let recency_score = total_recency / match_count;
                
                let frequency_score = if memory.reinforcement_count > 0 {
                    (memory.reinforcement_count as f64).log10()
                } else {
                    0.0
                };
                
                let intersection_score = total_weight * 100.0;
                let score = intersection_score + (recency_score * avg_w_rec) + (frequency_score * avg_w_freq);
                
                let explain_data = if explain {
                    Some(serde_json::json!({
                        "intersection_weighted": total_weight,
                        "intersection_score": intersection_score,
                        "recency_component": recency_score,
                        "frequency_component": frequency_score,
                        "weights": {
                            "recency": avg_w_rec,
                            "frequency": avg_w_freq
                        },
                        "match_count": match_count
                    }))
                } else {
                    None
                };

                results.push(RecallResult {
                    memory_id,
                    content: memory.content.clone(),
                    score,
                    intersection_count: match_count as usize,
                    recency_score,
                    reinforcement_score: frequency_score,
                    metadata: memory.metadata.clone(),
                    explain: explain_data,
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
