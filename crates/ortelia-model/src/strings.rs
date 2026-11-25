//! The string arena: all the strings of a store, packed in large blocks.
//!
//! - Blocks of 1 MiB, each a `String` created with its final capacity and never grown: nothing
//!   is copied when the arena grows, and there is no peak from a capacity doubling.
//! - A string is designated by a [`StrRef`] of 8 bytes: block number, start, length.
//! - A string too long for a block gets a block of its own, of its exact size.
//! - Reading gives a `&str` without `unsafe`: a block is a `String`, so Rust guarantees UTF-8,
//!   and slicing only checks its two bounds, in constant time.
//! - The arena only grows: a replaced string stays readable. Its space is only recovered by
//!   reading the saved file again.

/// Size of a regular block, in bytes.
const BLOCK_SIZE: usize = 1 << 20;

/// Bit widths of the fields of a [`StrRef`]: 24 bits of block, 20 of start, 20 of length.
const START_BITS: u32 = 20;
const LEN_BITS: u32 = 20;
const FIELD_MASK: u64 = (1 << 20) - 1;

/// Length field of a string that fills a block of its own (its length is the block's).
/// Strings of this length or longer get their own block.
const OWN_BLOCK: usize = (1 << LEN_BITS) - 1;

/// Maximum number of blocks: 2^24 blocks of 1 MiB, or 16 TiB.
const MAX_BLOCKS: usize = 1 << 24;

/// Reference to a string of a [`StringArena`], packed in 8 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrRef(u64);

impl StrRef {
    /// The empty string, which needs no space.
    pub const EMPTY: StrRef = StrRef(0);

    fn new(block: usize, start: usize, len: usize) -> StrRef {
        // The callers keep every field within its bits.
        StrRef(
            ((block as u64) << (START_BITS + LEN_BITS)) | ((start as u64) << LEN_BITS) | len as u64,
        )
    }

    fn block(self) -> usize {
        (self.0 >> (START_BITS + LEN_BITS)) as usize
    }

    fn start(self) -> usize {
        ((self.0 >> LEN_BITS) & FIELD_MASK) as usize
    }

    fn len(self) -> usize {
        (self.0 & FIELD_MASK) as usize
    }

    /// The same string once its arena has been absorbed by another, whose blocks came first:
    /// its block number moves by `offset` ([`StringArena::absorb`]).
    pub(crate) fn shifted(self, offset: usize) -> StrRef {
        if self == StrRef::EMPTY {
            // Designates no block.
            return self;
        }
        StrRef::new(self.block() + offset, self.start(), self.len())
    }
}

/// The strings of a store.
#[derive(Debug, Default)]
pub struct StringArena {
    blocks: Vec<String>,
    /// The block being filled, if any (a block of its own is never filled further).
    current: Option<usize>,
}

impl StringArena {
    pub fn new() -> StringArena {
        StringArena::default()
    }

    /// Adds a string and returns its reference.
    ///
    /// # Panics
    /// If the arena is full (16 TiB).
    pub fn push(&mut self, text: &str) -> StrRef {
        let len = text.len();
        if len == 0 {
            return StrRef::EMPTY;
        }
        if len >= OWN_BLOCK {
            let block = self.new_block(String::from(text));
            return StrRef::new(block, 0, OWN_BLOCK);
        }
        let block = match self.current {
            Some(block) if self.blocks[block].capacity() - self.blocks[block].len() >= len => block,
            _ => {
                // The rest of the previous block stays unused (less than `len` bytes).
                let block = self.new_block(String::with_capacity(BLOCK_SIZE));
                self.current = Some(block);
                block
            }
        };
        let start = self.blocks[block].len();
        // Within the capacity: the block is never reallocated.
        self.blocks[block].push_str(text);
        StrRef::new(block, start, len)
    }

    /// Reads a string.
    ///
    /// # Panics
    /// If `r` does not come from this arena.
    pub fn get(&self, r: StrRef) -> &str {
        if r == StrRef::EMPTY {
            return "";
        }
        let block = &self.blocks[r.block()];
        if r.len() == OWN_BLOCK {
            block
        } else {
            &block[r.start()..r.start() + r.len()]
        }
    }

