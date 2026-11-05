// Added 2026-11-03 by Justeck, Apache-2.0.
#[cfg(test)]
mod tests {
    use super::super::*;
    #[test]
    fn reused_worker_scratch_refills_after_toggle_and_full_texture_mode() {
        let mut cfg = rusty::Config::default();
        cfg.set_texture_visibility(0.).unwrap();
        rusty::set_config(cfg).unwrap();
        rusty::set_catalog(
            vec![
                "minecraft:air".into(),
                "minecraft:stone".into(),
                "minecraft:diamond_ore".into(),
            ],
            vec![Default::default(); 3],
        );
        let mut center = Chunk::empty(0, 0);
        let mut east = Chunk::empty(1, 0);
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    center.set(x, MIN_Y + y, z, 1);
                    east.set(x, MIN_Y + y, z, 1);
                }
            }
        }
        center.set(15, MIN_Y + 8, 8, 2);
        east.set(0, MIN_Y + 8, 8, 2);
        let mut region = [[None; 3]; 3];
        region[1][1] = Some(&center);
        region[1][2] = Some(&east);
        let mut scratch = MeshScratch::default();
        let mut own = [0; 4096];
        rusty::set_active(false);
        scratch.padded.fill(&region, 0, &mut own);
        let normal_blocks = scratch.padded.blocks.clone();
        let normal_light = scratch.padded.light.clone();
        assert_eq!(normal_blocks[pi(14, 8, 8)], 1);
        rusty::set_active(true);
        scratch.padded.fill(&region, 0, &mut own);
        assert_eq!(
            scratch.padded.blocks[pi(14, 8, 8)],
            0,
            "filter must run on the real reusable worker scratch"
        );
        assert_eq!(scratch.padded.blocks[pi(15, 8, 8)], 2);
        assert_eq!(
            scratch.padded.blocks[pi(16, 8, 8)],
            2,
            "chunk-border neighbor remains selected"
        );
        assert!(scratch.padded.light.iter().all(|&v| v == 255));
        rusty::set_active(false);
        scratch.padded.fill(&region, 0, &mut own);
        assert_eq!(
            scratch.padded.blocks, normal_blocks,
            "off must refill filtered cells from received source"
        );
        assert_eq!(
            scratch.padded.light, normal_light,
            "off must restore original light after scratch reuse"
        );
        let mut cfg = (*rusty::config()).clone();
        cfg.set_texture_visibility(100.).unwrap();
        rusty::set_config(cfg).unwrap();
        rusty::set_active(true);
        scratch.padded.fill(&region, 0, &mut own);
        assert_eq!(scratch.padded.blocks, normal_blocks);
        assert_eq!(scratch.padded.light, normal_light);
        assert_eq!(center.get(14, MIN_Y + 8, 8), 1);
        assert_eq!(center.get(15, MIN_Y + 8, 8), 2);
        assert_eq!(east.get(0, MIN_Y + 8, 8), 2);
        rusty::set_active(false);
    }
    #[test]
    #[ignore = "requires HORIZON_MESH_TEST_RESOURCES pointing at legally prepared 26.3 assets"]
    fn enclosed_border_ore_toggle_and_source_preservation() {
        let mut cfg = rusty::Config::default();
        cfg.set_texture_visibility(0.).unwrap();
        rusty::set_config(cfg).unwrap();
        let root = std::path::PathBuf::from(
            std::env::var_os("HORIZON_MESH_TEST_RESOURCES").expect("prepared resources"),
        );
        let assets = Assets::load(&root).unwrap();
        let facts = BlockFacts::load(&root, assets.registry).unwrap();
        let biomes = Biomes::load(&root).unwrap();
        let render = assets.reload(&root).unwrap();
        let visibility = crate::models::load_visibility(&root, assets.registry).unwrap();
        let tables = MeshTables::new(&assets, &render, &facts, visibility.clone()).unwrap();
        let stone = assets.registry.default_state("minecraft:stone").unwrap().0;
        let diamond = assets
            .registry
            .default_state("minecraft:diamond_ore")
            .unwrap()
            .0;
        let mut center = Chunk::empty(0, 0);
        let mut east = Chunk::empty(1, 0);
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    center.set(x, MIN_Y + y, z, stone);
                    east.set(x, MIN_Y + y, z, stone);
                }
            }
        }
        // The selected block touches a chunk boundary and is enclosed on every side.
        center.set(15, MIN_Y + 8, 8, diamond);
        let mut region = [[None; 3]; 3];
        region[1][1] = Some(&center);
        region[1][2] = Some(&east);
        let mut mesher = Mesher {
            nether_shading: false,
            shader_pack_lighting: false,
            separate_ao: false,
            assets: &assets,
            render: &render,
            tables: &tables,
            biomes: &biomes,
            visibility,
            zoom: BiomeZoom::new(0),
            blend_radius: 0,
            smooth_lighting: true,
            cutout_leaves: true,
            translucent_layers: &[],
            leaf_culling: crate::graphics::LeafCullingMode::default(),
        };
        let mut scratch = MeshScratch::default();
        let reusable = SectionMesh::default();
        rusty::set_active(false);
        let normal = mesher.mesh_reusing(&region, 0, reusable, &mut scratch);
        rusty::set_active(true);
        let xray = mesher.mesh_reusing(&region, 0, SectionMesh::default(), &mut scratch);
        let mut cfg = (*rusty::config()).clone();
        cfg.set_texture_visibility(100.).unwrap();
        rusty::set_config(cfg).unwrap();
        let maximum_textures =
            mesher.mesh_reusing(&region, 0, SectionMesh::default(), &mut scratch);
        rusty::set_active(false);
        let restored = mesher.mesh_reusing(&region, 0, SectionMesh::default(), &mut scratch);
        assert_eq!(
            xray.solid.len(),
            24,
            "six faces including across the chunk border"
        );
        assert!(xray.solid.iter().all(|v| v.light == 0xf0f0));
        assert_eq!(xray.visibility, ALL_VISIBLE);
        assert_eq!(maximum_textures.visibility, normal.visibility);
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&normal.solid),
            bytemuck::cast_slice::<_, u8>(&maximum_textures.solid),
            "100% textures must use the same terrain, UVs and lighting as normal play"
        );
        assert!(normal.solid.len() > xray.solid.len());
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&normal.solid),
            bytemuck::cast_slice::<_, u8>(&restored.solid)
        );
        assert_eq!(center.get(14, MIN_Y + 8, 8), stone);
        assert_eq!(center.get(15, MIN_Y + 8, 8), diamond);
        assert_eq!(east.get(0, MIN_Y + 8, 8), stone);
    }
}
