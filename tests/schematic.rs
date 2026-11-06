use rusty_horizon::rusty_schematic::*;
use std::collections::BTreeMap;

#[test]
fn sponge_empty_property_brackets_normalize_without_accepting_bad_entries() {
    for version in [2, 3] {
        for descriptor in ["minecraft:air[]", "minecraft:stone[]"] {
            let schematic = decode(&nbt(sponge(
                version,
                [1, 1, 1],
                vec![(descriptor, 0)],
                vec![0],
            )))
            .unwrap();
            assert!(schematic.palette[0].properties.is_empty());
            assert_eq!(
                schematic.palette[0].descriptor(),
                descriptor.trim_end_matches("[]")
            );
        }
        for descriptor in [
            "minecraft:air[",
            "minecraft:air]",
            "minecraft:air[,]",
            "minecraft:air[=]",
            "minecraft:air[a=]",
            "minecraft:air[=x]",
            "minecraft:air[a=x,]",
            "minecraft:air[,a=x]",
            "minecraft:air[a=x,a=y]",
            "minecraft:air[[]]",
            "minecraft:air[][]",
        ] {
            assert!(
                decode(&nbt(sponge(
                    version,
                    [1, 1, 1],
                    vec![(descriptor, 0)],
                    vec![0]
                )))
                .is_err(),
                "accepted malformed {descriptor}"
            );
        }
    }
}

