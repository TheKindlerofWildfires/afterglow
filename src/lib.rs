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

/*
Known issues: 
    drop isn't enabled 
    handshake can loop around discovery 
    test for double server is flawed (timing dependant)
    dropping certain acks can cause spinning, probably when drop happens

Unknown issues:
    spin heat is unknown
    congestion isn't tested (mimic is hard)

*/

