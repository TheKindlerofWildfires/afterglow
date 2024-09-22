use crate::serial::Serial;
use std::{
    cmp::{self, Reverse},
    collections::{btree_map, BTreeMap, BinaryHeap},
    iter::Take,
    num::Saturating,
};

#[derive(Debug, Clone)]
pub struct Tree {
    root: usize,
    pub arena: Vec<NodeElement>,
}
#[derive(Debug, Clone)]
pub struct NodeElement {
    parent: Option<usize>,
    data: Node,
}
#[derive(Debug, Clone)]
enum Node {
    Leaf(Leaf),
    Branch(Branch),
}
#[derive(Debug, Clone)]
struct Leaf {
    data: u8,
}
#[derive(Debug, Clone)]
struct Branch {
    right: usize,
    left: usize,
}

pub type Decoder<'a> = Take<UnboundedDecoder<'a>>;

#[derive(Debug)]
pub struct UnboundedDecoder<'a> {
    tree: &'a Tree,
    iter: std::vec::IntoIter<bool>,
}

impl Tree {
    pub fn bitter_serial(&self)->Bitter{
        let mut bitter = Bitter::new();
        self.serial_helper(&mut bitter, &self.arena[self.root]);
        bitter
    }
    fn serial_helper(&self, bitter: &mut Bitter, node: &NodeElement){
        match &node.data{
            Node::Leaf(leaf) => {
                bitter.push(true);
                bitter.add_u8(leaf.data);
            },
            Node::Branch(branch) => {
                bitter.push(false);
                self.serial_helper(bitter, &self.arena[branch.left]);
                self.serial_helper(bitter, &self.arena[branch.right]);
            },
        }
    }
    pub fn bitter_deserial(bitter: &Bitter,ptr: &mut usize)->Self{
        let mut arena = Vec::new();
        Self::deserial_helper(bitter, ptr, &mut arena);
        Self{
            root: arena.len()-1,
            arena
        }
    }
    fn deserial_helper(bitter: &Bitter, ptr: &mut usize, arena: &mut Vec<NodeElement>)->usize{
        //read the bit at ptr
        match bitter.get(ptr){
            true=>{
                let data = bitter.get_u8(ptr);
                arena.push(NodeElement{
                    parent: None,
                    data: Node::Leaf(Leaf{data})
                })
            },
            false=>{
                let left = Self::deserial_helper(bitter,ptr,arena);
                let right = Self::deserial_helper(bitter,ptr,arena);
                arena[left].parent =Some(arena.len());
                arena[right].parent = Some(arena.len());
                arena.push(NodeElement{
                    parent:None,
                    data: Node::Branch(Branch{right,left})
                });
            }
        }
        arena.len()-1
        //if its 1 use the leaf and return
        //if it's 0 use the branch 
        //for each of the branch returns mark them as parented by this node
    }
    pub fn unbounded_decoder(&self, bitter: Bitter) -> UnboundedDecoder
    {
        UnboundedDecoder {
            tree: self,
            iter: bitter.vec.into_iter(),
        }
    }

    pub fn decoder(&self, bitter: Bitter, num_symbols: usize) -> Decoder
    {
        self.unbounded_decoder(bitter).take(num_symbols)
    }
}

