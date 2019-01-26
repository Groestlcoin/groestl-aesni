// copyright 2017 Kaz Wesley

//! Implemenation of the Groestl hash function optimized for x86-64 systems with AES extensions.
//! WARNING: CPU feature detection and portable fallback are left to user!

#![no_std]

pub extern crate digest;

use block_buffer::byteorder::BigEndian;
use block_buffer::generic_array::typenum::{U32, U64};
use block_buffer::generic_array::GenericArray as BBGenericArray;
use block_buffer::BlockBuffer;
use core::mem;
use digest::generic_array::GenericArray as DGenericArray;
pub use digest::Digest;

mod sse2;
use sse2::sse2::{init, of512, tf512};

const ROWS: usize = 8;
const COLS: usize = 8;
const SIZE: usize = ROWS * COLS;
const BITS: u64 = 256;

#[derive(Clone)]
#[repr(C, align(16))]
struct HashState {
    chaining: [u64; SIZE / 8],
    block_counter: u64,
}

impl core::fmt::Debug for HashState {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        f.debug_struct("HashState")
            .field("chaining", &"(array)")
            .field("block_counter", &self.block_counter)
            .finish()
    }
}

#[repr(C, align(16))]
struct Align16<T>(T);

impl Default for HashState {
    fn default() -> Self {
        unsafe {
            let mut iv = Align16([0u64; SIZE / 8]);
            iv.0[COLS - 1] = BITS.to_be();
            Self {
                chaining: mem::transmute(init(mem::transmute(iv))),
                block_counter: 0,
            }
        }
    }
}

impl HashState {
    fn input_block(&mut self, block: &BBGenericArray<u8, U64>) {
        self.block_counter += 1;
        unsafe {
            tf512(
                mem::transmute(&mut self.chaining),
                &*(block.as_ptr() as *const _),
            );
        }
    }

    fn finalize(mut self) -> [u64; SIZE / 8] {
        unsafe {
            of512(mem::transmute(&mut self.chaining));
        }
        self.chaining
    }
}

#[derive(Clone, Default)]
pub struct Groestl256 {
    buffer: BlockBuffer<U64>,
    state: HashState,
}

impl core::fmt::Debug for Groestl256 {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        f.debug_struct("Groestl256")
            .field("state", &self.state)
            .field("buffer", &"(BlockBuffer<U64>)")
            .finish()
    }
}

impl digest::BlockInput for Groestl256 {
    type BlockSize = U64;
}

impl digest::Input for Groestl256 {
    fn input<T: AsRef<[u8]>>(&mut self, data: T) {
        let state = &mut self.state;
        self.buffer.input(data.as_ref(), |b| state.input_block(b));
    }
}

impl digest::FixedOutput for Groestl256 {
    type OutputSize = U32;

    fn fixed_result(self) -> DGenericArray<u8, U32> {
        let mut state = self.state;
        let mut buffer = self.buffer;
        let count = state.block_counter + 1 + (buffer.remaining() <= 8) as u64;
        buffer.len64_padding::<BigEndian, _>(count, |b| state.input_block(b));
        let result = state.finalize();
        let mut out: DGenericArray<u8, U32> = DGenericArray::default();
        for (out, &input) in out.as_mut_slice().chunks_mut(8).zip(&result[4..8]) {
            out[0] = input as u8;
            out[1] = (input >> 8) as u8;
            out[2] = (input >> 16) as u8;
            out[3] = (input >> 24) as u8;
            out[4] = (input >> 32) as u8;
            out[5] = (input >> 40) as u8;
            out[6] = (input >> 48) as u8;
            out[7] = (input >> 56) as u8;
        }
        out
    }
}

impl digest::Reset for Groestl256 {
    fn reset(&mut self) {
        *self = Groestl256::default();
    }
}
