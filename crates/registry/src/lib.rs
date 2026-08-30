use std::collections::HashMap;
use std::sync::Arc;

use glassvm_core::{MachineBundle, MachineId};

pub struct Registry {
    bundles: HashMap<MachineId, Arc<dyn MachineBundle>>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            bundles: HashMap::new(),
        }
    }

    pub fn register(&mut self, bundle: Arc<dyn MachineBundle>) -> Result<(), String> {
        bundle
            .validate_contract()
            .map_err(|errors| errors.join("; "))?;
        let id = bundle.descriptor().id.clone();
        if self.bundles.contains_key(&id) {
            return Err(format!("machine already registered: {}", id.0));
        }
        self.bundles.insert(id, bundle);
        Ok(())
    }

    pub fn get(&self, id: &MachineId) -> Option<Arc<dyn MachineBundle>> {
        self.bundles.get(id).cloned()
    }

    pub fn list(&self) -> Vec<MachineId> {
        let mut ids: Vec<MachineId> = self.bundles.keys().cloned().collect();
        ids.sort_by(|a, b| a.0.cmp(&b.0));
        ids
    }
}