impl<'a> Iterator for UnboundedDecoder<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let mut node = match self.tree.arena.get(self.tree.root) {
            Some(root) => root,
            None => return None,
        };

        loop {
            match &node.data {
                Node::Leaf(leaf) => return Some(leaf.data),
                Node::Branch(branch) => {
                    node = match self.iter.next() {
                        Some(true) => &self.tree.arena[branch.left],
                        Some(false) => &self.tree.arena[branch.right],
                        None => return None,
                    };
                }
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct EncodeError;

#[derive(Clone, Debug)]
pub struct Bitter {
    vec: Vec<bool>,
}

impl Default for Bitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Bitter {
    pub fn new() -> Self {
        let vec = Vec::new();
        Self { vec }
    }
    pub fn push(&mut self, bit: bool) {
        self.vec.push(bit);
    }
    pub fn extend(&mut self, other: &Self) {
        self.vec.extend_from_slice(&other.vec);
    }
    pub fn add_u8(&mut self, byte: u8){
        for i in 0..8{
            let offset = 1<<i;
            self.push(byte&offset == offset)
        }
    }
    pub fn get(&self, ptr: &mut usize)->bool{
        let out = self.vec[*ptr];
        *ptr+=1;
        out
    }
    pub fn len(&self)->usize{
        self.vec.len()
    }
    pub fn is_empty(&self)->bool{
        self.vec.len()==0
    }
    pub fn get_u8(&self, ptr: &mut usize)->u8{
        let mut out = 0;
        for i in 0..8{
            if self.vec[*ptr]{
                out |= 1<<i
            }
            *ptr+=1;
        }
        out
    }
    pub fn forward(&mut self,ptr: usize){
        self.vec = self.vec[ptr..].to_vec();
    }
}

impl Serial for Bitter {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut candidate = 0u8;
        let mut bit_idx = 0;
        self.vec.iter().enumerate().for_each(|(i, &bit)| {
            bit_idx = i & 0x7;
            match bit_idx {
                0 => {
                    if bit {
                        candidate |= 0x1
                    }
                }
                1 => {
                    if bit {
                        candidate |= 0x2
                    }
                }
                2 => {
                    if bit {
                        candidate |= 0x4
                    }
                }
                3 => {
                    if bit {
                        candidate |= 0x8
                    }
                }
                4 => {
                    if bit {
                        candidate |= 0x10
                    }
                }
                5 => {
                    if bit {
                        candidate |= 0x20
                    }
                }
                6 => {
                    if bit {
                        candidate |= 0x40
                    }
                }
                7 => {
                    if bit {
                        candidate |= 0x80
                    }
                    bytes.push(candidate);
                    candidate = 0;
                }
                _ => {
                    unreachable!()
                }
            }
        });
        if bit_idx != 7 {
            bytes.push(candidate);
        }
        bytes
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let mut vec = Vec::new();
        bytes[*start..].iter().for_each(|&byte| {
            vec.push(byte & 0x1 == 0x1);
            vec.push(byte & 0x2 == 0x2);
            vec.push(byte & 0x4 == 0x4);
            vec.push(byte & 0x8 == 0x8);
            vec.push(byte & 0x10 == 0x10);
            vec.push(byte & 0x20 == 0x20);
            vec.push(byte & 0x40 == 0x40);
            vec.push(byte & 0x80 == 0x80);
        });
        Self { vec }
    }
}

#[derive(Clone, Debug)]
pub struct Book {
    book: BTreeMap<u8, Bitter>,
}

impl Book {
    pub fn into_inner(self) -> BTreeMap<u8, Bitter> {
        self.book
    }

    pub fn symbols(&self) -> btree_map::Keys<u8, Bitter> {
        self.book.keys()
    }

    pub fn iter(&self) -> btree_map::Iter<u8, Bitter> {
        self.book.iter()
    }

    pub fn len(&self) -> usize {
        self.book.len()
    }

    pub fn is_empty(&self) -> bool {
        self.book.is_empty()
    }

    pub fn get(&self, k: &u8) -> Option<&Bitter>
    {
        self.book.get(k)
    }

    pub fn contains_symbol(&self, k: &u8) -> bool
    {
        self.book.contains_key(k)
    }

    pub fn encode(&self, buffer: &mut Bitter, k: &u8) -> Result<(), EncodeError>
    {
        match self.book.get(k) {
            Some(code) => buffer.extend(code),
            None => return Err(EncodeError {}),
        }

        Ok(())
    }

    fn new() -> Book {
        Book {
            book: BTreeMap::new(),
        }
    }

    fn build(&mut self, arena: &[NodeElement], node: &NodeElement, word: Bitter) {
        match &node.data {
            Node::Leaf(leaf) => {
                self.book.insert(leaf.data, word);
            }
            Node::Branch(branch) => {
                let mut left_word = word.clone();
                left_word.push(true);
                self.build(arena, &arena[branch.left], left_word);

                let mut right_word = word;
                right_word.push(false);
                self.build(arena, &arena[branch.right], right_word);
            }
        }
    }
}

/// Collects information about symbols and their weights used to construct
/// a Huffman code.
///
/// # Stability
///
/// The constructed code is guaranteed to be deterministic and stable across
/// semver compatible releases if:
///
/// * There is a strict order on the symbols `T`.
/// * No duplicate symbols are added.
///
/// The ordering of symbols will be used to break ties when weights are equal.
#[derive(Debug, Clone)]
pub struct CodeBuilder {
    heap: BinaryHeap<HeapData>,
    arena: Vec<NodeElement>,
}

impl CodeBuilder {
    /// Creates a new, empty `CodeBuilder<T, W>`.
    pub fn new() -> CodeBuilder {
        CodeBuilder {
            heap: BinaryHeap::new(),
            arena: Vec::new(),
        }
    }
    pub fn with_capacity(capacity: usize) -> CodeBuilder {
        CodeBuilder {
            heap: BinaryHeap::with_capacity(capacity),
            arena: Vec::with_capacity(2 * capacity),
        }
    }

    pub fn push(&mut self, symbol: u8, weight: u16) {
        self.heap.push(HeapData {
            weight: Reverse(weight),
            symbol,
            id: self.arena.len(),
        });

        self.arena.push(NodeElement {
            parent: None,
            data: Node::Leaf(Leaf { data: symbol }),
        });
    }

    /// Constructs a [book](struct.Book.html) and [tree](struct.Tree.html) pair
    /// for encoding and decoding.
    pub fn finish(mut self) -> (Book, Tree) {
        let mut book = Book::new();

        let root = loop {
            let left = match self.heap.pop() {
                Some(left) => left,
                None => {
                    return (
                        book,
                        Tree {
                            root: 0,
                            arena: self.arena,
                        },
                    )
                }
            };

            let right = match self.heap.pop() {
                Some(right) => right,
                None => break left,
            };

            let id = self.arena.len();

            self.arena[left.id].parent = Some(id);
            self.arena[right.id].parent = Some(id);
            self.heap.push(HeapData {
                weight: Reverse((Saturating(left.weight.0) + Saturating(right.weight.0)).0),
                symbol: cmp::min(left.symbol, right.symbol),
                id,
            });

            self.arena.push(NodeElement {
                parent: None,
                data: Node::Branch(Branch {
                    left: left.id,
                    right: right.id,
                }),
            });
        };

        book.build(&self.arena, &self.arena[root.id], Bitter::new());
        (
            book,
            Tree {
                root: root.id,
                arena: self.arena,
            },
        )
    }
}

impl Default for CodeBuilder {
    fn default() -> CodeBuilder {
        CodeBuilder::new()
    }
}

impl FromIterator<(u8, u16)> for CodeBuilder {
    fn from_iter<E>(weights: E) -> CodeBuilder
    where
        E: IntoIterator<Item = (u8, u16)>,
    {
        let iter = weights.into_iter();
        let (size_hint, _) = iter.size_hint();
        let mut code = CodeBuilder::with_capacity(size_hint);
        code.extend(iter);
        code
    }
}

impl Extend<(u8, u16)> for CodeBuilder {
    fn extend<E>(&mut self, weights: E)
    where
        E: IntoIterator<Item = (u8, u16)>,
    {
        for (symbol, weight) in weights {
            self.push(symbol, weight);
        }
    }
}

impl<'a> FromIterator<(&'a u8, &'a u16)> for CodeBuilder {
    fn from_iter<E>(weights: E) -> CodeBuilder
    where
        E: IntoIterator<Item = (&'a u8, &'a u16)>,
    {
        CodeBuilder::from_iter(weights.into_iter().map(|(k, v)| (*k, *v)))
    }
}

impl<'a> Extend<(&'a u8, &'a u16)> for CodeBuilder {
    fn extend<E>(&mut self, weights: E)
    where
        E: IntoIterator<Item = (&'a u8, &'a u16)>,
    {
        self.extend(weights.into_iter().map(|(k, v)| (*k, *v)));
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
struct HeapData {
    weight: Reverse<u16>,
    symbol: u8,
    id: usize,
}

impl Clone for HeapData {
    fn clone(&self) -> HeapData {
        HeapData {
            weight: Reverse(self.weight.0),
            symbol: self.symbol,
            id: self.id,
        }
    }
}

pub fn codebook<'a, I>(weights: I) -> (Book, Tree)
where
    I: IntoIterator<Item = (&'a u8, &'a u16)>,
{
    CodeBuilder::from_iter(weights).finish()
}
