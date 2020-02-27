#![feature(test)]

use std::net::UdpSocket;
use std::net::SocketAddr;

pub mod opcode {
	/// Read request
	pub static OPCODE_RRQ: [u8; 2] = [0, 1];
	/// Write request
	pub static OPCODE_WRQ: [u8; 2] = [0, 2];
	/// Data packet
	pub static OPCODE_DAT: [u8; 2] = [0, 3];
	/// Acknowledgment
	pub static OPCODE_ACK: [u8; 2] = [0, 4];
	/// Error
	pub static OPCODE_ERR: [u8; 2] = [0, 5];
}

static NULL: [u8; 1] = [0];
static NETASCII: [u8; 8] = [110, 101, 116, 97, 115, 99, 105, 105];

#[derive(Debug)]
pub enum TftpError {
	InvalidResponse(Vec<u8>),
	NotDefined(String),
	FileNotFound,
	AccessViolation,
	DiskFull,
	IllegalOperation,
	UnknownTransferID,
	FileAlreadyExists,
	NoSuchUser
}

impl TftpError {
	pub fn from_error_code(response: &[u8]) -> TftpError {
		// Match the error code byte
		match response {
			[0, 1, ..] => TftpError::FileNotFound,
			[0, 2, ..] => TftpError::AccessViolation,
			[0, 3, ..] => TftpError::DiskFull,
			[0, 4, ..] => TftpError::IllegalOperation,
			[0, 5, ..] => TftpError::UnknownTransferID,
			[0, 6, ..] => TftpError::FileAlreadyExists,
			[0, 7, ..] => TftpError::NoSuchUser,
			_ => TftpError::NotDefined(String::from_utf8_lossy(&response[2..]).to_string()),
		}
	}
}

fn send_ack(sock: &UdpSocket, block_num: &[u8], socket_addr: SocketAddr) {
	let payload = [&opcode::OPCODE_ACK, block_num].concat();
	
	sock.send_to(&payload, socket_addr).unwrap();
}

fn read_loop(sock: &UdpSocket) -> Result<(), TftpError> {
	let mut final_data = vec![];
	loop {
		// Opcode (2b) + data (512b)
		let mut response: [u8; 516] = [0u8; 516];
		let (bytes, socket_addr) = sock.recv_from(&mut response).unwrap();
		
		//println!("{}", String::from_utf8_lossy(&response[4..]));

		match response[0..2]  {
			// [0, 3] is OPCODE_DAT
			[0, 3] => {
				// Start with reading the file
				send_ack(sock, &response[2..4], socket_addr);

				final_data.extend(&response[4..]);
				if bytes < 2 + 2 + 512 {
					break;
				}
			},
			// [0, 5] is OPCODE_ERR
			[0, 5] => {
				// Parse the error
				return Err(TftpError::from_error_code(&response[2..bytes]))
			},
			_ => {
				return Err(TftpError::InvalidResponse(response.to_vec()))
			}
		}
	};

	println!("{}", String::from_utf8_lossy(&final_data));
	Ok(())
	
}

pub fn get_file(path: &str, sock: &UdpSocket) -> Result<(), TftpError> {
	// Better performance by ~40ns
	let payload = [&opcode::OPCODE_RRQ, path.as_bytes(), &NULL, &NETASCII, &NULL].concat();
	
	sock.send_to(&payload, "127.0.0.1:69").unwrap();

	// Enter the loop managing the retrival of data
	read_loop(sock)
}

extern crate test;

#[bench]
fn ttest(b: &mut test::Bencher) {
	let sock = UdpSocket::bind("127.0.0.1:5555").unwrap();
	b.iter(|| {
		test::black_box(get_file("cool.txt", &sock).unwrap());
	});
}
