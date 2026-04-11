use std::thread;
use std::time::{Duration, Instant};

const DURATION_SECS: u64 = 300;       // 5-minute burn
const TARGET_UTILIZATION: f64 = 0.30; // 30% duty cycle per core

fn cpu_burn(duration: Duration, utilization: f64) {
	let end = Instant::now() + duration;
	while Instant::now() < end {
		let busy_until = Instant::now() + Duration::from_secs_f64(utilization);
		// Active computation during the busy slice
		while Instant::now() < busy_until {
			// Volatile-ish busy work the compiler won't optimize away
			let _ = (2u64.pow(32) as f64).sqrt();
		}
		// Sleep for the idle remainder of the 1-second cycle
		let idle = Duration::from_secs_f64(1.0 - utilization);
		thread::sleep(idle);
	}
}

fn main() {
	let core_count = thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
	
	let duration = Duration::from_secs(DURATION_SECS);
	
	println!(
		"Starting CPU burn on {} core(s) for {}s ...",
		core_count, DURATION_SECS
	);
	
	let handles: Vec<_> = (0..core_count)
		.map(|_| thread::spawn(move || cpu_burn(duration, TARGET_UTILIZATION)))
		.collect();
	
	for handle in handles {
		handle.join().expect("Worker thread panicked");
	}
	
	println!("Done.");
}