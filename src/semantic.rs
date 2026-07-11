use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::ast::Module;
use crate::manifest::ExternalManifest;

pub fn analyze(module: &Module, manifest: Option<&ExternalManifest>) -> Result<()> {
    if module.name != module.end_name {
        bail!(
            "Module name mismatch at END: expected '{}', got '{}'",
            module.name,
            module.end_name
        );
    }

    let mut seen_local = HashSet::new();
    for import in &module.imports {
        if !seen_local.insert(import.local_name.clone()) {
            bail!("Duplicate import alias: '{}'", import.local_name);
        }

        if let Some(m) = manifest
            && m.resolve(&import.external_name).is_none()
        {
            bail!(
                "Import '{}' is not mapped to a crate in the manifest",
                import.external_name
            );
        }
    }

    Ok(())
}
