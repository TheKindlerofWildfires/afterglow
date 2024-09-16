
//#[cfg(test)]
pub mod basic {
    use crate::core::channel::MAX_PACKET_SIZE;
    use crate::listener::NeonListener;
    use crate::stream::NeonStream;
    use std::{io::ErrorKind, net::SocketAddr, thread, time::Duration};

    #[test]
    fn server_spawn_single() {
        let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let server = NeonListener::simplex(addr);
        assert!(server.is_ok());
    }

    #[test]
    fn server_client_single() {
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 1, Duration::from_millis(1000), target);
        match client {
            Ok(_) => panic!("No server should have replied"),
            Err(err) => {
                assert!(err.kind() == ErrorKind::NotConnected)
            }
        }
    }
    #[test]
    fn handshake_single_test() {
        handshake_single()
    }
    //Start a server and exit when the first connection spawns
    pub fn handshake_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept();
            thread::sleep(Duration::from_micros(1000));
            assert!(stream.is_ok());
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target);
        thread::sleep(Duration::from_micros(1000));
        assert!(client.is_ok());
        assert!(handle.join().is_ok())
    }

    //Start a server and send a small data packet
    pub fn small_data_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept().unwrap();
            thread::sleep(Duration::from_millis(1000));
            let data = stream.read();
            data.iter()
                .zip([1, 2, 3, 4, 5, 6, 7, 8].iter())
                .for_each(|(a, b)| {
                    assert!(a == b);
                });
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
        client.write(&[1, 2, 3, 4, 5, 6, 7, 8], Duration::from_millis(1000), true);
        thread::sleep(Duration::from_millis(1000));
        assert!(handle.join().is_ok())
    }

    pub fn medium_data_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept().unwrap();
            thread::sleep(Duration::from_millis(1000));
            let data = stream.read();
            let check_data = (0..MAX_PACKET_SIZE)
                .flat_map(|i| (i % 32).to_le_bytes())
                .collect::<Vec<_>>();
            data.iter().zip(check_data.iter()).for_each(|(a, b)| {
                assert!(a == b);
            });
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let data = (0..MAX_PACKET_SIZE)
            .flat_map(|i| (i % 32).to_le_bytes())
            .collect::<Vec<_>>();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
        client.write(&data, Duration::from_millis(1000), true);
        thread::sleep(Duration::from_millis(1000));
        assert!(handle.join().is_ok())
    }
    pub fn large_data_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept().unwrap();
            thread::sleep(Duration::from_millis(1000));
            let data = stream.read();
            let check_data = (0..MAX_PACKET_SIZE * 2)
                .flat_map(|i| (i % 128).to_le_bytes())
                .collect::<Vec<_>>();
            data.iter().zip(check_data.iter()).for_each(|(a, b)| {
                assert!(a == b);
            });
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let data = (0..MAX_PACKET_SIZE * 2)
            .flat_map(|i| (i % 128).to_le_bytes())
            .collect::<Vec<_>>();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
        client.write(&data, Duration::from_millis(1000), true);
        thread::sleep(Duration::from_millis(1000));
        assert!(handle.join().is_ok())
    }

    pub fn small_reply_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept().unwrap();
            let data = stream.read();
            data.iter()
                .zip([1, 2, 3, 4, 5, 6, 7, 8].iter())
                .for_each(|(a, b)| {
                    assert!(a == b);
                });
            stream.write(
                &[1, 1, 2, 3, 5, 8, 13, 21],
                Duration::from_millis(1000),
                true,
            )
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
        client.write(&[1, 2, 3, 4, 5, 6, 7, 8], Duration::from_millis(1000), true);
        let reply = client.read();
        reply
            .iter()
            .zip([1, 1, 2, 3, 5, 8, 13, 21].iter())
            .for_each(|(a, b)| {
                assert!(a == b);
            });
        assert!(handle.join().is_ok())
    }

    pub fn medium_reply_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept().unwrap();
            let data = stream.read();
            let check_data = (0..MAX_PACKET_SIZE)
                .flat_map(|i| (i % 32).to_le_bytes())
                .collect::<Vec<_>>();
            data.iter().zip(check_data.iter()).for_each(|(a, b)| {
                assert!(a == b);
            });
            stream.write(
                &(0..MAX_PACKET_SIZE)
                    .rev()
                    .flat_map(|i| (i % 32).to_le_bytes())
                    .collect::<Vec<_>>(),
                Duration::from_millis(1000),
                true,
            )
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
        client.write(
            &(0..MAX_PACKET_SIZE)
                .flat_map(|i| (i % 32).to_le_bytes())
                .collect::<Vec<_>>(),
            Duration::from_millis(1000),
            true,
        );
        let reply = client.read();
        reply
            .iter()
            .zip(
                (0..MAX_PACKET_SIZE)
                    .rev()
                    .flat_map(|i| (i % 32).to_le_bytes())
                    .collect::<Vec<_>>()
                    .iter(),
            )
            .for_each(|(a, b)| {
                assert!(a == b);
            });
        assert!(handle.join().is_ok())
    }

    pub fn large_reply_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept().unwrap();
            let data = stream.read();
            let check_data = (0..MAX_PACKET_SIZE * 5)
                .flat_map(|i| (i % 128).to_le_bytes())
                .collect::<Vec<_>>();
            data.iter().zip(check_data.iter()).for_each(|(a, b)| {
                assert!(a == b);
            });
            stream.write(
                &(0..MAX_PACKET_SIZE * 5)
                    .rev()
                    .flat_map(|i| (i % 128).to_le_bytes())
                    .collect::<Vec<_>>(),
                Duration::from_millis(1000),
                true,
            )
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
        client.write(
            &(0..MAX_PACKET_SIZE * 5)
                .flat_map(|i| (i % 128).to_le_bytes())
                .collect::<Vec<_>>(),
            Duration::from_millis(1000),
            true,
        );
        let reply = client.read();
        reply
            .iter()
            .zip(
                (0..MAX_PACKET_SIZE * 5)
                    .rev()
                    .flat_map(|i| (i % 128).to_le_bytes())
                    .collect::<Vec<_>>()
                    .iter(),
            )
            .for_each(|(a, b)| {
                assert!(a == b);
            });
        assert!(handle.join().is_ok())
    }

    pub fn server_double() {
        //start a server in a new thread
        let handle_server = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream_one = server.accept();
            let stream_two = server.accept();
            assert!(stream_one.is_ok());
            assert!(stream_two.is_ok());
            thread::sleep(Duration::from_millis(100));
        });
        let handle_client_one = thread::spawn(|| {
            let bind = "127.0.0.1:8000".parse::<SocketAddr>().unwrap();
            let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target);
            assert!(client.is_ok());

            thread::sleep(Duration::from_millis(100));
        });
        let handle_client_two= thread::spawn(|| {
            let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
            let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target);
            assert!(client.is_ok());
            thread::sleep(Duration::from_millis(100));
        });
        //connect to it with a client
        assert!(handle_server.join().is_ok());
        assert!(handle_client_one.join().is_ok());
        assert!(handle_client_two.join().is_ok());
    }
    pub fn server_double_send() {
        //start a server in a new thread
        let handle_server = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream_one = server.accept().unwrap();
            let stream_two = server.accept().unwrap();

            let data_one = stream_one.read();
            data_one
                .iter()
                .zip([1, 2, 3, 4, 5, 6, 7, 8].iter())
                .for_each(|(a, b)| {
                    assert!(a == b);
                });
            let data_two = stream_two.read();
            data_two
                .iter()
                .zip([1, 1, 2, 3, 5, 8, 13, 21].iter())
                .for_each(|(a, b)| {
                    assert!(a == b);
                });
            thread::sleep(Duration::from_millis(100));
        });
        let handle_client_one = thread::spawn(|| {
            let bind = "127.0.0.1:8000".parse::<SocketAddr>().unwrap();
            let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
            client.write(&[1, 2, 3, 4, 5, 6, 7, 8], Duration::from_millis(1000), true);
            thread::sleep(Duration::from_millis(100));
        });
        thread::sleep(Duration::from_millis(1000));

        let handle_client_two= thread::spawn(|| {
            let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
            let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
            client.write(&[1, 1, 2, 3, 5, 8, 13, 21], Duration::from_millis(1000), true);
            thread::sleep(Duration::from_millis(100));
        });
        //connect to it with a client
        assert!(handle_server.join().is_ok());
        assert!(handle_client_one.join().is_ok());
        assert!(handle_client_two.join().is_ok());
    }

    pub fn server_double_reply() {
        //start a server in a new thread
        let handle_server = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream_one = server.accept().unwrap();
            let stream_two = server.accept().unwrap();

            let data_one = stream_one.read();
            data_one
                .iter()
                .zip([1, 2, 3, 4, 5, 6, 7, 8].iter())
                .for_each(|(a, b)| {
                    assert!(a == b);
                });
            let data_two = stream_two.read();
            data_two
                .iter()
                .zip([1, 1, 2, 3, 5, 8, 13, 21].iter())
                .for_each(|(a, b)| {
                    assert!(a == b);
                });
            stream_one.write(
                &[0,2,4,8,16,32,64,128],
                Duration::from_millis(1000),
                true,
            );
            stream_two.write(
                &[1,3,5,9,17,33,64,129],
                Duration::from_millis(1000),
                true,
            )
        });
        let handle_client_one = thread::spawn(|| {
            let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
            let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
            client.write(&[1, 2, 3, 4, 5, 6, 7, 8], Duration::from_millis(1000), true);
            let reply = client.read();
            reply
            .iter()
            .zip([0,2,4,8,16,32,64,128].iter())
            .for_each(|(a, b)| {
                assert!(a == b);
            });

        });
        thread::sleep(Duration::from_millis(1000));
        let handle_client_two= thread::spawn(|| {
            let bind = "127.0.0.1:8193".parse::<SocketAddr>().unwrap();
            let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target).unwrap();
            client.write(&[1, 1, 2, 3, 5, 8, 13, 21], Duration::from_millis(1000), true);
            let reply = client.read();
            reply
            .iter()
            .zip([1,3,5,9,17,33,64,129].iter())
            .for_each(|(a, b)| {
                assert!(a == b);
            });

        });
        //connect to it with a client
        assert!(handle_server.join().is_ok());
        assert!(handle_client_one.join().is_ok());
        assert!(handle_client_two.join().is_ok());
    }

    pub fn keep_alive_single() {
        //start a server in a new thread
        let handle = thread::spawn(|| {
            let addr = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
            let mut server = NeonListener::simplex(addr).unwrap();
            //drop when a stream get's popped
            let stream = server.accept();
            //idle here for a while
            thread::sleep(Duration::from_secs(10));
            assert!(stream.is_ok());
        });
        //connect to it with a client
        let bind = "127.0.0.1:9000".parse::<SocketAddr>().unwrap();
        let target = "127.0.0.1:8128".parse::<SocketAddr>().unwrap();
        let client = NeonStream::simplex(bind, 3, Duration::from_millis(1000), target);
        //idle here for a while
        thread::sleep(Duration::from_secs(10));
        assert!(handle.join().is_ok())
    }
}