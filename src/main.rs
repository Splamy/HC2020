use std::cmp::{Ord, Ordering, PartialOrd};
use std::collections::BinaryHeap;
use std::default::Default;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{self, AtomicBool};

use bit_vec::BitVec;
use failure::Error;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use rayon::prelude::*;

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

	#[structopt(short, long)]
	/// Takes the best recursive matching library
	x_algo: bool,
}

struct Task {
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

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
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

	// State
	cur_time: u32,
	/// The library ids which are left.
	cur_libraries: BitVec,
	/// The books which are left.
	cur_books: BitVec,

	// Output
	takes: Vec<Take>,
}

struct BookScore<'a> {
	state: &'a TaskState,
	book: u32,
}

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main() {
	let opts: Opts = Opts::from_args();

	ctrlc::set_handler(move || {
		RUNNING.store(false, atomic::Ordering::SeqCst);
	})
	.expect("Error setting Ctrl-C handler");

	let files = find_files();
	let file = pick_file(&files[..], &opts.pick);
	let mut task = open_task(&file, &opts);
	run(&mut task, &opts);
	task.save_output();
}

impl PartialEq for BookScore<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.book == other.book
	}
}
impl Eq for BookScore<'_> {}

impl Ord for BookScore<'_> {
	fn cmp(&self, other: &Self) -> Ordering {
		self.state.book_scores[self.book as usize]
			.cmp(&self.state.book_scores[other.book as usize])
			.reverse()
	}
}

