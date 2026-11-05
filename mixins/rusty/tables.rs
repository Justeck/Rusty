// Rusty native hook, Copyright 2026 Justeck, Apache-2.0.
fn rusty_tables(facts: &BlockFacts, assets: &Assets) {
    rusty::set_catalog(
        facts.states.iter().map(|s| s.name.to_string()).collect(),
        (0..facts.states.len())
            .map(|i| {
                assets
                    .registry
                    .state(BlockStateId(i as u32))
                    .map(|s| {
                        s.properties
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect(),
    );
}
