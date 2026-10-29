//! Small unkeyed BLAKE3-256 for canonical value identity in the no-WASI guest.
//! Only the ordinary 32-byte hash mode is exposed (no keys, XOF or derivation).

const IV: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

macro_rules! mix {
    ($a:ident, $b:ident, $c:ident, $d:ident, $x:expr, $y:expr) => {
        $a = $a.wrapping_add($b).wrapping_add($x);
        $d = ($d ^ $a).rotate_right(16);
        $c = $c.wrapping_add($d);
        $b = ($b ^ $c).rotate_right(12);
        $a = $a.wrapping_add($b).wrapping_add($y);
        $d = ($d ^ $a).rotate_right(8);
        $c = $c.wrapping_add($d);
        $b = ($b ^ $c).rotate_right(7);
    };
}

#[derive(Clone, Copy)]
struct Output {
    cv: [u32; 8],
    block: [u32; 16],
    counter: u64,
    length: u32,
    flags: u32,
}

impl Output {
    fn compress(&self, root: bool) -> [u32; 16] {
        let [mut s0, mut s1, mut s2, mut s3, mut s4, mut s5, mut s6, mut s7] = self.cv;
        let [mut s8, mut s9, mut s10, mut s11, ..] = IV;
        let counter = self.counter.to_le_bytes();
        let mut s12 = u32::from_le_bytes([counter[0], counter[1], counter[2], counter[3]]);
        let mut s13 = u32::from_le_bytes([counter[4], counter[5], counter[6], counter[7]]);
        let mut s14 = self.length;
        let mut s15 = self.flags | if root { 8 } else { 0 };
        let m = &self.block;
        macro_rules! round {
            ($order:expr) => {
                let order = $order;
                mix!(s0, s4, s8, s12, m[order[0]], m[order[1]]);
                mix!(s1, s5, s9, s13, m[order[2]], m[order[3]]);
                mix!(s2, s6, s10, s14, m[order[4]], m[order[5]]);
                mix!(s3, s7, s11, s15, m[order[6]], m[order[7]]);
                mix!(s0, s5, s10, s15, m[order[8]], m[order[9]]);
                mix!(s1, s6, s11, s12, m[order[10]], m[order[11]]);
                mix!(s2, s7, s8, s13, m[order[12]], m[order[13]]);
                mix!(s3, s4, s9, s14, m[order[14]], m[order[15]]);
            };
        }
        round!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        round!([2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]);
        round!([3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1]);
        round!([10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6]);
        round!([12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4]);
        round!([9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7]);
        round!([11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13]);
        [
            s0 ^ s8,
            s1 ^ s9,
            s2 ^ s10,
            s3 ^ s11,
            s4 ^ s12,
            s5 ^ s13,
            s6 ^ s14,
            s7 ^ s15,
            s8 ^ self.cv[0],
            s9 ^ self.cv[1],
            s10 ^ self.cv[2],
            s11 ^ self.cv[3],
            s12 ^ self.cv[4],
            s13 ^ self.cv[5],
            s14 ^ self.cv[6],
            s15 ^ self.cv[7],
        ]
    }
    fn chaining(&self) -> [u32; 8] {
        self.compress(false)[..8].try_into().unwrap()
    }
}

fn chunk(bytes: &[u8], counter: u64) -> Output {
    let mut cv = IV;
    let blocks = bytes.len().max(1).div_ceil(64);
    for index in 0..blocks {
        let start = index * 64;
        let end = bytes.len().min(start + 64);
        let part = &bytes[start..end];
        let mut padded;
        let full = if part.len() == 64 {
            part
        } else {
            padded = [0; 64];
            padded[..part.len()].copy_from_slice(part);
            &padded
        };
        // Both branches provide 64 initialized bytes. A byte slice has no
        // word alignment guarantee; conversion preserves little-endian BLAKE3
        // words on native test hosts as well as the admitted Wasm target.
        let block = unsafe { full.as_ptr().cast::<[u32; 16]>().read_unaligned() }.map(u32::from_le);
        let output = Output {
            cv,
            block,
            counter,
            length: part.len() as u32,
            flags: (if index == 0 { 1 } else { 0 }) | (if index + 1 == blocks { 2 } else { 0 }),
        };
        if index + 1 == blocks {
            return output;
        }
        cv = output.chaining();
    }
    unreachable!()
}
fn parent(left: [u32; 8], right: [u32; 8]) -> Output {
    let mut block = [0; 16];
    block[..8].copy_from_slice(&left);
    block[8..].copy_from_slice(&right);
    Output {
        cv: IV,
        block,
        counter: 0,
        length: 64,
        flags: 4,
    }
}

pub fn reference(bytes: &[u8]) -> [u8; 71] {
    let count = bytes.len().max(1).div_ceil(1024);
    let mut stack = [[0u32; 8]; core::mem::size_of::<usize>() * 8];
    let mut stack_length = 0;
    for index in 0..count - 1 {
        let mut cv = chunk(&bytes[index * 1024..index * 1024 + 1024], index as u64).chaining();
        let mut total = index + 1;
        while total & 1 == 0 {
            stack_length -= 1;
            cv = parent(stack[stack_length], cv).chaining();
            total >>= 1;
        }
        stack[stack_length] = cv;
        stack_length += 1;
    }
    let index = count - 1;
    let mut output = chunk(&bytes[index * 1024..], index as u64);
    while stack_length != 0 {
        stack_length -= 1;
        output = parent(stack[stack_length], output.chaining());
    }
    let words = output.compress(true);
    let mut text = [0; 71];
    text[..7].copy_from_slice(b"blake3:");
    for (word_index, word) in words[..8].iter().enumerate() {
        for (byte_index, byte) in word.to_le_bytes().into_iter().enumerate() {
            let index = 7 + word_index * 8 + byte_index * 2;
            text[index] = b"0123456789abcdef"[usize::from(byte >> 4)];
            text[index + 1] = b"0123456789abcdef"[usize::from(byte & 15)];
        }
    }
    text
}
