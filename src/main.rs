extern crate itertools;
extern crate once_cell;
extern crate regex;
extern crate serde;
extern crate rayon;

mod constraint;
mod defn;
mod env;
mod misc;
mod multiverse;
mod solver;
//mod tsp_solver;

use std::env::args;
use std::error::Error;
use std::io;
use std::time::Instant;

fn main_stdin() -> Result<(), Box<dyn Error>> {
    let mut strdefn = String::new();
    let stdin = io::stdin();
    for _ in 0..38 {
        let mut line = String::new();
        stdin.read_line(&mut line)?;
        strdefn.push_str(&line);
    }
    let defn = defn::of_string(&strdefn)?;
    let mut env = env::Env::new(3600 * 24 * 30);

    let start_time = Instant::now(); // Startzeit erfassen
    let outcome = solver::solve(&mut env, &defn, true); // set verbose to false to disable debug println
    let elapsed_time = start_time.elapsed();

    println!("{}", outcome);
    println!("{:?}", outcome);
    println!("Solver Laufzeit: {:.3?} Sekunden", elapsed_time.as_secs_f64());
    Ok(())
}

/*
fn main_tsp() -> Result<(), Box<dyn Error>> {
    let mut strdefn = String::new();
    let stdin = io::stdin();
    for _ in 0..38 {
        let mut line = String::new();
        stdin.read_line(&mut line)?;
        strdefn.push_str(&line);
    }
    let defn = defn::of_string(&strdefn)?;
    let mut env = env::Env::new(10);
    let start_time = Instant::now(); // Startzeit erfassen
    let outcome = tsp_solver::run(&mut env, &defn, true);
    let elapsed_time = start_time.elapsed();

    println!("{}", outcome);
    println!("Solver Laufzeit: {:.3?} Sekunden", elapsed_time.as_secs_f64());
    Ok(())
}
 */
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = args().collect();
    if args.len() != 2 {
        Err("Wrong number of arguments to program".into())
    } else if args[1] == "-" {
        main_stdin()
    } else if args[1] == "tsp" {
        Err("There seems to be nothing here?!".into())
        //main_tsp()
    } else {
        Err("Wrong argument to program".into())
    }
}
