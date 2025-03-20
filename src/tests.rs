use crate::{self as slang, Blob, Error, FileSystem, Result};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

#[test]
fn compile() {
	let global_session = slang::GlobalSession::new().unwrap();

	let search_path = std::ffi::CString::new("shaders").unwrap();

	// All compiler options are available through this builder.
	let session_options = slang::CompilerOptions::default()
		.optimization(slang::OptimizationLevel::High)
		.matrix_layout_row(true);

	let target_desc = slang::TargetDesc::default()
		.format(slang::CompileTarget::Dxil)
		.profile(global_session.find_profile("sm_6_5"));

	let targets = [target_desc];
	let search_paths = [search_path.as_ptr()];

	let session_desc = slang::SessionDesc::default()
		.targets(&targets)
		.search_paths(&search_paths)
		.options(&session_options);

	let session = global_session.create_session(&session_desc).unwrap();
	let module = session.load_module("test.slang").unwrap();
	assert_eq!(module.entry_point_count(), 1);
	let entry_point = module.find_entry_point_by_name("main").unwrap();

	let program = session
		.create_composite_component_type(&[module.deref().clone(), entry_point.deref().clone()])
		.unwrap();

	let linked_program = program.link().unwrap();

	// Entry point to the reflection API.
	let reflection = linked_program.layout(0).unwrap();
	assert_eq!(reflection.entry_point_count(), 1);
	assert_eq!(reflection.parameter_count(), 3);

	let shader_bytecode = linked_program.entry_point_code(0, 0).unwrap();
	assert_ne!(shader_bytecode.as_slice().len(), 0);
}

#[test]
fn custom_file_system() {
	struct TestFileSystem;

	impl FileSystem for TestFileSystem {
		fn load_file(&self, path: &str) -> Result<Blob> {
			match path {
				"virtual.slang" => {
					let code = b"[shader(\"compute\")]\n[numthreads(1,1,1)]\nvoid main() {}";
					Ok(Blob::from(code.as_slice()))
				}
				_ => Err(Error::Result(-1)),
			}
		}
	}

	let global_session = slang::GlobalSession::new().unwrap();

	let search_path = std::ffi::CString::new("shaders").unwrap();

	// All compiler options are available through this builder.
	let session_options = slang::CompilerOptions::default()
		.optimization(slang::OptimizationLevel::High)
		.matrix_layout_row(true);

	let target_desc = slang::TargetDesc::default()
		.format(slang::CompileTarget::Dxil)
		.profile(global_session.find_profile("sm_6_5"));

	let targets = [target_desc];
	let search_paths = [search_path.as_ptr()];
	let desc = slang::SessionDesc::default()
		.targets(&targets)
		.search_paths(&search_paths)
		.options(&session_options)
		.file_system(TestFileSystem);
	let session = global_session.create_session(&desc).unwrap();

	let module = session.load_module("virtual.slang").unwrap();
	assert_eq!(module.entry_point_count(), 1);
	assert_eq!(module.name(), "virtual.slang");
}

#[test]
fn memory_blob() {
	// Test basic blob creation
	let data = b"Test data";
	let blob = Blob::from(data.as_slice());
	assert_eq!(blob.as_slice(), data);

	// Test reference counting
	let blob2 = blob.clone();
	assert_eq!(blob2.as_slice(), data);
	drop(blob);
	assert_eq!(blob2.as_slice(), data); // Should still be valid
}

#[test]
fn custom_filesystem_with_memory_blobs() {
	// Create a virtual filesystem that stores files in memory
	#[derive(Default)]
	struct VirtualFS {
		files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
	}

	impl FileSystem for VirtualFS {
		fn load_file(&self, path: &str) -> Result<Blob> {
			let files = self.files.lock().unwrap();
			match files.get(path) {
				Some(data) => Ok(Blob::from(data.clone())),
				None => Err(Error::Result(-1)),
			}
		}
	}

	let fs = VirtualFS::default();

	// Add some test files
	{
		let mut files = fs.files.lock().unwrap();
		files.insert(
			"test1.slang".to_string(),
			b"[shader(\"compute\")] void main() {}".to_vec(),
		);
		files.insert(
			"test2.slang".to_string(),
			b"struct Test { float value; };".to_vec(),
		);
	}

	let global_session = slang::GlobalSession::new().unwrap();

	// Test loading and using files through our custom filesystem
	let desc = slang::SessionDesc::default().file_system(fs);
	let session = global_session.create_session(&desc).unwrap();

	// Try loading both files
	let module1 = session.load_module("test1.slang").unwrap();
	assert_eq!(module1.entry_point_count(), 1);
	assert_eq!(module1.name(), "test1.slang");

	let module2 = session.load_module("test2.slang").unwrap();
	assert_eq!(module2.name(), "test2.slang");

	// Test error case with non-existent file
	let err_module = session.load_module("nonexistent.slang");
	assert!(err_module.is_err());
}
