pub mod congestion;
pub mod core;
pub mod huffman;
pub mod packet;
pub mod serial;
pub mod sha;
pub mod utils;
pub mod window;
pub mod tests;
pub mod listener;
pub mod stream;
pub mod connection;
/*
    Plan:
        fix keep alive spiral
        regression
        Do full loss

*/

//Known issues: 
//spin heat is unknown
//congestion isn't tested
//drop isn't enabled
//Discovery loop when drop happens possible -> timeout 
//keep alive spiral when lots of loss in multi packet (is this drop related?, did we timeout)
//not fully tested with mass loss 

