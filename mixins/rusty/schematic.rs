// Portable named-state schematics. Registry membership is checked by each host adapter.
// Dense storage is x + z*width + y*width*length; sparse gaps are structure_void.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};

mod nbt {
    // Small bounded Java-NBT reader. Arrays borrow the validated input bytes.
    use super::{MAX_DECOMPRESSED_BYTES, MAX_NBT_DEPTH};
    use std::collections::BTreeMap;

    const MAX_TAGS: usize = 4_194_304;
    const MAX_TREE_BYTES: usize = 128 * 1024 * 1024;
    pub(super) type Compound<'a> = BTreeMap<String, Tag<'a>>;

    #[derive(Debug)]
    pub(super) enum Tag<'a> {
        Unused,
        Short(i16),
        Int(i32),
        String(String),
        Bytes(&'a [u8]),
        Ints(&'a [u8]),
        Longs(&'a [u8]),
        List(u8, Vec<Tag<'a>>),
        Compound(Compound<'a>),
    }

    pub(super) fn parse(bytes: &[u8]) -> Result<Compound<'_>, String> {
        if bytes.len() > MAX_DECOMPRESSED_BYTES {
            return Err("NBT exceeds expanded byte budget".into());
        }
        let mut r = Reader {
            bytes,
            cursor: 0,
            tags: 0,
            allocated: 0,
        };
        if r.byte()? != 10 {
            return Err("NBT root must be a named compound".into());
        }
        r.string()?;
        let root = r.tag(10, 1)?;
        if r.cursor != bytes.len() {
            return Err("trailing bytes after NBT root".into());
        }
        match root {
            Tag::Compound(c) => Ok(c),
            _ => unreachable!(),
        }
    }

    struct Reader<'a> {
        bytes: &'a [u8],
        cursor: usize,
        tags: usize,
        allocated: usize,
    }
    impl<'a> Reader<'a> {
        fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
            let end = self.cursor.checked_add(len).ok_or("NBT length overflow")?;
            let data = self
                .bytes
                .get(self.cursor..end)
                .ok_or("truncated NBT payload")?;
            self.cursor = end;
            Ok(data)
        }
        fn byte(&mut self) -> Result<u8, String> {
            Ok(self.take(1)?[0])
        }
        fn length(&mut self) -> Result<usize, String> {
            let n = i32::from_be_bytes(self.take(4)?.try_into().unwrap());
            usize::try_from(n).map_err(|_| "negative NBT length".into())
        }
        fn charge(&mut self, bytes: usize) -> Result<(), String> {
            self.allocated = self
                .allocated
                .checked_add(bytes)
                .ok_or("NBT allocation overflow")?;
            if self.allocated > MAX_TREE_BYTES {
                return Err("NBT tree allocation budget exceeded".into());
            }
            Ok(())
        }
        fn string(&mut self) -> Result<String, String> {
            let len = u16::from_be_bytes(self.take(2)?.try_into().unwrap()) as usize;
            self.charge(len * 3 + 32)?;
            decode_mutf8(self.take(len)?)
        }
        fn array(&mut self, width: usize) -> Result<&'a [u8], String> {
            let count = self.length()?;
            self.take(
                count
                    .checked_mul(width)
                    .ok_or("NBT array length overflow")?,
            )
        }
        fn tag(&mut self, kind: u8, depth: usize) -> Result<Tag<'a>, String> {
            if depth > MAX_NBT_DEPTH {
                return Err("NBT nesting depth exceeded".into());
            }
            self.tags += 1;
            if self.tags > MAX_TAGS {
                return Err("NBT tag count exceeded".into());
            }
            Ok(match kind {
                1 => {
                    self.take(1)?;
                    Tag::Unused
                }
                2 => Tag::Short(i16::from_be_bytes(self.take(2)?.try_into().unwrap())),
                3 => Tag::Int(i32::from_be_bytes(self.take(4)?.try_into().unwrap())),
                4 | 6 => {
                    self.take(8)?;
                    Tag::Unused
                }
                5 => {
                    self.take(4)?;
                    Tag::Unused
                }
                7 => Tag::Bytes(self.array(1)?),
                8 => Tag::String(self.string()?),
                9 => {
                    let element = self.byte()?;
                    let len = self.length()?;
                    if element > 12 || (element == 0 && len != 0) {
                        return Err("invalid NBT list type".into());
                    }
                    if len > MAX_TAGS - self.tags || len > self.bytes.len() - self.cursor {
                        return Err("NBT list length exceeds remaining input/tag budget".into());
                    }
                    // Charge the complete reserved vector before any nested list can reserve.
                    // Charging each node only when visited would undercount pending outer slots.
                    self.charge(len.saturating_mul(std::mem::size_of::<Tag<'_>>()))?;
                    let mut values = Vec::with_capacity(len);
                    for _ in 0..len {
                        values.push(self.tag(element, depth + 1)?);
                    }
                    Tag::List(element, values)
                }
                10 => {
                    let mut values = BTreeMap::new();
                    loop {
                        let element = self.byte()?;
                        if element == 0 {
                            break;
                        }
                        let key = self.string()?;
                        if values.contains_key(&key) {
                            return Err(format!("duplicate NBT tag: {key}"));
                        }
                        self.charge(128)?;
                        let value = self.tag(element, depth + 1)?;
                        values.insert(key, value);
                    }
                    Tag::Compound(values)
                }
                11 => Tag::Ints(self.array(4)?),
                12 => Tag::Longs(self.array(8)?),
                _ => return Err("unknown NBT tag type".into()),
            })
        }
    }

    // Java DataInput.readUTF uses encoded UTF-16 units, not ordinary UTF-8.
    fn decode_mutf8(bytes: &[u8]) -> Result<String, String> {
        let mut units = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let b = bytes[i];
            if (1..=0x7f).contains(&b) {
                units.push(b as u16);
                i += 1;
            } else if (0xc0..=0xdf).contains(&b) {
                let c = *bytes.get(i + 1).ok_or("truncated modified UTF-8")?;
                if c & 0xc0 != 0x80 {
                    return Err("invalid modified UTF-8 continuation".into());
                }
                let u = ((b as u16 & 31) << 6) | (c as u16 & 63);
                if u < 128 && !(b == 0xc0 && c == 0x80) {
                    return Err("overlong modified UTF-8".into());
                }
                units.push(u);
                i += 2;
            } else if (0xe0..=0xef).contains(&b) {
                let c = *bytes.get(i + 1).ok_or("truncated modified UTF-8")?;
                let d = *bytes.get(i + 2).ok_or("truncated modified UTF-8")?;
                if c & 0xc0 != 0x80 || d & 0xc0 != 0x80 {
                    return Err("invalid modified UTF-8 continuation".into());
                }
                let u = ((b as u16 & 15) << 12) | ((c as u16 & 63) << 6) | (d as u16 & 63);
                if u < 2048 {
                    return Err("overlong modified UTF-8".into());
                }
                units.push(u);
                i += 3;
            } else {
                return Err("invalid Java modified UTF-8 sequence".into());
            }
        }
        String::from_utf16(&units).map_err(|_| "unpaired modified UTF-8 surrogate".into())
    }

