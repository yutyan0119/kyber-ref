#![allow(dead_code)]

#[cfg(feature = "90s")]
use crate::aes256ctr::*;
#[cfg(not(feature = "90s"))]
use crate::{fips202::*, params::*};
#[cfg(feature = "90s")]
use sha2::{Digest, Sha256, Sha512};

#[cfg(feature = "90s-fixslice")]
use aes::cipher::{generic_array::GenericArray, KeyIvInit, StreamCipher};
#[cfg(feature = "90s-fixslice")]
type Aes256Ctr = ctr::Ctr32BE<aes::Aes256>;

#[cfg(feature = "90s")]
pub const AES256CTR_BLOCKBYTES: usize = 64;

#[cfg(feature = "90s")]
pub const XOF_BLOCKBYTES: usize = AES256CTR_BLOCKBYTES;
#[cfg(not(feature = "90s"))]
pub const XOF_BLOCKBYTES: usize = SHAKE128_RATE;

#[cfg(not(feature = "90s"))]
pub type XofState = KeccakState;

#[cfg(feature = "90s")]
pub type XofState = Aes256CtrCtx;

#[derive(Copy, Clone)]
pub struct KeccakState {
    pub s: [u64; 25],
    pub pos: usize,
}

impl KeccakState {
    pub fn new() -> Self {
        KeccakState {
            s: [0u64; 25],
            pos: 0usize,
        }
    }

    pub fn reset(&mut self) {
        self.s = [0u64; 25];
        self.pos = 0;
    }
}

/// SHA3-256
#[cfg(not(feature = "90s"))]
pub fn hash_h(out: &mut [u8], input: &[u8], inlen: usize) {
    sha3_256(out, input, inlen);
}

/// 90s mode SHA2-256
#[cfg(feature = "90s")]
pub fn hash_h(out: &mut [u8], input: &[u8], inlen: usize) {
    let mut hasher = Sha256::new();
    hasher.update(&input[..inlen]);
    let digest = hasher.finalize();
    out[..digest.len()].copy_from_slice(&digest);
}

#[cfg(not(feature = "90s"))]
pub fn hash_g(out: &mut [u8], input: &[u8], inlen: usize) {
    sha3_512(out, input, inlen);
}

#[cfg(feature = "90s")]
pub fn hash_g(out: &mut [u8], input: &[u8], inlen: usize) {
    let mut hasher = Sha512::new();
    hasher.update(&input[..inlen]);
    let digest = hasher.finalize();
    out[..digest.len()].copy_from_slice(&digest);
}

#[cfg(not(feature = "90s"))]
pub fn xof_absorb(state: &mut XofState, input: &[u8], x: u8, y: u8) {
    kyber_shake128_absorb(state, &input, x, y);
}

#[cfg(feature = "90s")]
pub fn xof_absorb(state: &mut XofState, input: &[u8], x: u8, y: u8) {
    let mut nonce = [0u8; 12];
    nonce[0] = x;
    nonce[1] = y;
    aes256ctr_init(state, &input, nonce);
}

#[cfg(not(feature = "90s"))]
pub fn xof_squeezeblocks(out: &mut [u8], outblocks: usize, state: &mut XofState) {
    kyber_shake128_squeezeblocks(out, outblocks, state);
}

#[cfg(feature = "90s")]
pub fn xof_squeezeblocks(out: &mut [u8], outblocks: usize, state: &mut XofState) {
    aes256ctr_squeezeblocks(out, outblocks, state);
}

#[cfg(not(feature = "90s"))]
pub fn prf(out: &mut [u8], outbytes: usize, key: &[u8], nonce: u8) {
    shake256_prf(out, outbytes, &key, nonce);
}

#[cfg(feature = "90s")]
pub fn prf(out: &mut [u8], _outbytes: usize, key: &[u8], nonce: u8) {
    #[cfg(feature = "90s-fixslice")]
    {
        // RustCrypto fixslice
        let mut expnonce = [0u8; 16];
        expnonce[0] = nonce;
        let key = GenericArray::from_slice(key);
        let iv = GenericArray::from_slice(&expnonce);
        let mut cipher = Aes256Ctr::new(&key, &iv);
        cipher.apply_keystream(out);
        return;
    }
    #[cfg(not(feature = "90s-fixslice"))]
    // Pornin bitslice
    aes256ctr_prf(out, _outbytes, &key, nonce);
}

#[cfg(not(feature = "90s"))]
pub fn kdf(out: &mut [u8], input: &[u8], inlen: usize) {
    shake256(out, KYBER_SSBYTES, input, inlen);
}

#[cfg(feature = "90s")]
pub fn kdf(out: &mut [u8], input: &[u8], inlen: usize) {
    let mut hasher = Sha256::new();
    hasher.update(&input[..inlen]);
    let digest = hasher.finalize();
    out[..digest.len()].copy_from_slice(&digest);
}

/// Name:  kyber_shake128_absorb
///
/// Description: Absorb step of the SHAKE128 specialized for the Kyber context.
///
/// Arguments:   - u64 *s:     (uninitialized) output Keccak state
///  - const [u8] input:  KYBER_SYMBYTES input to be absorbed into s
///  - u8  x    additional byte of input
///  - u8  y    additional byte of input
#[cfg(not(feature = "90s"))]
fn kyber_shake128_absorb(
  s: &mut KeccakState,
  input: &[u8],
  x: u8,
  y: u8
)
{
  let mut extseed = [0u8; KYBER_SYMBYTES + 2];
  extseed[..KYBER_SYMBYTES].copy_from_slice(input);
  extseed[KYBER_SYMBYTES] = x;
  extseed[KYBER_SYMBYTES+1] = y;
  println!("shake128_absorb extseed: {:?}", extseed);
  shake128_absorb_once(s, &extseed, KYBER_SYMBYTES + 2);
  let hex_state = s.s.iter().map(|x| format!("{:016x}", x)).collect::<Vec<String>>().join("");
  println!("shak128 endofinput state: {:?}", hex_state);
}

/// Name:  kyber_shake128_squeezeblocks
///
/// Description: Squeeze step of SHAKE128 XOF. Squeezes full blocks of SHAKE128_RATE bytes each.
///  Modifies the state. Can be called multiple times to keep squeezing,
///  i.e., is incremental.
///
/// Arguments:   - [u8] output:  output blocks
///  - u64 nblocks: number of blocks to be squeezed (written to output)
///  - keccak_state *s:  in/output Keccak state
#[cfg(not(feature = "90s"))]
fn kyber_shake128_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut KeccakState) {
    shake128_squeezeblocks(output, nblocks, s);
}

/// Name:  shake256_prf
///
/// Description: Usage of SHAKE256 as a PRF, concatenates secret and public input
///  and then generates outlen bytes of SHAKE256 output
///
/// Arguments:   - [u8] output:  output
///  - u64 outlen:  number of requested output bytes
///  - const [u8]  key:  the key (of length KYBER_SYMBYTES)
///  - const [u8]  nonce:  single-byte nonce (public PRF input)
#[cfg(not(feature = "90s"))]
fn shake256_prf(output: &mut [u8], outlen: usize, key: &[u8], nonce: u8) {
    let mut extkey = [0u8; KYBER_SYMBYTES + 1];
    extkey[..KYBER_SYMBYTES].copy_from_slice(key);
    extkey[KYBER_SYMBYTES] = nonce;
    shake256(output, outlen, &extkey, KYBER_SYMBYTES + 1);
}
