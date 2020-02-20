use serde::{Deserialize, Serialize};
use std::default::Default;
use structopt::StructOpt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
struct TaskState {
	// CODE HERE
}

static running: std::sync::atomic::AtomicBool = AtomicBool::new(true);

fn main() {
	let opts: Opts = Opts::from_args();

    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

	let files = find_files();
	let file = pick_file(&files[..], &opts.pick);
	let mut task = open_task(&file, &opts);
	run(&mut task);
	gen_out(&mut task.state);
}


fn parse_in(data: &str, state: &mut TaskState) {
	// CODE HERE
}

fn run(task: &mut Task) {
	//task.save_state();
	// CODE HERE
	while running.load(Ordering::SeqCst) {
		thread::sleep(Duration::from_secs(1));
	}
}

fn gen_out(state: &mut TaskState) {
	// CODE HERE
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
		let data = read_as_string(&task.file_in);
		parse_in(&data, &mut task.state);
	} else {
		// Restore state
		task.load_state();
		println!("Restored state: {:?}", task.state);
	}

	task
}

// I/O

fn read_as_string(file: &str) -> String {
	println!("Reading {}", file);
	let mut f = File::open(&file).unwrap();
	let mut buffer = String::new();
	f.read_to_string(&mut buffer).unwrap();
	buffer
}

fn write_from_string(file: &str, data: &str) {
	println!("Writing {}", file);
	let mut f = File::create(&file).unwrap();
	f.write_all(data.as_bytes()).unwrap();
}

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
		let data = read_as_string(file);
		serde_json::from_str(&data).unwrap()
	}

	fn save_state(&self, file: &str) {
		let data = serde_json::to_string(&self).unwrap();
		write_from_string(file, &data);
	}
}
