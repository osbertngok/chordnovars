pub extern crate pest;
pub extern crate pest_derive;

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/pitch.pest"]
pub struct PitchParser;
