use chrono::DateTime;
use regex::{Captures, Regex};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use clap::{App, Arg, ArgGroup, value_t};


#[derive(Debug)]
struct MaxHole {
    time1: i64,
    line1: Option<String>,
    time2: i64,
    line2: Option<String>,
    lasttime: i64,
    lastline: Option<String>,
}

impl MaxHole {
    pub fn new() -> MaxHole {
        MaxHole {
            time1: 0,
            line1: None,
            time2: 0,
            line2: None,
            lasttime: 0,
            lastline: None,
        }
    }

    pub fn update(mut self, timems: i64, line: String) -> MaxHole {
        if self.line1 == None {
            self.time1 = timems;
            self.line1 = Some(line);
        } else if self.line2 == None {
	    self.time2 = timems;
	    self.line2 = Some(line);
        } else if self.lastline == None {
	    if timems - self.time2 > self.time2 - self.time1 {
		self.line1 = self.line2;
		self.time1 = self.time2;
		self.time2 = timems;
		self.line2 = Some(line);
	    } else {
		self.lastline = Some(line);
		self.lasttime = timems;
	    }
	} else {
            if timems - self.lasttime > self.time2 - self.time1 {
                self.time1 = self.lasttime;
                self.line1 = self.lastline;
		self.time2 = timems;
		self.line2 = Some(line);
		self.lastline = None;
            } else {
                self.lastline = Some(line);
                self.lasttime = timems;
            }
        }
	return self;
    }
}

#[derive(Debug)]
struct ThreshHole {
    lasttime: Option<i64>,
    lastline: Option<String>,
    threshold: i64
}

impl ThreshHole {
    pub fn new(threshold: i64) -> ThreshHole {
	ThreshHole {
	    lasttime: None,
	    lastline: None,
	    threshold
	}
    }
    pub fn update(mut self, time: i64, line: String) -> ThreshHole {
	match self.lasttime {
	    Some(ll) => {
		if ll - time > self.threshold {
		    println!("{}\n{}\n", line, self.lastline.unwrap());
		}
	    }
	    None => (),
	}
	self.lastline = Some(line);
	self.lasttime = Some(time);
	return self
    }
}

enum Hole {
    TH(ThreshHole),
    MH(MaxHole)
}



fn main_loop(path: &Path, threshold_ms: &Option<i64>) {
    let pattern_str = r"^\[(2021-.{0,30}Z)\]";
    let pattern = Regex::new(pattern_str).unwrap();
    let file = File::open(Path::new(path)).unwrap();
    let bf = BufReader::new(file);
    let mut algo = match threshold_ms {
	Some(timems) => Hole::TH(ThreshHole::new(timems.clone())),
	None => Hole::MH(MaxHole::new())
    };
    let mut skipped: u64 = 0;
    for line in bf.lines() {
	let line = match line {
	    Ok(good_string) => good_string,
	    _err => { skipped = skipped.saturating_add(1); continue; }
	};
	let dates: Option<Captures> = pattern.captures(&line);
        let date = match dates {
            None => { skipped = skipped.saturating_add(1); continue; }
            Some(capts) => capts.get(1).unwrap().as_str(),
        };
        let dt = DateTime::parse_from_rfc3339(date)
            .unwrap()
            .timestamp_millis();
	match algo {
	    Hole::TH(th) => { algo = Hole::TH(th.update(dt, line)) },
	    Hole::MH(mh) => { algo = Hole::MH(mh.update(dt, line)) }
	}
    }

    match algo {
	Hole::MH(mh) => if mh.line1 != None && mh.line2 != None {
	    println!("Biggest difference:\n{}\n{}\n", mh.line1.unwrap(), mh.line2.unwrap());
	} else {
	    println!("Parsed ts regex on less than 2 lines!")
	}
	_ => ()
    }
    println!("Skipped {} lines because of ill-formatted timestamp.", skipped)
}

fn main() {
    let matches = App::new("Hole finding program")
        .version("0.1")
        .author("Krzysztof Piecuch. <piecuch@kpiecuch.pl>")
        .about("Finds big holes in logfiles")
        .arg(Arg::with_name("input")
             .short("f")
             .long("file")
             .value_name("FILE")
             .help("Sets a logfile to read from.")
	     .takes_value(true)
             .default_value("-"))
        .group(ArgGroup::with_name("algorithm")
               .required(true)
               .args(&["maxhole", "threshold"]))
	.arg(Arg::with_name("maxhole")
	     .short("m").long("maxhole").takes_value(false))
        .arg(Arg::with_name("threshold")
             .short("t").long("threshold")
	     .takes_value(true).value_name("THRESHOLD")
	     .help("minimum hole length to report about (ms)."))
	.get_matches();

    let file = match matches.value_of("input").unwrap() {
	"-" => Path::new("/dev/stdin"),
	other => Path::new(other),
    };
    if matches.is_present("maxhole") {
	main_loop(file, &None);
    } else {
	let mut threshold_ms = 500;
	if matches.is_present("threshold") {
	    threshold_ms = value_t!(matches, "threshold", i64).unwrap_or_else(|e| e.exit());
	}
	main_loop(file, &Some(threshold_ms));
    }
}
