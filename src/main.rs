use std::fs::{self, File};
//use std::fs::File;
use structopt::StructOpt;
use serde::{Deserialize, Serialize};
use std::io::Read;

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

#[derive(Serialize, Deserialize)]
struct TaskState {
	// TODO
}

fn main() {
	let opts: Opts = Opts::from_args();

	let files = find_files();
	let file = pick_file(&files[..], opts.pick);

	// todo check skip parse
	let task = create_task(&file);
	// - import data
	// - run
	// - save => state
	// - save => output
}

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

fn create_task(name: &str) -> Task {
	let mut f = File::open("foo.txt").unwrap();
	let mut buffer = String::new();
	f.read_to_string(&mut buffer).unwrap();

	Task {
		task_name: name.to_string(),
		state: parse(buffer)
	}
}

fn parse(data: String) -> TaskState {
	// TODO
	TaskState {
		// empty ?
	}
}

fn run(task: &mut Task) {

}

fn save(task: &mut Task) {

}

impl Task {
	fn get_filename_in(&self) -> String { self.combine_name("in") }

	fn combine_name(&self, ext: &str) -> String {
		let mut f = self.task_name.to_string();
		f.push('.');
		f.push_str(ext);
		f
	}
}
