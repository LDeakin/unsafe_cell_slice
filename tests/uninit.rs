use std::mem::MaybeUninit;

use rayon::prelude::*;
use unsafe_cell_slice::UnsafeCellSlice;

const N: usize = 64;

#[test]
fn spare_capacity_ser() {
    let mut data: Vec<u32> = Vec::with_capacity(N);
    {
        let spare = UnsafeCellSlice::new(&mut data.spare_capacity_mut()[..N]);
        let (lo, hi) = unsafe { (spare.index_mut(..N / 2), spare.index_mut(N / 2..)) };
        for (i, v) in hi.iter_mut().enumerate() {
            v.write((N / 2 + i) as u32);
        }
        for (i, v) in lo.iter_mut().enumerate() {
            v.write(i as u32);
        }
    }
    // SAFETY: all `N` elements were written above.
    unsafe { data.set_len(N) };
    assert!(data.iter().enumerate().all(|(i, &v)| v == i as u32));
}

#[test]
fn spare_capacity_par() {
    let mut data: Vec<u32> = Vec::with_capacity(N);
    {
        let spare = UnsafeCellSlice::new(&mut data.spare_capacity_mut()[..N]);
        (0..N).into_par_iter().for_each(|i| {
            // SAFETY: each index is written by exactly one task.
            let v: &mut MaybeUninit<u32> = unsafe { spare.index_mut(i) };
            v.write(i as u32);
        });
    }
    // SAFETY: all `N` elements were written above.
    unsafe { data.set_len(N) };
    assert!(data.iter().enumerate().all(|(i, &v)| v == i as u32));
}

#[test]
fn from_round_trip_ser() {
    let mut data = vec![0u16; 4];
    {
        let init = UnsafeCellSlice::new(&mut data);
        let uninit: UnsafeCellSlice<MaybeUninit<u16>> = init.into();
        assert_eq!(uninit.len(), 4);
        let (a, b) = unsafe { (uninit.index_mut(1), uninit.index_mut(2..)) };
        a.write(7);
        for v in b {
            v.write(9);
        }
    }
    assert_eq!(data, [0, 7, 9, 9]);
}

/// Accepts both initialised and uninitialised bytes.
fn write_first<'a>(bytes: impl Into<UnsafeCellSlice<'a, MaybeUninit<u8>>>, value: u8) {
    let bytes = bytes.into();
    unsafe { bytes.index_mut(0) }.write(value);
}

#[test]
fn from_into_generic_ser() {
    // The reflexive `From` impl still applies alongside the `MaybeUninit` one.
    let mut uninit = [MaybeUninit::<u8>::uninit(); 2];
    write_first(UnsafeCellSlice::new(&mut uninit), 1);
    assert_eq!(unsafe { uninit[0].assume_init() }, 1);

    let mut init = [0u8; 2];
    write_first(UnsafeCellSlice::new(&mut init), 2);
    assert_eq!(init, [2, 0]);
}

#[test]
fn write_copy_of_slice_at_ser() {
    let mut data: Vec<u32> = Vec::with_capacity(N);
    {
        let spare = UnsafeCellSlice::new(&mut data.spare_capacity_mut()[..N]);
        let src: Vec<u32> = (0..N as u32).collect();
        unsafe {
            spare.write_copy_of_slice_at(N / 2, &src[N / 2..]);
            spare.write_copy_of_slice_at(0, &src[..N / 2]);
            spare.write_copy_of_slice_at(N, &[]);
        }
    }
    // SAFETY: all `N` elements were written above.
    unsafe { data.set_len(N) };
    assert!(data.iter().enumerate().all(|(i, &v)| v == i as u32));
}

#[test]
fn write_copy_of_slice_at_par() {
    const CHUNK: usize = 4;
    let mut data: Vec<u32> = Vec::with_capacity(N);
    {
        let spare = UnsafeCellSlice::new(&mut data.spare_capacity_mut()[..N]);
        (0..N / CHUNK).into_par_iter().for_each(|c| {
            let src: Vec<u32> = (c * CHUNK..(c + 1) * CHUNK).map(|i| i as u32).collect();
            // SAFETY: each chunk is written by exactly one task.
            unsafe { spare.write_copy_of_slice_at(c * CHUNK, &src) };
        });
    }
    // SAFETY: all `N` elements were written above.
    unsafe { data.set_len(N) };
    assert!(data.iter().enumerate().all(|(i, &v)| v == i as u32));
}

#[test]
fn write_copy_of_slice_at_from_init_ser() {
    let mut data = vec![0u16; 4];
    {
        let uninit: UnsafeCellSlice<MaybeUninit<u16>> = UnsafeCellSlice::new(&mut data).into();
        unsafe { uninit.write_copy_of_slice_at(1, &[7, 9]) };
    }
    assert_eq!(data, [0, 7, 9, 0]);
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn write_copy_of_slice_at_oob_ser() {
    let mut data = [MaybeUninit::<u8>::uninit(); 2];
    let data = UnsafeCellSlice::new(&mut data);
    unsafe { data.write_copy_of_slice_at(1, &[1, 2]) };
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn write_copy_of_slice_at_overflow_ser() {
    let mut data = [MaybeUninit::<u8>::uninit(); 2];
    let data = UnsafeCellSlice::new(&mut data);
    unsafe { data.write_copy_of_slice_at(usize::MAX, &[1]) };
}
