use bit_vec::BitVec;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

use std::default::Default;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[derive(StructOpt)]
#[structopt(about, author)]
struct Opts {
	#[structopt()]
	/// takes the first matching file
	pick: String,

	#[structopt(short, long)]
	/// Ignores the state file and parses the in file again
	reparse: bool,
}

struct Task {
	task_name: String,
	file_in: String,
	file_out: String,
	file_state: String,

	state: TaskState,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Library {
	signup_time: u32,
	books_per_day: u32,
	books: BitVec,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Take {
	library: u32,
	books: Vec<u32>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct TaskState {
	// Input
	libraries: Vec<Library>,
	book_scores: Vec<u16>,
	duration: u32,

	// Output
	takes: Vec<Take>,
}

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
	let opts: Opts = Opts::from_args();

	ctrlc::set_handler(move || {
		RUNNING.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");

	let files = find_files();
	let file = pick_file(&files[..], &opts.pick);
	let mut task = open_task(&file, &opts);
	run(&mut task);
	task.state.gen_out();
}

impl TaskState {
	fn parse_in(&mut self, data: &str) {
	}

	fn gen_out(&self) {
	}
}

impl Take {
	fn score(&self, state: &TaskState, start: u32) -> u32 {
		let lib = &state.libraries[self.library as usize];
		let start = start + lib.signup_time;
		self.books
			.iter()
			.take(state.duration as usize)
			.cloned()
			.map(|b| state.book_scores[b as usize] as u32)
			.sum()
	}
}

fn run(task: &mut Task) {
	//task.save_state();
	// CODE HERE
	while RUNNING.load(Ordering::SeqCst) {
		thread::sleep(Duration::from_secs(1));
	}
}

// FRAME ======================================================================

// File picker

const FILE_PATH: &'static str = "./data/";

fn find_files() -> Vec<String> {
	let paths = fs::read_dir(FILE_PATH).expect("No file found starting with your substring");
	let mut files = vec![];
	for path in paths {
		let p = path.unwrap().path();
		if let Some(file_stem) = p.file_stem().and_then(|s| s.to_str()) {
			if let Some(file_ext) = p.extension().and_then(|s| s.to_str()) {
				if file_ext == "in" {
					files.push(file_stem.to_string());
				}
			}
		} // else { println!("Skipping file {:?}", p); }
	}

	files
}

fn pick_file<'a, T: AsRef<str>>(files: &'a [T], starts_with: &str) -> &'a T {
	for f in files {
		if f.as_ref().starts_with(starts_with) {
			return f;
		}
	}
	panic!("Not found");
}

// Reader

fn open_task(name: &str, opts: &Opts) -> Task {
	let mut task = Task::new(name);

	let state_exists = Path::new(&task.file_state).exists();

	if !state_exists || opts.reparse {
		// Read from raw file
		let data = fs::read_to_string(&task.file_in).unwrap();
		task.state.parse_in(&data);
	} else {
		// Restore state
		task.load_state();
		println!("Restored state: {:?}", task.state);
	}

	task
}

// I/O

fn combine_name(base: &str, ext: &str) -> String {
	let mut f = String::new();
	f.push_str(FILE_PATH);
	f.push_str(base);
	f.push('.');
	f.push_str(ext);
	f
}

impl Task {
	fn new(base: &str) -> Task {
		Task {
			task_name: base.to_string(),
			file_in: combine_name(base, "in"),
			file_out: combine_name(base, "out"),
			file_state: combine_name(base, "state"),

			state: TaskState::default(),
		}
	}

	fn load_state(&mut self) {
		self.state = TaskState::load_state(&self.file_state);
	}

	fn save_state(&self) {
		self.state.save_state(&self.file_state);
	}
}

impl TaskState {
	fn load_state(file: &str) -> TaskState {
		let data = fs::read_to_string(file).unwrap();
		serde_json::from_str(&data).unwrap()
	}

	fn save_state(&self, file: &str) {
		let data = serde_json::to_string(&self).unwrap();
		fs::write(file, data.as_bytes()).unwrap();
	}
}
