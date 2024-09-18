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
    Plan is to create all 'send' side functions with stubs
    Figure out where they are called, fill those out
    Fill out tht send stubs
    Error sweep
    Create all process side functions with stubs (mostly done)
    Fill out process stubs
    Error sweep
    Figure out missing parts
    Reeval

    Start tests

    Tests:
        client/server send/recv data
            correct ack/second ack
        A smaller mss gets negotiated because a fragment happens(on both ends) (Use a drop in 'Send fragmenty' socket)
        A lost packet is recovered (Use a drop in 'send lossy' socket)
        Graceful shutdown on both ends
        Error state via dropped connection
        Congestion via the drop in 'slow socket / slowing socket'
        fake a drop message (case is not super clear -> maybe block loss packets)
        duplex a channel on two different send/recv ports

        Plan: 
            Clippy up a little
            Engage the ack system / loss dissect
            Create features for 'Fragment socket' 
            Create features for 'Lossy socket'

        Ack issue v2:
        Acks seems to be 'working' or close to it, but the test cases aren't finished

*/

//Known issues: Acks are partial
//Condvars not fully done
//A few unwraps left
//loss isn't tested
//congestion isn't tested
//drop isn't enabled
