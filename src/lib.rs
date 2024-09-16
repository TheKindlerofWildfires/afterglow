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

        Summary of the ack issue:
            I think recv needs to track seq no  its seen / ack no it's seen / ack square no it's seen
            Seq no-> tracks which packets contig
            ack no-> tracks which acks it's sent
            ack square no -> tracks which acks squares it's sent

            I think send needs to track seq no / ack no / ack square no
            seq no -> tracks which seq to send next
            ack no -> tracks which packets it's confirmed and can drop
            ack square no -> tracks which acks are confirmed and don't need to be resent

*/

//Known issues: Acks are offline
//Condvars instead of sleeps
//Too many unwraps, not enough grace
//Some kind of slowness
//high duplicate code rate