    pub(super) fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), String> {
        let len: usize = value
            .encode_utf16()
            .map(|u| match u {
                1..=0x7f => 1,
                0..=0x7ff => 2,
                _ => 3,
            })
            .sum();
        let len = u16::try_from(len).map_err(|_| "NBT string exceeds 65535 encoded bytes")?;
        out.extend(len.to_be_bytes());
        for u in value.encode_utf16() {
            match u {
                1..=0x7f => out.push(u as u8),
                0..=0x7ff => out.extend([0xc0 | (u >> 6) as u8, 0x80 | (u & 63) as u8]),
                _ => out.extend([
                    0xe0 | (u >> 12) as u8,
                    0x80 | ((u >> 6) & 63) as u8,
                    0x80 | (u & 63) as u8,
                ]),
            }
        }
        Ok(())
    }
}
use nbt::{Compound, Tag};

pub const MAX_INPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_DECOMPRESSED_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_NBT_DEPTH: usize = 64;
pub const MAX_CELLS: usize = 1_048_576;
pub const MAX_AXIS: u32 = 256;
pub const MAX_PALETTE_ENTRIES: usize = 65_536;
const MAX_REGIONS: usize = 4096;
const MAX_STATE_BYTES: usize = 4096;
const MAX_NAME_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct BlockState {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}
impl BlockState {
    pub fn descriptor(&self) -> String {
        if self.properties.is_empty() {
            return self.name.clone();
        }
        format!(
            "{}[{}]",
            self.name,
            self.properties
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
    fn validate(&self) -> Result<(), String> {
        let (namespace, path) = self
            .name
            .split_once(':')
            .ok_or("block state needs namespace:name")?;
        let namespace_ok = !namespace.is_empty()
            && namespace
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
        let path_ok = !path.is_empty()
            && path
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-/".contains(&b));
        if !namespace_ok || !path_ok {
            return Err("invalid block resource identifier".into());
        }
        if self.properties.iter().any(|(k, v)| !token(k) || !token(v)) {
            return Err("invalid block property key/value".into());
        }
        if self.descriptor().len() > MAX_STATE_BYTES {
            return Err("block state descriptor too long".into());
        }
        Ok(())
    }
    fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_STATE_BYTES {
            return Err("block state descriptor too long".into());
        }
        let (name, props) = match text.split_once('[') {
            Some((name, tail)) => (
                name,
                Some(
                    tail.strip_suffix(']')
                        .ok_or("unterminated block properties")?,
                ),
            ),
            None => (text, None),
        };
        let mut properties = BTreeMap::new();
        if let Some(props) = props.filter(|props| !props.is_empty()) {
            for entry in props.split(',') {
                let (k, v) = entry
                    .split_once('=')
                    .ok_or("block property requires key=value")?;
                if properties.insert(k.into(), v.into()).is_some() {
                    return Err("duplicate block property".into());
                }
            }
        }
        let state = Self {
            name: name.into(),
            properties,
        };
        state.validate()?;
        Ok(state)
    }
}
fn token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Schematic {
    pub name: String,
    pub size: [u32; 3],
    pub palette: Vec<BlockState>,
    pub blocks: Vec<u32>,
    pub ignored_block_entities: u32,
    pub ignored_entities: u32,
}
impl Schematic {
    pub fn validate(&self) -> Result<(), String> {
        let count = volume(self.size)?;
        if self.name.len() > MAX_NAME_BYTES {
            return Err("schematic name too long".into());
        }
        if self.palette.is_empty() || self.palette.len() > MAX_PALETTE_ENTRIES {
            return Err("invalid palette size".into());
        }
        let mut states = BTreeSet::new();
        let mut state_bytes = 0usize;
        for state in &self.palette {
            state.validate()?;
            let descriptor = state.descriptor();
            state_bytes = state_bytes
                .checked_add(descriptor.len())
                .ok_or("palette length overflow")?;
            if state_bytes > MAX_INPUT_BYTES {
                return Err("palette byte budget exceeded".into());
            }
            if !states.insert(descriptor) {
                return Err("duplicate palette state".into());
            }
        }
        if self.blocks.len() != count {
            return Err("block count does not match dimensions".into());
        }
        if self
            .blocks
            .iter()
            .any(|&id| id as usize >= self.palette.len())
        {
            return Err("block palette index out of bounds".into());
        }
        Ok(())
    }

