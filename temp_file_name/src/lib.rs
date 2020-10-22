//! This is a tiny crate with one purpose:
//! generate temporary filenames
//! from a hash of arbitrary data
//! so that they are consistent across different crates.
//! Its intended purpose is to either
//! - give a consistent name, which can -- for example -- be used to cache the expensive output of an input across runs,
//! - or to allow a caller to know what filename a different crate will have given some abstract data.
//!
//! It would be trivially easy to replicate; it exists primarily to allow consistency across
//! otherwise unrelated crates when handling data

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

/// Use the hash of this object for various purposes, such
/// as generating a unique filename
pub trait HashToString {
	/// get a string of the hash of this object;
	/// useful for things like filenames
	fn hash_to_string(&self) -> String;
	/// get the hash of this object
	fn as_hash(&self) -> u64;
}

impl <T> HashToString for T where T: Hash {
	
	fn as_hash(&self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.hash(&mut hasher);
		hasher.finish()
	}

	fn hash_to_string(&self) -> String {
		self.as_hash()
			.to_string()
	}
}

/// Get a temporary file path for an object
pub trait TempFilePath {
	/// Get a suitable temporary filename for a hashable object;
	/// this filename should be based on the hash and consistent across multiple calls,
	/// so that later callers can use the function to reconstruct an already-created filename.
	/// Note that the output is a string, not a PathBuf, since we can guarantee that the filename
	/// will be valid unicode.
	fn temp_filename(&self, ext: &str) -> String;
	/// Generate a temporary filepath for an object, giving it the extension `ext`. This should
	/// join the result of `temp_filename` to a consistent temporary directory which is guaranteed to be
	/// either the result of `std::env::temp_dir` or (if `folder_name` is not None) a subdirectory called `folder_name`.
	fn temp_file_path<P: AsRef<Path>>(&self, folder_name: Option<P>, ext: &str) -> PathBuf;
}

impl <T> TempFilePath for T where T: HashToString {
	
	fn temp_filename(&self, ext: &str) -> String {
		format!("{}.{}", self.hash_to_string(), ext)
	}

	fn temp_file_path<P: AsRef<Path>>(&self, folder_name: Option<P>, ext: &str) -> PathBuf {
		let filename = self.temp_filename(ext);
		let mut tmp_dir = std::env::temp_dir();
		if let Some(folder_name) = folder_name {
			tmp_dir = tmp_dir.join(folder_name);
		}
		if !tmp_dir.exists() {
			std::fs::create_dir_all(&tmp_dir).unwrap();
		}
		tmp_dir.join(&filename)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Debug, Hash)]
	struct RandomType(String);

	#[test]
	fn test_hash_to_string() {
		let target = RandomType("Hello".into());
		let expected_hash = 12991522711919756218;
		assert_eq!(target.as_hash(), expected_hash);
		assert_eq!(target.hash_to_string(), expected_hash.to_string());
		let expected_filename = format!("{}.txt", expected_hash);
		assert_eq!(target.temp_filename("txt"), expected_filename);
		assert_eq!(
			target.temp_file_path(None::<&str>, "txt"),
			std::env::temp_dir().join(&expected_filename)
		);
		assert_eq!(
			target.temp_file_path(Some("testing_temp_file_name"), "txt"),
			std::env::temp_dir()
				.join("testing_temp_file_name")
				.join(&expected_filename)
		);
	}
}