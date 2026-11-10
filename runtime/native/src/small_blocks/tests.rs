use super::*;

/// Fills a block with a pattern made from its address and size.
fn fill(p: *mut u8, bytes: usize) {
    for i in 0..bytes {
        // SAFETY: the block holds `bytes` bytes.
        unsafe { *p.add(i) = (p as usize).wrapping_add(i) as u8 ^ bytes as u8 };
    }
}

/// Whether a block still holds its pattern.
fn intact(p: *mut u8, bytes: usize) -> bool {
    (0..bytes).all(|i| {
        // SAFETY: the block holds `bytes` bytes.
        unsafe { *p.add(i) == (p as usize).wrapping_add(i) as u8 ^ bytes as u8 }
    })
}

#[test]
fn sizes_land_on_lists_of_their_own_size() {
    for (bytes, list) in [(1, 1), (16, 1), (17, 2), (32, 2), (33, 3), (512, 32)] {
        assert_eq!(size_index(bytes), list);
    }
    assert!(serves(Layout::from_size_align(512, 16).unwrap()));
    assert!(!serves(Layout::from_size_align(513, 16).unwrap()));
    assert!(!serves(Layout::from_size_align(64, 32).unwrap()));
}

#[test]
fn blocks_are_aligned_apart_and_reused() {
    // SAFETY: each block goes back through `give` with its size.
    unsafe {
        let blocks: Vec<(*mut u8, usize)> = (1..=MOST)
            .step_by(7)
            .map(|bytes| (take(bytes), bytes))
            .collect();
        for (p, bytes) in &blocks {
            assert_eq!(*p as usize % STEP, 0);
            fill(*p, *bytes);
        }
        for (p, bytes) in &blocks {
            assert!(intact(*p, *bytes), "a block changed: {bytes} bytes");
        }
        for (p, bytes) in &blocks {
            give(*p, *bytes);
        }
        // The last block given back is the next one taken of its size.
        let (p, bytes) = *blocks.last().unwrap();
        assert_eq!(take(bytes), p);
        give(p, bytes);
    }
}

#[test]
fn blocks_freed_on_other_threads_stay_whole() {
    let (send, receive) = std::sync::mpsc::channel::<(usize, usize)>();
    let takers: Vec<_> = (0..4)
        .map(|_| {
            let send = send.clone();
            std::thread::spawn(move || {
                for round in 0..200_000usize {
                    let bytes = 1 + round * 31 % MOST;
                    // SAFETY: the receiver gives it back with this size.
                    let p = unsafe { take(bytes) };
                    fill(p, bytes);
                    send.send((p as usize, bytes)).unwrap();
                }
            })
        })
        .collect();
    drop(send);
    let giver = std::thread::spawn(move || {
        for (p, bytes) in receive {
            assert!(intact(p as *mut u8, bytes));
            // SAFETY: it came from `take(bytes)`, and only this thread has it.
            unsafe { give(p as *mut u8, bytes) };
        }
    });
    takers.into_iter().for_each(|t| t.join().unwrap());
    giver.join().unwrap();
}

#[test]
fn a_thread_that_ends_leaves_its_blocks_for_others() {
    let i = size_index(100);
    let before = LEFT[i].load(std::sync::atomic::Ordering::Relaxed);
    for _ in 0..3 {
        std::thread::spawn(|| {
            // SAFETY: given back at once, with this size.
            unsafe {
                let p = take(100);
                fill(p, 100);
                give(p, 100);
            }
        })
        .join()
        .unwrap();
    }
    // Not `shared().free[i]`: tests run at once, and another one may have
    // taken the blocks already.
    let after = LEFT[i].load(std::sync::atomic::Ordering::Relaxed);
    assert!(after >= before + 3, "each thread left its block for others");
    // What other threads left is usable, and what they didn't cut too.
    std::thread::spawn(move || {
        // SAFETY: given back with this size.
        unsafe {
            let blocks: Vec<*mut u8> = (0..10_000).map(|_| take(100)).collect();
            blocks.iter().for_each(|p| fill(*p, 100));
            assert!(blocks.iter().all(|p| intact(*p, 100)));
            blocks.iter().for_each(|p| give(*p, 100));
        }
    })
    .join()
    .unwrap();
}