    /// Mirror across the X axis first, then rotate clockwise in 90-degree steps.
    /// Unsupported directional properties fail transactionally, before coordinates change.
    pub fn transformed(&self, rotation: u8, mirror: bool) -> Result<Self, String> {
        self.validate()?;
        let rotation = rotation % 4;
        if rotation == 0 && !mirror {
            return Ok(self.clone());
        }
        let mut out = self.clone();
        for state in &mut out.palette {
            transform_state(state, rotation, mirror)?;
        }
        if rotation % 2 == 1 {
            out.size = [self.size[2], self.size[1], self.size[0]];
        }
        for y in 0..self.size[1] {
            for z in 0..self.size[2] {
                for x in 0..self.size[0] {
                    let (mut nx, mut nz) = (if mirror { self.size[0] - 1 - x } else { x }, z);
                    let (mut width, mut length) = (self.size[0], self.size[2]);
                    for _ in 0..rotation {
                        (nx, nz) = (length - 1 - nz, nx);
                        (width, length) = (length, width);
                    }
                    out.blocks[index([nx, y, nz], out.size)] =
                        self.blocks[index([x, y, z], self.size)];
                }
            }
        }
        out.validate()?;
        Ok(out)
    }

    /// Includes air and structure_void; adapters separately normalize placement materials.
    /// Call validate before consuming a model assembled through its public fields.
    pub fn block_totals(&self) -> BTreeMap<String, u32> {
        let mut totals = BTreeMap::new();
        let mut counts = vec![0u32; self.palette.len()];
        for &id in &self.blocks {
            if let Some(count) = counts.get_mut(id as usize) {
                *count += 1;
            } else {
                *totals
                    .entry(format!("<invalid_palette_index:{id}>"))
                    .or_insert(0) += 1;
            }
        }
        for (state, count) in self.palette.iter().zip(counts) {
            if count != 0 {
                *totals.entry(state.descriptor()).or_insert(0) += count;
            }
        }
        totals
    }
}

fn volume(size: [u32; 3]) -> Result<usize, String> {
    if size.iter().any(|&axis| axis == 0 || axis > MAX_AXIS) {
        return Err("schematic axis must be 1..=256".into());
    }
    let count = size
        .iter()
        .try_fold(1usize, |n, &v| n.checked_mul(v as usize))
        .ok_or("dimension overflow")?;
    if count > MAX_CELLS {
        return Err("schematic exceeds cell budget".into());
    }
    Ok(count)
}
fn index(p: [u32; 3], size: [u32; 3]) -> usize {
    (p[0] + p[2] * size[0] + p[1] * size[0] * size[2]) as usize
}

pub fn decode(bytes: &[u8]) -> Result<Schematic, String> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("schematic exceeds input byte budget".into());
    }
    let expanded;
    let raw = if bytes.starts_with(&[0x1f, 0x8b]) {
        // bufread decoder leaves all bytes after the first member observable in Cursor.
        let mut decoder = flate2::bufread::GzDecoder::new(Cursor::new(bytes));
        let mut output = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = decoder
                .read(&mut chunk)
                .map_err(|e| format!("invalid gzip: {e}"))?;
            if n == 0 {
                break;
            }
            if output.len() + n > MAX_DECOMPRESSED_BYTES {
                return Err("gzip exceeds expanded byte budget".into());
            }
            output.extend_from_slice(&chunk[..n]);
        }
        if decoder.get_ref().position() as usize != bytes.len() {
            return Err("gzip must have one member and no trailing bytes".into());
        }
        expanded = output;
        &expanded[..]
    } else {
        bytes
    };
    let root = nbt::parse(raw)?;
    let schematic = if root.contains_key("Regions") {
        decode_litematic(&root)?
    } else if root.contains_key("size")
        && (root.contains_key("palette") || root.contains_key("palettes"))
    {
        decode_vanilla(&root)?
    } else if let Some(value) = root.get("Schematic") {
        let inner = compound(value)?;
        if int(required(inner, "Version")?)? != 3 {
            return Err("nested Sponge schematic must be version 3".into());
        }
        decode_sponge(inner, 3)?
    } else if root.contains_key("Version") {
        let version = int(required(&root, "Version")?)?;
        if version != 2 {
            return Err("unsupported schematic format/version".into());
        }
        decode_sponge(&root, 2)?
    } else {
        return Err(
            "unsupported schematic format (legacy numeric .schematic is not supported)".into(),
        );
    };
    schematic.validate()?;
    Ok(schematic)
}

