use std::thread;

fn main() {
    // Get the number of CPU cores available
    let num_cpus = num_cpus::get();
    let num_threads = num_cpus * 4;

    let mut handles = vec![];

    // Spawn the threads
    for _ in 0..num_threads {
        let handle = thread::spawn(move || {
            loop {
                // Perform some meaningful calculation
                let _sum_of_squares: u64 = sum_of_square(10000);
                // loop until Ctrl-C is pressed
                
            }
        });
        handles.push(handle);
    }

    // block until all threads have finished
    for handle in handles {
        handle.join().unwrap();
    }
}

#[inline(never)]
fn sum_of_square(n: u64) -> u64 {
    (1..=n).map(|x| x * x).sum()
}
