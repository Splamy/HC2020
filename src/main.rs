use bit_vec::BitVec;
use failure::Error;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

use std::default::Default;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

type Result<T> = std::result::Result<T, Error>;

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
	task.save_output();
}

impl TaskState {
	fn parse_in(&mut self, data: &str) {
		let mut lines = data.lines();
		let l = lines.next().unwrap(); // (amount books, amount libs, days scanning)
		let mut s = l.split(' ').map(|n| n.parse::<u32>().unwrap());
		let amount_books = s.next().unwrap() as usize;
		let amount_libs = s.next().unwrap() as usize;
		self.duration = s.next().unwrap();

		let l = lines.next().unwrap();
		self.book_scores.extend(l.split(' ').map(|n| n.parse::<u16>().unwrap()));
		assert_eq!(self.book_scores.len(), amount_books);
		for i in 0..amount_libs {
			let l = lines.next().unwrap(); // (amt books, signup time, books/day)
			let mut s = l.split(' ').map(|n| n.parse::<u32>().unwrap());
			let amount_books_lib = s.next().unwrap() as usize;
			let signup_time = s.next().unwrap();
			let books_per_day = s.next().unwrap();

			let mut lib = Library {
				signup_time,
				books_per_day,
				books: BitVec::from_elem(amount_books, false)
			};

			// Read books
			let l = lines.next().unwrap();
			for b in l.split(' ').map(|n| n.parse::<usize>().unwrap()) {
				lib.books.set(b, true);
			}
			assert_eq!(lib.books.iter().filter(|x| *x).count(), amount_books_lib);

			self.libraries.push(lib);
		}
	}

	fn gen_out(&self, w: &mut Write) -> Result<()> {
		println!("Saving output");
		writeln!(w, "{}", self.takes.len())?;
		for t in &self.takes {
			writeln!(w, "{} {}", t.library, t.books.len())?;
			for b in &t.books {
				write!(w, "{}", b)?;
			}
			writeln!(w)?;
		}
		println!("Score: {}", self.score());
		Ok(())
	}

	fn score(&self) -> u32 {
		let mut score = 0;
		let mut start = 0;
		for t in &self.takes {
			score += t.score(self, start);
			start += self.libraries[t.library as usize].signup_time;
		}
		score
	}

	/// Advances the state by one step.
	///
	/// Returns `true` if done or `false` if more steps should be done.
	fn do_step(&mut self) -> bool {
		let cur_time: u32 = self.takes.iter()
			.map(|t| self.libraries[t.library as usize].signup_time)
			.sum();
		println!("Time {}/{}", cur_time, self.duration);
		if cur_time >= self.duration {
			return true;
		}

		// Search for the best library

		false
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
	while RUNNING.load(Ordering::SeqCst) && !task.state.do_step() {
	}
	task.save_state();
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

	fn save_output(&self) {
		let mut f = File::create(&self.file_out).unwrap();
		self.state.gen_out(&mut f).unwrap();
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
