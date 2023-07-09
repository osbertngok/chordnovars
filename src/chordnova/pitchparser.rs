pub extern crate pest;
pub extern crate pest_derive;

use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "chordnova/pitch.pest"]
pub struct PitchParser;

use crate::chordnova::pitchparser::pest::Parser;
use pest::error::Error;