fn required<'a, 'b>(c: &'b Compound<'a>, key: &str) -> Result<&'b Tag<'a>, String> {
    c.get(key).ok_or_else(|| format!("missing NBT tag: {key}"))
}
fn compound<'a, 'b>(t: &'b Tag<'a>) -> Result<&'b Compound<'a>, String> {
    match t {
        Tag::Compound(c) => Ok(c),
        _ => Err("expected NBT compound".into()),
    }
}
fn int(t: &Tag<'_>) -> Result<i32, String> {
    match t {
        Tag::Int(n) => Ok(*n),
        _ => Err("expected NBT int".into()),
    }
}
fn short(t: &Tag<'_>) -> Result<u32, String> {
    match t {
        Tag::Short(n) => Ok(*n as u16 as u32),
        _ => Err("expected NBT short".into()),
    }
}
fn text<'a, 'b>(t: &'b Tag<'a>) -> Result<&'b str, String> {
    match t {
        Tag::String(s) => Ok(s),
        _ => Err("expected NBT string".into()),
    }
}
fn list<'a, 'b>(t: &'b Tag<'a>, kind: u8) -> Result<&'b [Tag<'a>], String> {
    match t {
        Tag::List(k, v) if *k == kind || (*k == 0 && v.is_empty()) => Ok(v),
        _ => Err("wrong NBT list element type".into()),
    }
}
fn bytes<'a, 'b>(t: &'b Tag<'a>) -> Result<&'b [u8], String> {
    match t {
        Tag::Bytes(b) => Ok(b),
        _ => Err("expected NBT byte array".into()),
    }
}
fn optional_count(c: &Compound<'_>, key: &str) -> Result<u32, String> {
    c.get(key)
        .map(|v| {
            list(v, 10)
                .and_then(|v| u32::try_from(v.len()).map_err(|_| "entity count overflow".into()))
        })
        .unwrap_or(Ok(0))
}
fn name(c: &Compound<'_>) -> Result<String, String> {
    match c.get("Metadata") {
        Some(meta) => compound(meta)?
            .get("Name")
            .map(|v| text(v).map(String::from))
            .unwrap_or(Ok("Imported schematic".into())),
        None => Ok("Imported schematic".into()),
    }
}
fn state_compound(t: &Tag<'_>) -> Result<BlockState, String> {
    let c = compound(t)?;
    let mut state = BlockState {
        name: text(required(c, "Name")?)?.into(),
        properties: BTreeMap::new(),
    };
    if let Some(p) = c.get("Properties") {
        for (k, v) in compound(p)? {
            state.properties.insert(k.clone(), text(v)?.into());
        }
    }
    state.validate()?;
    Ok(state)
}
fn state_list(t: &Tag<'_>) -> Result<Vec<BlockState>, String> {
    let entries = list(t, 10)?;
    if entries.is_empty() || entries.len() > MAX_PALETTE_ENTRIES {
        return Err("invalid palette size".into());
    }
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .map(|t| {
            let state = state_compound(t)?;
            if !seen.insert(state.descriptor()) {
                return Err("duplicate palette state".into());
            }
            Ok(state)
        })
        .collect()
}
fn coord_list(t: &Tag<'_>) -> Result<[i32; 3], String> {
    let items = list(t, 3)?;
    if items.len() != 3 {
        return Err("coordinate/size list must contain 3 ints".into());
    }
    Ok([int(&items[0])?, int(&items[1])?, int(&items[2])?])
}
fn coord_compound(t: &Tag<'_>) -> Result<[i32; 3], String> {
    let c = compound(t)?;
    Ok([
        int(required(c, "x")?)?,
        int(required(c, "y")?)?,
        int(required(c, "z")?)?,
    ])
}
fn positive_size(v: [i32; 3]) -> Result<[u32; 3], String> {
    let size = [
        u32::try_from(v[0]).map_err(|_| "negative size")?,
        u32::try_from(v[1]).map_err(|_| "negative size")?,
        u32::try_from(v[2]).map_err(|_| "negative size")?,
    ];
    volume(size)?;
    Ok(size)
}
fn check_offset(c: &Compound<'_>) -> Result<(), String> {
    if let Some(t) = c.get("Offset")
        && !matches!(t,Tag::Ints(b) if b.len()==12)
    {
        return Err("Offset must be an int array of length 3".into());
    }
    Ok(())
}

