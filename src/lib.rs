use std::net::UdpSocket;
use std::net::SocketAddr;

/// Module containing the opcodes used by TFTP
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

/// The null byte (0u8)
static NULL: [u8; 1] = [0];
/// "NETASCII" in bytes for optimization
static NETASCII: [u8; 8] = [110, 101, 116, 97, 115, 99, 105, 105];

/// A enum containg the possible errors returned by put_file and get_file
#[derive(Debug)]
pub enum TftpError {
	/// Invalid response
	InvalidResponse(Vec<u8>),
	/// Undefined error (see string)
	NotDefined(String),
	/// File not found
	FileNotFound,
	/// Access violation
	AccessViolation,
	/// Disk storage is full
	DiskFull,
	/// Illegal operation requested
	IllegalOperation,
	/// Unknown transfer ID
	UnknownTransferID,
	/// File already exists
	FileAlreadyExists,
	/// User does not exist
	NoSuchUser
}

impl TftpError {
	/// Converts a slice into its respected error where the slice begins with the two error bytes
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

/// Writes a file into the TFTP server
///
/// ```rust
/// use std::net::UdpSocket;
///
/// use tftp_lib::put_file;
///
/// let sock = UdpSocket::bind("0.0.0.0:0").unwrap();
///
/// put_file("pathname.txt", "Testing".as_bytes(), &sock);
/// ```
pub fn put_file(path: &str, data: &[u8], sock: &UdpSocket) -> Result<(), TftpError> {
	// Better performance by ~40ns
	let payload = [&opcode::OPCODE_WRQ, path.as_bytes(), &NULL, &NETASCII, &NULL].concat();
	
	sock.send_to(&payload, "127.0.0.1:69").unwrap();

	// Enter the loop managing the retrival of data
	let mut sends_completed: u16 = 1;
	let mut final_recv = false;
	loop {
		// Opcode (2b) + data (512b)
		let mut response: [u8; 516] = [0u8; 516];
		let (bytes, socket_addr) = sock.recv_from(&mut response).unwrap();

		match response[0..2]  {
			// [0, 3] is OPCODE_DAT
			[0, 4] => {
				// Start with sending the file
				if final_recv == true {
					break;
				}

				if u16::from_be_bytes([response[2], response[3]]) == sends_completed {
					sends_completed += 1;
				}

				let mut end = ((sends_completed) * 512) as usize;
				if end > data.len() {
					end = data.len();
					final_recv = true;
				}

				sock.send_to(&[&opcode::OPCODE_DAT, &sends_completed.to_be_bytes(), &data[((sends_completed-1)*512) as usize..end]].concat(), socket_addr).unwrap();
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

	Ok(())
}

/// Writes a file into the TFTP server
///
/// ```rust
/// use std::net::UdpSocket;
///
/// # use tftp_lib::put_file;
/// use tftp_lib::get_file;
///
/// let sock = UdpSocket::bind("0.0.0.0:0").unwrap();
///
/// # put_file("pathname.txt", "Testing".as_bytes(), &sock);
/// println!("{}", String::from_utf8_lossy(&get_file("pathname.txt", &sock).unwrap()));
/// ```
pub fn get_file(path: &str, sock: &UdpSocket) -> Result<Vec<u8>, TftpError> {
	// Better performance by ~40ns
	let payload = [&opcode::OPCODE_RRQ, path.as_bytes(), &NULL, &NETASCII, &NULL].concat();
	
	sock.send_to(&payload, "127.0.0.1:69").unwrap();

	// Enter the loop managing the retrival of data
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
				if bytes < 516 {
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

	Ok(final_data)
}