impl PartialOrd for BookScore<'_> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl TaskState {
	fn take_free_time(&self, take: &Take) -> u32 {
		let lib = &self.libraries[take.library as usize];
		self.remaining_time()
			.saturating_sub(lib.signup_time)
	}

	fn take_rem_time(&self, take: &Take) -> u32 {
		let lib = &self.libraries[take.library as usize];
		self.remaining_time()
			.saturating_sub(lib.signup_time)
			.saturating_sub(take.books.len() as u32 / lib.books_per_day)
	}

	fn parse_in(&mut self, data: &str) {
		let mut lines = data.lines();
		let l = lines.next().unwrap(); // (amount books, amount libs, days scanning)
		let mut s = l.split(' ').map(|n| n.parse::<u32>().unwrap());
		let amount_books = s.next().unwrap() as usize;
		let amount_libs = s.next().unwrap() as usize;
		self.duration = s.next().unwrap();

		let l = lines.next().unwrap();
		self.book_scores
			.extend(l.split(' ').map(|n| n.parse::<u16>().unwrap()));
		assert_eq!(self.book_scores.len(), amount_books);
		for _ in 0..amount_libs {
			let l = lines.next().unwrap(); // (amt books, signup time, books/day)
			let mut s = l.split(' ').map(|n| n.parse::<u32>().unwrap());
			let amount_books_lib = s.next().unwrap() as usize;
			let signup_time = s.next().unwrap();
			let books_per_day = s.next().unwrap();

			let mut lib = Library {
				signup_time,
				books_per_day,
				books: BitVec::from_elem(amount_books, false),
			};

			// Read books
			let l = lines.next().unwrap();
			for b in l.split(' ').map(|n| n.parse::<usize>().unwrap()) {
				lib.books.set(b, true);
			}
			assert_eq!(
				lib.books.iter().filter(|x| *x).count(),
				amount_books_lib
			);

			self.libraries.push(lib);
		}

		self.cur_time = 0;
		self.cur_libraries = BitVec::from_elem(amount_libs, true);
		self.cur_books = BitVec::from_elem(amount_books, true);
	}

	fn gen_out(&self, w: &mut dyn Write) -> Result<()> {
		println!("Saving output");
		writeln!(w, "{}", self.takes.len())?;
		for t in &self.takes {
			writeln!(w, "{} {}", t.library, t.books.len())?;
			let mut first = true;
			for b in &t.books {
				if first {
					first = false
				} else {
					write!(w, " ")?;
				}
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
	fn do_step(&mut self, opts: &Opts) -> bool {
		println!("Time {}/{}", self.cur_time, self.duration);
		if self.cur_time >= self.duration {
			return true;
		}

		// Search for the best library
		/*let take_vec = self
			.cur_libraries
			.iter()
			.enumerate()
			.filter_map(|(i, l)| if l { Some(i as u32) } else { None })
			.collect::<Vec<u32>>();*/

		/*let take = take_vec
			.par_iter()
			.map(|lib| self.step_compute_library_score(*lib))
			.max_by(|t1, t2| t1.1.partial_cmp(&t2.1).unwrap());*/

		let take = if opts.x_algo {
			let mut take_l = (0..self.cur_libraries.len() as u32).into_par_iter()
			.filter(|i| self.cur_libraries.get(*i as usize).unwrap_or_default())
			.map(|lib| self.step_compute_library_score(lib))
			.collect::<Vec::<_>>();

			take_l.sort_by(|t1, t2| t1.1.partial_cmp(&t2.1).unwrap());
			if take_l.len() >= 2 {
				let t1 = &take_l[take_l.len() - 1];
				let t2 = &take_l[take_l.len() - 2];

				if self.take_free_time(&t2.0) < self.take_rem_time(&t1.0) {
					Some(t2.clone())
				} else {
					Some(t1.clone())
				}
			} else  {
				take_l.last().cloned()
			}

			//.max_by(|t1, t2| t1.1.partial_cmp(&t2.1).unwrap())
		} else {
			(0..self.cur_libraries.len() as u32).into_par_iter()
			.filter(|i| self.cur_libraries.get(*i as usize).unwrap_or_default())
			.map(|lib| self.step_compute_library_score(lib))
			.max_by(|t1, t2| t1.1.partial_cmp(&t2.1).unwrap())
		};

		if let Some((take, _score)) = take {
			self.cur_libraries.set(take.library as usize, false);
			if take.books.is_empty() {
				println!("No more libraries found");
				return true;
			}

			self.cur_time += self.libraries[take.library as usize].signup_time;

			for book in &take.books {
				self.cur_books.set(*book as usize, false);
			}

			self.takes.push(take);
			false
		} else {
			true
		}
	}

	fn remaining_time(&self) -> u32 { self.duration - self.cur_time }

	fn step_compute_library_score(&self, library: u32) -> (Take, f32) {
		let lib = &self.libraries[library as usize];
		if let Some(left_time) =
			self.remaining_time().checked_sub(lib.signup_time)
		{
			let book_count = left_time * lib.books_per_day;
			if book_count == 0 {
				return (Take { library, books: Vec::new() }, 0f32);
			}

			let mut heap = BinaryHeap::with_capacity(book_count as usize);

			lib.books
				.iter()
				.enumerate()
				.filter_map(|(i, x)| {
					if x && self.cur_books.get(i).unwrap_or_default() {
						Some(BookScore { state: self, book: i as u32 })
					} else {
						None
					}
				})
				.for_each(|book| {
					if heap.len() >= book_count as usize {
						if *heap.peek().unwrap() > book {
							heap.pop();
							heap.push(book);
						}
					} else {
						heap.push(book);
					}
				});
				//.collect::<Vec<_>>();
			/*books.sort_by(|a, b| {
				self.book_scores[*a as usize]
					.cmp(&self.book_scores[*b as usize])
					.reverse()
			});
			books.truncate(book_count as usize);*/

			let books = heap.iter().map(|b| b.book).collect();

			let take = Take { library, books };
			let score = take.score(self, self.cur_time) as f32 / lib.signup_time as f32;
			(take, score)
		} else {
			(Take { library, books: Vec::new() }, 0f32)
		}
	}
}

impl Take {
	fn score(&self, state: &TaskState, start: u32) -> u32 {
		let lib = &state.libraries[self.library as usize];
		let len = state.duration.saturating_sub(start + lib.signup_time);
		self.books
			.iter()
			.take((len * lib.books_per_day) as usize)
			.cloned()
			.map(|b| state.book_scores[b as usize] as u32)
			.sum()
	}
}

fn run(task: &mut Task, opts: &Opts) {
	while RUNNING.load(atomic::Ordering::SeqCst) && !task.state.do_step(opts) {}
	task.save_state();
}

// FRAME ======================================================================

// File picker

const FILE_PATH: &'static str = "./data/";

fn find_files() -> Vec<String> {
	let paths = fs::read_dir(FILE_PATH)
		.expect("No file found starting with your substring");
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
		//println!("Restored state: {:?}", task.state);
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
			file_in: combine_name(base, "in"),
			file_out: combine_name(base, "out"),
			file_state: combine_name(base, "state"),

			state: TaskState::default(),
		}
	}

	fn load_state(&mut self) {
		self.state = TaskState::load_state(&self.file_state);
	}

	fn save_state(&self) { self.state.save_state(&self.file_state); }

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