fn decode_sponge(c: &Compound<'_>, version: i32) -> Result<Schematic, String> {
    let size = [
        short(required(c, "Width")?)?,
        short(required(c, "Height")?)?,
        short(required(c, "Length")?)?,
    ];
    let count = volume(size)?;
    int(required(c, "DataVersion")?)?;
    check_offset(c)?;
    let data = if version == 3 {
        compound(required(c, "Blocks")?)?
    } else {
        c
    };
    let mapping = compound(required(data, "Palette")?)?;
    if mapping.is_empty() || mapping.len() > MAX_PALETTE_ENTRIES {
        return Err("invalid palette size".into());
    }
    let mut by_id = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (descriptor, id) in mapping {
        let id = u32::try_from(int(id)?).map_err(|_| "negative Sponge palette id")?;
        let state = BlockState::parse(descriptor)?;
        if !seen.insert(state.descriptor()) {
            return Err("duplicate canonical palette state".into());
        }
        if by_id.insert(id, state).is_some() {
            return Err("duplicate Sponge palette id".into());
        }
    }
    let mut palette = Vec::with_capacity(by_id.len());
    let mut remap = BTreeMap::new();
    for (id, state) in by_id {
        remap.insert(id, palette.len() as u32);
        palette.push(state);
    }
    let stream = bytes(required(
        data,
        if version == 3 { "Data" } else { "BlockData" },
    )?)?;
    let mut cursor = 0;
    let mut blocks = Vec::with_capacity(count);
    for _ in 0..count {
        let id = read_varint(stream, &mut cursor)?;
        blocks.push(
            *remap
                .get(&id)
                .ok_or("Sponge block id is absent from palette")?,
        );
    }
    if cursor != stream.len() {
        return Err("Sponge block data has excess cells/bytes".into());
    }
    Ok(Schematic {
        name: name(c)?,
        size,
        palette,
        blocks,
        ignored_block_entities: optional_count(data, "BlockEntities")?,
        ignored_entities: optional_count(c, "Entities")?,
    })
}
fn read_varint(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let mut value = 0u32;
    for n in 0..5 {
        let b = *data.get(*cursor).ok_or("truncated Sponge varint")?;
        *cursor += 1;
        if n == 4 && b > 7 {
            return Err("Sponge varint overflows nonnegative i32".into());
        }
        value |= ((b & 127) as u32) << (7 * n);
        if b & 128 == 0 {
            if n > 0 && b == 0 {
                return Err("overlong Sponge varint".into());
            }
            return Ok(value);
        }
    }
    Err("overlong Sponge varint".into())
}

