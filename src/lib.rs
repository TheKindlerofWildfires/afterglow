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
        test dropping both sides for the handshake until it works reliably
        disable ctrl dropping, test data dropping across wide range
        Add back in ctrl dropping and repeat

*/

//Known issues: Acks are partial
//Condvars not fully done
//loss isn't tested
//congestion isn't tested
//drop isn't enabled
//dropping discovers can cause looping if one side thinks it's connected (it got the discover) and the other thinks it's negotiating (dropped along the way)
//solution is to 1: force drops to be discover, 2: ack discover packets to change state (Will include a discover request/discover response type)
//Discovery loop when drop happens possible -> timeout 