    /// Moves the blocks of `other` after those of this arena, without copying their strings, and
    /// returns the offset to apply to the block numbers of `other`'s references
    /// ([`StrRef::shifted`]).
    ///
    /// The block being filled stays this arena's: the last block of `other` is never filled
    /// further, so at most one partly empty block (less than 1 MiB) is lost.
    ///
    /// # Panics
    /// If the arena would hold more than its maximum number of blocks (16 TiB).
    pub(crate) fn absorb(&mut self, other: StringArena) -> usize {
        let offset = self.blocks.len();
        assert!(
            offset + other.blocks.len() <= MAX_BLOCKS,
            "string arena full"
        );
        // Moves each `String` (a pointer, a length, a capacity), not its bytes.
        self.blocks.extend(other.blocks);
        offset
    }

    fn new_block(&mut self, block: String) -> usize {
        assert!(self.blocks.len() < MAX_BLOCKS, "string arena full");
        self.blocks.push(block);
        self.blocks.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_accented_and_long_strings() {
        let mut arena = StringArena::new();
        let empty = arena.push("");
        let accented = arena.push("Brest-Guipavas, côte d’Émeraude ✓");
        let long = "é".repeat(BLOCK_SIZE); // 2 MiB of two-byte characters
        let long_ref = arena.push(&long);
        let after = arena.push("après");
        assert_eq!(arena.get(empty), "");
        assert_eq!(arena.get(accented), "Brest-Guipavas, côte d’Émeraude ✓");
        assert_eq!(arena.get(long_ref), long);
        assert_eq!(arena.get(after), "après");
    }

    #[test]
    fn string_of_exactly_the_own_block_threshold() {
        let mut arena = StringArena::new();
        let text = "x".repeat(OWN_BLOCK);
        let r = arena.push(&text);
        assert_eq!(arena.get(r), text);
        let just_below = "y".repeat(OWN_BLOCK - 1);
        let r2 = arena.push(&just_below);
        assert_eq!(arena.get(r2), just_below);
    }

    #[test]
    fn thousands_of_strings_over_several_blocks() {
        let mut arena = StringArena::new();
        let texts: Vec<String> = (0..20_000)
            .map(|i| format!("chaîne n° {i} {}", "z".repeat(i % 300)))
            .collect();
        let refs: Vec<StrRef> = texts.iter().map(|t| arena.push(t)).collect();
        assert!(arena.blocks.len() > 2, "{} blocks", arena.blocks.len());
        for (text, r) in texts.iter().zip(&refs) {
            assert_eq!(arena.get(*r), text);
        }
    }

    #[test]
    fn absorbed_strings_stay_readable_and_are_not_copied() {
        let mut arena = StringArena::new();
        let mine = arena.push("avant");
        let mut other = StringArena::new();
        let short = other.push("court");
        let long = "l".repeat(OWN_BLOCK + 10);
        let long_ref = other.push(&long);
        let empty = other.push("");
        let address = other.blocks[0].as_ptr();

        let offset = arena.absorb(other);
        assert_eq!(offset, 1);
        assert_eq!(arena.get(mine), "avant");
        assert_eq!(arena.get(short.shifted(offset)), "court");
        assert_eq!(arena.get(long_ref.shifted(offset)), long);
        assert_eq!(arena.get(empty.shifted(offset)), "");
        // The bytes did not move; the block being filled is still the first one.
        assert_eq!(arena.blocks[1].as_ptr(), address);
        let after = arena.push("après");
        assert_eq!(after.block(), 0);
        assert_eq!(arena.get(after), "après");
    }

    #[test]
    fn old_reference_stays_readable_and_blocks_never_grow() {
        let mut arena = StringArena::new();
        let first = arena.push("première");
        for i in 0..100_000 {
            arena.push(&format!("valeur {i}"));
        }
        assert_eq!(arena.get(first), "première");
        assert!(arena.blocks.iter().all(|b| b.capacity() == BLOCK_SIZE));
    }
}