// Deliberately independent binary fixtures: no production serializer or parser helpers.
#[derive(Clone)]
enum Tag {
    Short(i16),
    Int(i32),
    Str(String),
    Bytes(Vec<u8>),
    Ints(Vec<i32>),
    Longs(Vec<u64>),
    List(u8, Vec<Tag>),
    Compound(Vec<(String, Tag)>),
}
fn c(entries: Vec<(&str, Tag)>) -> Tag {
    Tag::Compound(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn s(value: &str) -> Tag {
    Tag::Str(value.into())
}
fn list_int(v: [i32; 3]) -> Tag {
    Tag::List(3, v.into_iter().map(Tag::Int).collect())
}
fn xyz(v: [i32; 3]) -> Tag {
    c(vec![
        ("x", Tag::Int(v[0])),
        ("y", Tag::Int(v[1])),
        ("z", Tag::Int(v[2])),
    ])
}
fn state(name: &str, props: &[(&str, &str)]) -> Tag {
    c(vec![
        ("Name", s(name)),
        (
            "Properties",
            c(props.iter().map(|&(k, v)| (k, s(v))).collect()),
        ),
    ])
}
fn id(t: &Tag) -> u8 {
    match t {
        Tag::Short(_) => 2,
        Tag::Int(_) => 3,
        Tag::Str(_) => 8,
        Tag::Bytes(_) => 7,
        Tag::Ints(_) => 11,
        Tag::Longs(_) => 12,
        Tag::List(_, _) => 9,
        Tag::Compound(_) => 10,
    }
}
fn string(out: &mut Vec<u8>, s: &str) {
    out.extend((s.len() as u16).to_be_bytes());
    out.extend(s.as_bytes());
}
fn payload(out: &mut Vec<u8>, t: &Tag) {
    match t {
        Tag::Short(v) => out.extend(v.to_be_bytes()),
        Tag::Int(v) => out.extend(v.to_be_bytes()),
        Tag::Str(v) => string(out, v),
        Tag::Bytes(v) => {
            out.extend((v.len() as i32).to_be_bytes());
            out.extend(v)
        }
        Tag::Ints(v) => {
            out.extend((v.len() as i32).to_be_bytes());
            for x in v {
                out.extend(x.to_be_bytes())
            }
        }
        Tag::Longs(v) => {
            out.extend((v.len() as i32).to_be_bytes());
            for x in v {
                out.extend(x.to_be_bytes())
            }
        }
        Tag::List(kind, v) => {
            out.push(*kind);
            out.extend((v.len() as i32).to_be_bytes());
            for x in v {
                assert_eq!(id(x), *kind);
                payload(out, x)
            }
        }
        Tag::Compound(v) => {
            for (k, x) in v {
                out.push(id(x));
                string(out, k);
                payload(out, x)
            }
            out.push(0)
        }
    }
}
fn nbt(t: Tag) -> Vec<u8> {
    let mut out = vec![10, 0, 0];
    payload(&mut out, &t);
    out
}
fn sponge(version: i32, dims: [i16; 3], palette: Vec<(&str, i32)>, data: Vec<u8>) -> Tag {
    let p = c(palette
        .into_iter()
        .map(|(name, id)| (name, Tag::Int(id)))
        .collect());
    let mut fields = vec![
        ("Version", Tag::Int(version)),
        ("DataVersion", Tag::Int(4000)),
        ("Width", Tag::Short(dims[0])),
        ("Height", Tag::Short(dims[1])),
        ("Length", Tag::Short(dims[2])),
        ("Offset", Tag::Ints(vec![9, -2, 4])),
    ];
    if version == 2 {
        fields.extend([("Palette", p), ("BlockData", Tag::Bytes(data))]);
        c(fields)
    } else {
        fields.push((
            "Blocks",
            c(vec![
                ("Palette", p),
                ("Data", Tag::Bytes(data)),
                ("BlockEntities", Tag::List(10, vec![c(vec![])])),
            ]),
        ));
        c(vec![("Schematic", c(fields))])
    }
}
fn region(pos: [i32; 3], size: [i32; 3], palette: Vec<Tag>, words: Vec<u64>) -> Tag {
    c(vec![
        ("Position", xyz(pos)),
        ("Size", xyz(size)),
        ("BlockStatePalette", Tag::List(10, palette)),
        ("BlockStates", Tag::Longs(words)),
        ("TileEntities", Tag::List(10, vec![c(vec![])])),
        ("Entities", Tag::List(10, vec![c(vec![])])),
    ])
}
fn litematic(version: i32, regions: Vec<(&str, Tag)>) -> Tag {
    c(vec![
        ("Version", Tag::Int(version)),
        ("Regions", c(regions)),
    ])
}
fn simple() -> Schematic {
    Schematic {
        name: "fixture".into(),
        size: [2, 1, 3],
        palette: vec![
            BlockState {
                name: "minecraft:air".into(),
                properties: BTreeMap::new(),
            },
            BlockState {
                name: "minecraft:oak_stairs".into(),
                properties: BTreeMap::from([
                    ("facing".into(), "north".into()),
                    ("shape".into(), "inner_left".into()),
                ]),
            },
        ],
        blocks: vec![0, 1, 1, 0, 0, 1],
        ignored_block_entities: 0,
        ignored_entities: 0,
    }
}

#[test]
fn sponge_versions_sparse_palette_ids_and_properties_are_exact() {
    for version in [2, 3] {
        let got = decode(&nbt(sponge(
            version,
            [2, 1, 1],
            vec![
                ("minecraft:oak_log[waterlogged=false,axis=x]", 129),
                ("minecraft:air", 4),
            ],
            vec![129, 1, 4],
        )))
        .unwrap();
        assert_eq!(got.size, [2, 1, 1]);
        assert_eq!(
            got.palette[got.blocks[0] as usize].descriptor(),
            "minecraft:oak_log[axis=x,waterlogged=false]"
        );
        assert_eq!(got.palette[got.blocks[1] as usize].name, "minecraft:air");
        assert_eq!(got.ignored_block_entities, if version == 3 { 1 } else { 0 });
    }
}
#[test]
fn vanilla_sparse_structure_uses_void_and_counts_ignored_payloads() {
    let root = c(vec![
        ("size", list_int([3, 1, 1])),
        (
            "palette",
            Tag::List(10, vec![state("minecraft:stone", &[])]),
        ),
        (
            "blocks",
            Tag::List(
                10,
                vec![c(vec![
                    ("pos", list_int([1, 0, 0])),
                    ("state", Tag::Int(0)),
                    ("nbt", c(vec![("LootTable", s("ignored"))])),
                ])],
            ),
        ),
        ("entities", Tag::List(10, vec![c(vec![])])),
    ]);
    let got = decode(&nbt(root)).unwrap();
    assert_eq!(got.size, [3, 1, 1]);
    assert_eq!(
        got.palette[got.blocks[0] as usize].name,
        "minecraft:structure_void"
    );
    assert_eq!(got.palette[got.blocks[1] as usize].name, "minecraft:stone");
    assert_eq!(got.ignored_entities, 1);
    assert_eq!(got.ignored_block_entities, 1);
}
#[test]
fn litematic_4_to_7_signed_extents_and_cross_word_bitstream() {
    // 17 states -> 5-bit IDs. Entry12 starts at bit60 and crosses the u64 boundary.
    // IDs0..15 once, then0: literal words hand-checked independently.
    let words = vec![0xc5a9_2839_8a41_8820, 0x7b9a];
    for version in 4..=7 {
        let palette = (0..17)
            .map(|i| state(&format!("fixture:block_{i}"), &[]))
            .collect();
        let got = decode(&nbt(litematic(
            version,
            vec![(
                "negative",
                region([20, 2, -4], [-17, 1, 1], palette, words.clone()),
            )],
        )))
        .unwrap();
        assert_eq!(got.size, [17, 1, 1]);
        for (i, &block) in got.blocks.iter().enumerate() {
            assert_eq!(
                got.palette[block as usize].name,
                format!("fixture:block_{}", if i < 16 { i } else { 0 })
            );
        }
        assert_eq!(got.ignored_entities, 1);
        assert_eq!(got.ignored_block_entities, 1);
    }
}
#[test]
fn litematic_disjoint_regions_preserve_void_and_reject_any_overlap() {
    let r = region(
        [0, 0, 0],
        [1, 1, 1],
        vec![state("minecraft:stone", &[])],
        vec![0],
    );
    let r2 = region(
        [2, 0, 0],
        [1, 1, 1],
        vec![state("minecraft:air", &[])],
        vec![0],
    );
    let got = decode(&nbt(litematic(7, vec![("a", r.clone()), ("b", r2)]))).unwrap();
    assert_eq!(got.size, [3, 1, 1]);
    assert_eq!(
        got.palette[got.blocks[1] as usize].name,
        "minecraft:structure_void"
    );
    assert!(decode(&nbt(litematic(7, vec![("a", r.clone()), ("b", r)]))).is_err());
}
#[test]
fn malformed_sponge_ids_varints_duplicates_and_lengths_are_rejected() {
    for (palette, data) in [
        (vec![("minecraft:stone", 0)], vec![1]),
        (vec![("minecraft:stone", 0), ("minecraft:dirt", 0)], vec![0]),
        (vec![("minecraft:stone", 0)], vec![128]),
        (vec![("minecraft:stone", 0)], vec![128, 128, 128, 128, 16]),
        (vec![("minecraft:stone", 0)], vec![128, 0]),
        (vec![("minecraft:stone", 0)], vec![0, 0]),
        (vec![("minecraft:stone[a=x,a=y]", 0)], vec![0]),
    ] {
        assert!(decode(&nbt(sponge(2, [1, 1, 1], palette, data))).is_err());
    }
    assert!(
        decode(&nbt(sponge(
            2,
            [-1, 1, 1],
            vec![("minecraft:air", 0)],
            vec![0]
        )))
        .is_err()
    );
    let mut root = match sponge(2, [1, 1, 1], vec![("minecraft:air", 0)], vec![0]) {
        Tag::Compound(v) => v,
        _ => unreachable!(),
    };
    root.push(("Version".into(), Tag::Int(2)));
    assert!(decode(&nbt(Tag::Compound(root))).is_err());
    let valid = nbt(sponge(2, [1, 1, 1], vec![("minecraft:air", 0)], vec![0]));
    assert!(decode(&valid[..valid.len() - 1]).is_err());
    let mut trailing = valid;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
}
#[test]
fn transform_non_square_coordinates_and_directional_states() {
    let input = simple();
    let got = input.transformed(1, false).unwrap();
    assert_eq!(got.size, [3, 1, 2]);
    assert_eq!(got.blocks, vec![0, 1, 0, 1, 0, 1]);
    assert_eq!(
        got.palette[1].descriptor(),
        "minecraft:oak_stairs[facing=east,shape=inner_left]"
    );
    let mirrored = input.transformed(0, true).unwrap();
    assert_eq!(mirrored.blocks, vec![1, 0, 0, 1, 1, 0]);
    assert_eq!(
        mirrored.palette[1].descriptor(),
        "minecraft:oak_stairs[facing=north,shape=inner_right]"
    );
    assert_eq!(input.transformed(4, false).unwrap(), input);
    assert!(
        input
            .transformed(1, false)
            .unwrap()
            .transformed(3, false)
            .unwrap()
            == input
    );
    let mut directional = input;
    directional.palette[1] = BlockState {
        name: "minecraft:oak_door".into(),
        properties: BTreeMap::from([
            ("facing".into(), "east".into()),
            ("hinge".into(), "left".into()),
            ("axis".into(), "x".into()),
            ("rotation".into(), "1".into()),
        ]),
    };
    assert_eq!(
        directional.transformed(1, true).unwrap().palette[1].descriptor(),
        "minecraft:oak_door[axis=z,facing=north,hinge=right,rotation=3]"
    );
    directional.palette[1]
        .properties
        .insert("orientation".into(), "down_east".into());
    assert!(directional.transformed(1, false).is_err());
}
#[test]
fn export_roundtrip_preserves_cells_states_and_sorted_totals() {
    let input = simple();
    input.validate().unwrap();
    let encoded = encode_sponge(&input, 4000).unwrap();
    let got = decode(&encoded).unwrap();
    assert_eq!(got.size, input.size);
    assert_eq!(got.blocks, input.blocks);
    assert_eq!(got.palette, input.palette);
    assert_eq!(got.name, input.name);
    assert_eq!(
        got.block_totals(),
        BTreeMap::from([
            ("minecraft:air".into(), 3),
            (
                "minecraft:oak_stairs[facing=north,shape=inner_left]".into(),
                3
            )
        ])
    );
    let mut bad = input;
    bad.blocks[0] = 2;
    assert!(bad.validate().is_err());
    assert!(encode_sponge(&bad, 4000).is_err());
}

fn gzip(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut writer = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    writer.write_all(data).unwrap();
    writer.finish().unwrap()
}
#[test]
fn gzip_validates_crc_footer_single_member_and_expansion_budget() {
    let raw = nbt(sponge(2, [1, 1, 1], vec![("minecraft:air", 0)], vec![0]));
    let packed = gzip(&raw);
    assert!(decode(&packed).is_ok());
    assert!(decode(&packed[..packed.len() - 1]).is_err());
    let mut corrupt = packed.clone();
    let i = corrupt.len() - 8;
    corrupt[i] ^= 1;
    assert!(decode(&corrupt).is_err());
    let mut tail = packed.clone();
    tail.push(0);
    assert!(decode(&tail).is_err());
    let mut two = packed.clone();
    two.extend(packed);
    assert!(decode(&two).is_err());
    let bomb = gzip(&vec![0; MAX_DECOMPRESSED_BYTES + 1]);
    assert!(decode(&bomb).is_err());
    assert!(decode(&vec![0; MAX_INPUT_BYTES + 1]).is_err());
}
#[test]
fn mutf8_metadata_handles_nul_and_surrogate_pairs_and_rejects_bad_encodings() {
    let root = sponge(2, [1, 1, 1], vec![("minecraft:air", 0)], vec![0]);
    let Tag::Compound(mut entries) = root else {
        unreachable!()
    };
    entries.push(("Metadata".into(), c(vec![("Name", s("MARK"))])));
    let original = nbt(Tag::Compound(entries));
    let offset = original.windows(4).position(|x| x == b"MARK").unwrap();
    // NUL followed by U+1F680 ROCKET, each encoded as Java writeUTF code units.
    let mut modified = original[..offset - 2].to_vec();
    modified.extend([0, 8, 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xba, 0x80]);
    modified.extend(&original[offset + 4..]);
    let got = decode(&modified).unwrap();
    assert_eq!(got.name, "\0🚀");
    assert_eq!(
        decode(&encode_sponge(&got, 4000).unwrap()).unwrap().name,
        got.name
    );
    for bytes in [
        &[0xed, 0xa0, 0xbd][..],
        &[0xc1, 0x81],
        &[0xf0, 0x9f, 0x9a, 0x80],
    ] {
        let mut bad = original[..offset - 2].to_vec();
        bad.extend((bytes.len() as u16).to_be_bytes());
        bad.extend(bytes);
        bad.extend(&original[offset + 4..]);
        assert!(decode(&bad).is_err());
    }
}
#[test]
fn malformed_nbt_and_coordinate_budgets_reject_before_activation() {
    let mut absurd = vec![10, 0, 0, 9, 0, 1, b'a', 10];
    absurd.extend(i32::MAX.to_be_bytes());
    assert!(decode(&absurd).is_err());
    let mut deep = vec![10, 0, 0];
    for _ in 0..MAX_NBT_DEPTH {
        deep.extend([10, 0, 1, b'a']);
    }
    deep.extend(vec![0; MAX_NBT_DEPTH + 1]);
    assert!(decode(&deep).is_err());
    for (pos, size, words) in [
        ([0, 0, 0], [i32::MIN, 1, 1], vec![0]),
        ([i32::MAX, 0, 0], [2, 1, 1], vec![0]),
        ([0, 0, 0], [1, 1, 1], vec![]),
        ([0, 0, 0], [1, 1, 1], vec![3]),
    ] {
        assert!(
            decode(&nbt(litematic(
                7,
                vec![(
                    "r",
                    region(pos, size, vec![state("minecraft:air", &[])], words)
                )]
            )))
            .is_err()
        );
    }
    let mut too_big = simple();
    too_big.size = [256, 256, 256];
    assert!(too_big.validate().is_err());
    let block = c(vec![("pos", list_int([0, 0, 0])), ("state", Tag::Int(0))]);
    for blocks in [
        vec![block.clone(), block],
        vec![c(vec![
            ("pos", list_int([-1, 0, 0])),
            ("state", Tag::Int(0)),
        ])],
    ] {
        assert!(
            decode(&nbt(c(vec![
                ("size", list_int([1, 1, 1])),
                (
                    "palette",
                    Tag::List(10, vec![state("minecraft:stone", &[])])
                ),
                ("blocks", Tag::List(10, blocks))
            ])))
            .is_err()
        );
    }
}

#[test]
fn export_payload_matches_independent_sponge3_bytes() {
    use std::io::Read;
    let input = simple();
    let packed = encode_sponge(&input, 4000).unwrap();
    let mut actual = Vec::new();
    flate2::read::GzDecoder::new(&packed[..])
        .read_to_end(&mut actual)
        .unwrap();
    let expected = c(vec![(
        "Schematic",
        c(vec![
            ("Version", Tag::Int(3)),
            ("DataVersion", Tag::Int(4000)),
            ("Width", Tag::Short(2)),
            ("Height", Tag::Short(1)),
            ("Length", Tag::Short(3)),
            ("Offset", Tag::Ints(vec![0, 0, 0])),
            ("Metadata", c(vec![("Name", s("fixture"))])),
            (
                "Blocks",
                c(vec![
                    (
                        "Palette",
                        c(vec![
                            ("minecraft:air", Tag::Int(0)),
                            (
                                "minecraft:oak_stairs[facing=north,shape=inner_left]",
                                Tag::Int(1),
                            ),
                        ]),
                    ),
                    ("Data", Tag::Bytes(vec![0, 1, 1, 0, 0, 1])),
                ]),
            ),
        ]),
    )]);
    assert_eq!(actual, nbt(expected));
}
#[test]
fn variant_zero_is_deterministic_and_unresolved_states_keep_exact_properties() {
    let root = c(vec![
        ("size", list_int([1, 1, 1])),
        (
            "palettes",
            Tag::List(
                9,
                vec![
                    Tag::List(
                        10,
                        vec![state("uninstalled:custom_block", &[("enabled", "true")])],
                    ),
                    Tag::List(10, vec![state("minecraft:air", &[])]),
                ],
            ),
        ),
        (
            "blocks",
            Tag::List(
                10,
                vec![c(vec![
                    ("pos", list_int([0, 0, 0])),
                    ("state", Tag::Int(0)),
                ])],
            ),
        ),
    ]);
    let got = decode(&nbt(root)).unwrap();
    assert_eq!(
        got.palette[0].descriptor(),
        "uninstalled:custom_block[enabled=true]"
    );
    assert_eq!(got.palette[0].properties.len(), 1);
}
#[test]
fn rail_connection_transform_and_inverse_preserve_geometry() {
    let mut model = Schematic {
        name: "connections".into(),
        size: [1, 1, 1],
        palette: vec![BlockState {
            name: "minecraft:rail".into(),
            properties: BTreeMap::from([("shape".into(), "ascending_west".into())]),
        }],
        blocks: vec![0],
        ignored_block_entities: 0,
        ignored_entities: 0,
    };
    assert_eq!(
        model.transformed(1, true).unwrap().palette[0].descriptor(),
        "minecraft:rail[shape=ascending_south]"
    );
    model.palette[0].properties = BTreeMap::from([("shape".into(), "north_east".into())]);
    assert_eq!(
        model.transformed(1, true).unwrap().palette[0].descriptor(),
        "minecraft:rail[shape=north_east]"
    );
    model.palette[0].properties = BTreeMap::from([
        ("north".into(), "low".into()),
        ("east".into(), "none".into()),
        ("south".into(), "tall".into()),
        ("west".into(), "low".into()),
    ]);
    assert_eq!(
        model.transformed(1, true).unwrap().palette[0].descriptor(),
        "minecraft:rail[east=low,north=none,south=low,west=tall]"
    );
    for rotation in 0..4 {
        assert_eq!(
            model
                .transformed(rotation, false)
                .unwrap()
                .transformed((4 - rotation) % 4, false)
                .unwrap(),
            model
        );
        assert_eq!(
            model
                .transformed(rotation, true)
                .unwrap()
                .transformed(rotation, true)
                .unwrap(),
            model
        );
    }
}
#[test]
fn litematic_one_to_three_entry_palettes_and_unsigned_high_bits() {
    for (count, word, expected) in [
        (1, 0, 0),
        (2, 0x4000_0000_0000_0000, 1),
        (3, 0x8000_0000_0000_0000, 2),
    ] {
        let palette = (0..count)
            .map(|i| state(&format!("fixture:block_{i}"), &[]))
            .collect();
        let got = decode(&nbt(litematic(
            7,
            vec![("r", region([0, 0, 0], [32, 1, 1], palette, vec![word]))],
        )))
        .unwrap();
        assert_eq!(got.blocks.len(), 32);
        assert_eq!(
            got.palette[got.blocks[31] as usize].name,
            format!("fixture:block_{expected}")
        );
    }
}