fn append_void(palette: &mut Vec<BlockState>) -> Result<u32, String> {
    if let Some(id) = palette
        .iter()
        .position(|s| s.name == "minecraft:structure_void" && s.properties.is_empty())
    {
        return Ok(id as u32);
    }
    if palette.len() == MAX_PALETTE_ENTRIES {
        return Err("no palette room for sparse structure_void".into());
    }
    let id = palette.len() as u32;
    palette.push(BlockState {
        name: "minecraft:structure_void".into(),
        properties: BTreeMap::new(),
    });
    Ok(id)
}
fn fill_gaps(palette: &mut Vec<BlockState>, blocks: &mut [u32]) -> Result<(), String> {
    if blocks.contains(&u32::MAX) {
        let void = append_void(palette)?;
        for id in blocks {
            if *id == u32::MAX {
                *id = void;
            }
        }
    }
    Ok(())
}
fn decode_vanilla(c: &Compound<'_>) -> Result<Schematic, String> {
    let size = positive_size(coord_list(required(c, "size")?)?)?;
    if c.contains_key("palette") && c.contains_key("palettes") {
        return Err("ambiguous vanilla palette variants".into());
    }
    let mut palette = if let Some(p) = c.get("palette") {
        state_list(p)?
    } else {
        let variants = list(required(c, "palettes")?, 9)?;
        if variants.is_empty() {
            return Err("empty vanilla palette variants".into());
        }
        let first = state_list(&variants[0])?;
        for p in &variants[1..] {
            let other = state_list(p)?;
            if other.len() != first.len() {
                return Err("vanilla variant palette lengths differ".into());
            }
        }
        first
    };
    let mut blocks = vec![u32::MAX; volume(size)?];
    let entries = list(required(c, "blocks")?, 10)?;
    if entries.len() > MAX_CELLS {
        return Err("vanilla block-list budget exceeded".into());
    }
    let mut ignored_block_entities = 0;
    for entry in entries {
        let block = compound(entry)?;
        let p = coord_list(required(block, "pos")?)?;
        if (0..3).any(|a| p[a] < 0 || p[a] as u32 >= size[a]) {
            return Err("vanilla position outside schematic bounds".into());
        }
        let state = u32::try_from(int(required(block, "state")?)?)
            .map_err(|_| "negative vanilla state index")?;
        if state as usize >= palette.len() {
            return Err("vanilla palette index out of bounds".into());
        }
        let i = index([p[0] as u32, p[1] as u32, p[2] as u32], size);
        if blocks[i] != u32::MAX {
            return Err("duplicate vanilla block position".into());
        }
        blocks[i] = state;
        if let Some(t) = block.get("nbt") {
            compound(t)?;
            ignored_block_entities += 1;
        }
    }
    fill_gaps(&mut palette, &mut blocks)?;
    Ok(Schematic {
        name: name(c)?,
        size,
        palette,
        blocks,
        ignored_block_entities,
        ignored_entities: optional_count(c, "entities")?,
    })
}

