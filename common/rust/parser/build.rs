use std::fs::{File, create_dir_all, canonicalize};
use std::io::prelude::*;
use std::io::BufReader;


fn prepend(input: File, mut output: File, text: &str) -> std::io::Result<()> {
    let buffered = BufReader::new(input);
    writeln!(output, "{}", text)?;
    for line in buffered.lines() {
        writeln!(output, "{}", line?)?;
    }
    Ok(())
}

/* fixes a scalajs bug https://github.com/scala-js/scala-js/issues/3677/ */
fn scalajs_fix() -> std::io::Result<()> {
    let root = canonicalize("../../..").expect("Couldn't get root of workspace.");
    let parser_path = &root.join("target").join("scala-parser.js");
    let pkg_path = &root.join("common").join("rust").join("parser").join("pkg");
    let parser_path_fix = &pkg_path.join("scala-parser.js");
    
    let original = File::open(&parser_path)
          .expect(&format!("{} {}", "Could not find file ", parser_path.to_str().unwrap()));
    create_dir_all(&pkg_path)
          .expect(&format!("{} {}", "Could not create file ", pkg_path.to_str().unwrap()));
    let fixed = File::create(&parser_path_fix)
          .expect(&format!("{} {}", "Could not create file ", parser_path_fix.to_str().unwrap()));
    
    prepend(original, fixed, "var __ScalaJSEnv = { global: window };")
}

fn main() -> std::io::Result<()>  {
    scalajs_fix()?;
    Ok(())
}