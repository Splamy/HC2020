use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about, author)]
struct Opts {
	#[structopt()]
	/// takes the first matching file
	pick: String,
}

struct Task {
	task_name: String,
	state: TaskState,
}

#[derive(Serialize, Deserialize, Default)]
struct TaskState {
	// TODO
}

fn main() {
	let opts: Opts = Opts::from_args();

	let files = find_files();
	let file = pick_file(&files[..], opts.pick);

	// todo check skip parse
	let task = open_task(&file);
	// - import data
	// - run
	// - save => state
	// - save => output
}


fn parse_in(data: &str, state: &mut TaskState) {
	// CODE HERE
}

fn run(task: &mut Task) {
	// CODE HERE
}

fn gen_out(task: &mut Task) {
	// CODE HERE
}

// File picker

fn find_files() -> Vec<String> {
	let paths = fs::read_dir("./").unwrap();
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

fn pick_file<'a, T: AsRef<str>>(files: &'a [T], starts_with: String) -> &'a T {
	for f in files {
		if f.as_ref().starts_with(&starts_with) {
			return f;
		}
	}
	panic!("Not found");
}

// Reader

fn open_task(name: &str) -> Task {
	let mut task = Task {
		task_name: name.to_string(),
		state: TaskState::default(),
	};

	let in_file = task.get_filename_in();
	let state_file = task.get_filename_state();
	
	let state_exists = Path::new(&state_file).exists();

	if state_exists {
		// Read from raw file
		task.load_state();
	} else {
		// Read from raw file
		let data = read_as_string(&in_file);
		parse_in(&data, &mut (task.state));
	}

	task
}

// I/O

fn read_as_string(file: &str) -> String {
	let mut f = File::open(&file).unwrap();
	let mut buffer = String::new();
	f.read_to_string(&mut buffer).unwrap();
	buffer
}

fn write_from_string(file: &str, data: &str) {
	let mut f = File::open(&file).unwrap();
	f.write_all(data.as_bytes()).unwrap();
}

impl Task {
	fn get_filename_in(&self) -> String {
		self.combine_name("in")
	}
	fn get_filename_state(&self) -> String {
		self.combine_name("state")
	}
	fn get_filename_out(&self) -> String {
		self.combine_name("out")
	}

	fn combine_name(&self, ext: &str) -> String {
		let mut f = self.task_name.to_string();
		f.push('.');
		f.push_str(ext);
		f
	}

	fn load_state(&mut self) {
		let state_file = self.get_filename_state();
		let data = read_as_string(&state_file);
		self.state = serde_json::from_str(&data).unwrap();
	}

	fn save_state(&self) {
		let state_file = self.get_filename_state();
		let data = serde_json::to_string(&self.state).unwrap();
		write_from_string(&state_file, &data);
	}
}