fn decode_litematic(c: &Compound<'_>) -> Result<Schematic, String> {
    let version = int(required(c, "Version")?)?;
    if !(4..=7).contains(&version) {
        return Err("unsupported litematic version (named states 4..=7 only)".into());
    }
    let regions = compound(required(c, "Regions")?)?;
    if regions.is_empty() || regions.len() > MAX_REGIONS {
        return Err("invalid litematic region count".into());
    }
    let mut min = [i32::MAX; 3];
    let mut max = [i32::MIN; 3];
    let mut descriptions = Vec::with_capacity(regions.len());
    let mut total_cells = 0usize;
    for value in regions.values() {
        let region = compound(value)?;
        let pos = coord_compound(required(region, "Position")?)?;
        let signed = coord_compound(required(region, "Size")?)?;
        let mut low = [0; 3];
        let mut size = [0; 3];
        for a in 0..3 {
            size[a] = signed[a]
                .checked_abs()
                .ok_or("litematic signed size overflow")? as u32;
            let end = pos[a]
                .checked_add(
                    signed[a]
                        .checked_sub(signed[a].signum())
                        .ok_or("litematic size overflow")?,
                )
                .ok_or("litematic endpoint overflow")?;
            low[a] = pos[a].min(end);
            min[a] = min[a].min(low[a]);
            max[a] = max[a].max(pos[a].max(end));
        }
        total_cells = total_cells
            .checked_add(volume(size)?)
            .ok_or("litematic cell count overflow")?;
        if total_cells > MAX_CELLS {
            return Err("litematic region cell budget exceeded".into());
        }
        descriptions.push((region, low, size));
    }
    let mut size = [0; 3];
    for a in 0..3 {
        size[a] = u32::try_from(i64::from(max[a]) - i64::from(min[a]) + 1)
            .map_err(|_| "litematic enclosing extent overflow")?;
    }
    let mut blocks = vec![u32::MAX; volume(size)?];
    let mut palette = Vec::new();
    let mut palette_ids = BTreeMap::new();
    let mut ignored_block_entities = 0u32;
    let mut ignored_entities = 0u32;
    for (region, low, local_size) in descriptions {
        let local_palette = state_list(required(region, "BlockStatePalette")?)?;
        let mut remap = Vec::with_capacity(local_palette.len());
        for state in local_palette {
            let descriptor = state.descriptor();
            let id = match palette_ids.get(&descriptor) {
                Some(&id) => id,
                None => {
                    if palette.len() == MAX_PALETTE_ENTRIES {
                        return Err("combined litematic palette exceeds budget".into());
                    }
                    let id = palette.len() as u32;
                    palette.push(state);
                    palette_ids.insert(descriptor, id);
                    id
                }
            };
            remap.push(id);
        }
        let bits = (usize::BITS - (remap.len() - 1).leading_zeros()).max(2) as usize;
        let count = volume(local_size)?;
        let needed = (count * bits).div_ceil(64);
        let words = match required(region, "BlockStates")? {
            Tag::Longs(b) if b.len() == needed * 8 => *b,
            _ => return Err("incorrect litematic packed-long array length".into()),
        };
        let word = |i: usize| u64::from_be_bytes(words[i * 8..i * 8 + 8].try_into().unwrap());
        for y in 0..local_size[1] {
            for z in 0..local_size[2] {
                for x in 0..local_size[0] {
                    let bit = index([x, y, z], local_size) * bits;
                    let shift = bit % 64;
                    let wi = bit / 64;
                    let mut value = word(wi) >> shift;
                    if shift + bits > 64 {
                        value |= word(wi + 1) << (64 - shift);
                    }
                    let id = (value & ((1u64 << bits) - 1)) as usize;
                    let state = *remap
                        .get(id)
                        .ok_or("litematic palette index out of bounds")?;
                    let p = [
                        (i64::from(low[0]) - i64::from(min[0])) as u32 + x,
                        (i64::from(low[1]) - i64::from(min[1])) as u32 + y,
                        (i64::from(low[2]) - i64::from(min[2])) as u32 + z,
                    ];
                    let i = index(p, size);
                    if blocks[i] != u32::MAX {
                        return Err("overlapping litematic regions".into());
                    }
                    blocks[i] = state;
                }
            }
        }
        ignored_block_entities = ignored_block_entities
            .checked_add(optional_count(region, "TileEntities")?)
            .ok_or("block entity count overflow")?;
        ignored_entities = ignored_entities
            .checked_add(optional_count(region, "Entities")?)
            .ok_or("entity count overflow")?;
    }
    fill_gaps(&mut palette, &mut blocks)?;
    Ok(Schematic {
        name: name(c)?,
        size,
        palette,
        blocks,
        ignored_block_entities,
        ignored_entities,
    })
}

