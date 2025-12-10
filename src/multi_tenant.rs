//! Multi-tenant engine supporting project isolation.

use crate::engine::CueMapEngine;
use crate::persistence::PersistenceManager;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub type ProjectId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub project_id: ProjectId,
    pub total_memories: usize,
    pub total_cues: usize,
    pub created_at: f64,
    pub last_activity: f64,
}

#[derive(Clone)]
pub struct MultiTenantEngine {
    projects: Arc<DashMap<ProjectId, Arc<CueMapEngine>>>,
    snapshots_dir: PathBuf,
}

impl MultiTenantEngine {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_snapshots_dir("./snapshots")
    }
    
    pub fn with_snapshots_dir<P: AsRef<Path>>(dir: P) -> Self {
        let snapshots_dir = dir.as_ref().to_path_buf();
        
        // Create snapshots directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&snapshots_dir) {
            eprintln!("Warning: Failed to create snapshots directory: {}", e);
        }
        
        Self {
            projects: Arc::new(DashMap::new()),
            snapshots_dir,
        }
    }
    
    pub fn get_or_create_project(&self, project_id: ProjectId) -> Arc<CueMapEngine> {
        if let Some(engine) = self.projects.get(&project_id) {
            engine.clone()
        } else {
            let engine = Arc::new(CueMapEngine::new());
            self.projects.insert(project_id, engine.clone());
            engine
        }
    }
    
    pub fn get_project(&self, project_id: &ProjectId) -> Option<Arc<CueMapEngine>> {
        self.projects.get(project_id).map(|e| e.clone())
    }
    
    pub fn list_projects(&self) -> Vec<ProjectStats> {
        self.projects
            .iter()
            .map(|entry| {
                let project_id = entry.key().clone();
                let engine = entry.value();
                let stats = engine.get_stats();
                
                ProjectStats {
                    project_id,
                    total_memories: stats.get("total_memories")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    total_cues: stats.get("total_cues")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    created_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                    last_activity: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                }
            })
            .collect()
    }
    
    pub fn delete_project(&self, project_id: &ProjectId) -> bool {
        self.projects.remove(project_id).is_some()
    }
    
    /// Insert a pre-loaded project engine (for static loading)
    #[allow(dead_code)]
    pub fn insert_project(&self, project_id: ProjectId, engine: Arc<CueMapEngine>) {
        self.projects.insert(project_id, engine);
    }
    
    /// Save a project snapshot to disk
    pub fn save_project(&self, project_id: &ProjectId) -> Result<PathBuf, String> {
        let engine = self.get_project(project_id)
            .ok_or_else(|| format!("Project '{}' not found", project_id))?;
        
        let snapshot_path = self.snapshots_dir.join(format!("{}.bin", project_id));
        PersistenceManager::save_to_path(&engine, &snapshot_path)
            .map_err(|e| format!("Failed to save project: {}", e))?;
        
        Ok(snapshot_path)
    }
    
    /// Load a project snapshot from disk
    pub fn load_project(&self, project_id: &ProjectId) -> Result<Arc<CueMapEngine>, String> {
        let snapshot_path = self.snapshots_dir.join(format!("{}.bin", project_id));
        
        if !snapshot_path.exists() {
            return Err(format!("Snapshot for project '{}' not found", project_id));
        }
        
        let (memories, cue_index) = PersistenceManager::load_from_path(&snapshot_path)
            .map_err(|e| format!("Failed to load project: {}", e))?;
        
        let engine = Arc::new(CueMapEngine::from_state(memories, cue_index));
        self.projects.insert(project_id.clone(), engine.clone());
        
        Ok(engine)
    }
    
    /// Save all projects to disk
    pub fn save_all(&self) -> HashMap<String, Result<PathBuf, String>> {
        let mut results = HashMap::new();
        
        for entry in self.projects.iter() {
            let project_id = entry.key().clone();
            let result = self.save_project(&project_id);
            results.insert(project_id, result);
        }
        
        results
    }
    
    /// Load all available snapshots from disk
    pub fn load_all(&self) -> HashMap<String, Result<(), String>> {
        let mut results = HashMap::new();
        let snapshots = self.list_snapshots();
        
        for project_id in snapshots {
            let result = self.load_project(&project_id)
                .map(|_| ())
                .map_err(|e| format!("Failed to load: {}", e));
            results.insert(project_id, result);
        }
        
        results
    }
    
    /// List available snapshots on disk
    pub fn list_snapshots(&self) -> Vec<String> {
        PersistenceManager::list_snapshots_in_dir(&self.snapshots_dir)
    }
    
    /// Delete a project snapshot from disk
    #[allow(dead_code)]
    pub fn delete_snapshot(&self, project_id: &ProjectId) -> Result<(), String> {
        let snapshot_path = self.snapshots_dir.join(format!("{}.bin", project_id));
        PersistenceManager::delete_snapshot(&snapshot_path)
    }
    
    #[allow(dead_code)]
    pub fn get_global_stats(&self) -> HashMap<String, serde_json::Value> {
        let projects = self.list_projects();
        
        let total_memories: usize = projects.iter().map(|p| p.total_memories).sum();
        let total_cues: usize = projects.iter().map(|p| p.total_cues).sum();
        
        let mut stats = HashMap::new();
        stats.insert(
            "total_projects".to_string(),
            serde_json::json!(projects.len()),
        );
        stats.insert(
            "total_memories".to_string(),
            serde_json::json!(total_memories),
        );
        stats.insert(
            "total_cues".to_string(),
            serde_json::json!(total_cues),
        );
        stats.insert(
            "projects".to_string(),
            serde_json::json!(projects),
        );
        
        stats
    }
}

/// Validate project ID format
pub fn validate_project_id(project_id: &str) -> bool {
    // Allow alphanumeric, hyphens, underscores
    // Length between 3 and 64 characters
    if project_id.len() < 3 || project_id.len() > 64 {
        return false;
    }
    
    project_id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}