fn direction(value: &str, rotation: u8, mirror: bool) -> Result<String, String> {
    let dirs = ["north", "east", "south", "west"];
    if matches!(value, "up" | "down") {
        return Ok(value.into());
    }
    let mut i = dirs
        .iter()
        .position(|&s| s == value)
        .ok_or("unsupported facing value")?;
    if mirror {
        i = (4 - i) % 4;
    }
    Ok(dirs[(i + rotation as usize) % 4].into())
}
fn handed(value: &str) -> Result<String, String> {
    match value {
        "left" => Ok("right".into()),
        "right" => Ok("left".into()),
        _ => Err("unsupported handed state value".into()),
    }
}
fn transform_shape(value: &str, rotation: u8, mirror: bool) -> Result<String, String> {
    match value {
        "straight" => Ok(value.into()),
        "inner_left" | "inner_right" | "outer_left" | "outer_right" => {
            if !mirror {
                return Ok(value.into());
            }
            let (prefix, side) = value.rsplit_once('_').unwrap();
            Ok(format!("{prefix}_{}", handed(side)?))
        }
        "north_south" | "east_west" => Ok(if rotation % 2 == 1 {
            if value == "north_south" {
                "east_west"
            } else {
                "north_south"
            }
        } else {
            value
        }
        .into()),
        s if s.starts_with("ascending_") => Ok(format!(
            "ascending_{}",
            direction(&s[10..], rotation, mirror)?
        )),
        "north_east" | "north_west" | "south_east" | "south_west" => {
            let (a, b) = value.split_once('_').unwrap();
            let a = direction(a, rotation, mirror)?;
            let b = direction(b, rotation, mirror)?;
            let (vertical, horizontal) = if matches!(a.as_str(), "north" | "south") {
                (a, b)
            } else {
                (b, a)
            };
            Ok(format!("{vertical}_{horizontal}"))
        }
        _ => Err("unsupported directional shape property".into()),
    }
}
fn transform_state(state: &mut BlockState, rotation: u8, mirror: bool) -> Result<(), String> {
    let mut properties = BTreeMap::new();
    for (key, value) in &state.properties {
        let new_key = if matches!(key.as_str(), "north" | "east" | "south" | "west") {
            direction(key, rotation, mirror)?
        } else {
            key.clone()
        };
        let new_value = match key.as_str() {
            "facing" | "horizontal_facing" => direction(value, rotation, mirror)?,
            "axis" | "horizontal_axis" => match value.as_str() {
                "x" if rotation % 2 == 1 => "z".into(),
                "z" if rotation % 2 == 1 => "x".into(),
                "x" | "y" | "z" => value.clone(),
                _ => return Err("unsupported axis property".into()),
            },
            "rotation" => {
                let mut v = value
                    .parse::<u8>()
                    .map_err(|_| "unsupported rotation property")?;
                if v >= 16 {
                    return Err("rotation property outside 0..15".into());
                }
                if mirror {
                    v = (16 - v) % 16;
                }
                ((v + rotation * 4) % 16).to_string()
            }
            "hinge" => {
                if mirror {
                    handed(value)?
                } else {
                    if !matches!(value.as_str(), "left" | "right") {
                        return Err("unsupported hinge property".into());
                    }
                    value.clone()
                }
            }
            "shape" => transform_shape(value, rotation, mirror)?,
            "type" if mirror && matches!(value.as_str(), "left" | "right") => handed(value)?,
            "orientation" => {
                return Err(
                    "orientation transform is unsupported; use original orientation".into(),
                );
            }
            _ => {
                if !matches!(
                    key.as_str(),
                    "north" | "east" | "south" | "west" | "up" | "down"
                ) && value
                    .split('_')
                    .any(|v| matches!(v, "north" | "east" | "south" | "west"))
                {
                    return Err(format!("unsupported directional property: {key}"));
                }
                value.clone()
            }
        };
        properties.insert(new_key, new_value);
    }
    state.properties = properties;
    Ok(())
}

/// Export a states-only, single-member gzip Sponge v3 file. Entity data is omitted.
pub fn encode_sponge(schematic: &Schematic, data_version: i32) -> Result<Vec<u8>, String> {
    schematic.validate()?;
    if data_version < 0 {
        return Err("DataVersion must be nonnegative".into());
    }
    let mut out = Vec::new();
    header(&mut out, 10, "")?;
    header(&mut out, 10, "Schematic")?;
    write_int(&mut out, "Version", 3)?;
    write_int(&mut out, "DataVersion", data_version)?;
    for (key, axis) in ["Width", "Height", "Length"]
        .into_iter()
        .zip(schematic.size)
    {
        header(&mut out, 2, key)?;
        out.extend((axis as u16).to_be_bytes());
    }
    header(&mut out, 11, "Offset")?;
    out.extend(3i32.to_be_bytes());
    out.extend([0u8; 12]);
    header(&mut out, 10, "Metadata")?;
    header(&mut out, 8, "Name")?;
    nbt::write_string(&mut out, &schematic.name)?;
    out.push(0);
    header(&mut out, 10, "Blocks")?;
    header(&mut out, 10, "Palette")?;
    for (id, state) in schematic.palette.iter().enumerate() {
        write_int(&mut out, &state.descriptor(), id as i32)?;
    }
    out.push(0);
    let mut data = Vec::with_capacity(schematic.blocks.len());
    for &id in &schematic.blocks {
        let mut v = id;
        loop {
            let b = (v & 127) as u8;
            v >>= 7;
            data.push(if v == 0 { b } else { b | 128 });
            if v == 0 {
                break;
            }
        }
    }
    header(&mut out, 7, "Data")?;
    out.extend((data.len() as i32).to_be_bytes());
    out.extend(data);
    out.extend([0, 0, 0]);
    if out.len() > MAX_DECOMPRESSED_BYTES {
        return Err("encoded NBT exceeds expanded budget".into());
    }
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&out)
        .map_err(|e| format!("gzip encoding failed: {e}"))?;
    let bytes = gzip
        .finish()
        .map_err(|e| format!("gzip encoding failed: {e}"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("encoded schematic exceeds input budget".into());
    }
    Ok(bytes)
}
fn header(out: &mut Vec<u8>, kind: u8, key: &str) -> Result<(), String> {
    out.push(kind);
    nbt::write_string(out, key)
}
fn write_int(out: &mut Vec<u8>, key: &str, value: i32) -> Result<(), String> {
    header(out, 3, key)?;
    out.extend(value.to_be_bytes());
    Ok(())
}
